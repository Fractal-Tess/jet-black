use domain::{ActionKind, ApprovedAction, Id, RelativePath, Repository, Worktree, limits};
use sha2::{Digest, Sha256};
use std::{
    ffi::{CString, OsStr, OsString},
    fs,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Component, Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    thread,
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationState {
    pub repository_head_sha: String,
    pub worktree_head_sha: String,
    pub worktree_dirty: bool,
}

#[derive(Debug, Clone)]
pub struct GitService {
    repository_roots: Vec<PathBuf>,
    worktree_root: PathBuf,
    git_binary: PathBuf,
}

impl GitService {
    pub fn new(repository_roots: Vec<PathBuf>, worktree_root: PathBuf) -> Result<Self, GitError> {
        fs::create_dir_all(&worktree_root)?;
        let worktree_root = fs::canonicalize(worktree_root)?;
        let repository_roots = repository_roots
            .into_iter()
            .map(fs::canonicalize)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            repository_roots,
            worktree_root,
            git_binary: resolve_git_binary()?,
        })
    }

    pub fn register(&self, selected: &Path) -> Result<Repository, GitError> {
        let canonical = fs::canonicalize(selected)?;
        self.ensure_repository_root(&canonical)?;
        let unsafe_config = self.git_output(&canonical, ["config", "--local", "--get-regexp", r"^(core\.fsmonitor|filter\..*\.(clean|smudge|process)|diff\..*\.textconv|fsmonitor\.)"])?;
        if unsafe_config.status.success() && !unsafe_config.stdout.is_empty() {
            return Err(GitError::UnsupportedRepository("executable Git filters"));
        }
        if self.git_text(&canonical, ["rev-parse", "--is-bare-repository"])? != "false" {
            return Err(GitError::BareRepository);
        }
        if self.git_text(&canonical, ["rev-parse", "--is-inside-work-tree"])? != "true" {
            return Err(GitError::NotRepository);
        }
        if !self
            .git_text(&canonical, ["status", "--porcelain"])?
            .is_empty()
        {
            return Err(GitError::DirtyRepository);
        }
        if !self
            .git_text(&canonical, ["submodule", "status"])?
            .is_empty()
        {
            return Err(GitError::UnsupportedRepository("submodules"));
        }
        if self
            .git_text(&canonical, ["ls-files", "-s"])?
            .lines()
            .any(|line| line.starts_with("160000 "))
        {
            return Err(GitError::UnsupportedRepository("linked submodules"));
        }
        if fs::read(canonical.join(".gitattributes")).is_ok_and(|bytes| {
            String::from_utf8_lossy(&bytes)
                .lines()
                .any(|line| line.contains("filter=lfs"))
        }) {
            return Err(GitError::UnsupportedRepository("Git LFS"));
        }
        if self
            .git_text(&canonical, ["worktree", "list", "--porcelain"])?
            .lines()
            .filter(|line| line.starts_with("worktree "))
            .count()
            > 1
        {
            return Err(GitError::UnsupportedRepository("linked worktrees"));
        }
        let base_sha = self.git_text(&canonical, ["rev-parse", "HEAD"])?;
        if base_sha.len() != 40 && base_sha.len() != 64 {
            return Err(GitError::MissingCommit);
        }
        let default_branch = self.git_text(&canonical, ["branch", "--show-current"])?;
        let primary_remote = self
            .git_optional_text(&canonical, ["remote", "get-url", "origin"])
            .map(|remote| sanitize_remote(&remote));
        let identity = primary_remote
            .clone()
            .unwrap_or_else(|| canonical.display().to_string());
        Ok(Repository {
            id: Id::new_v4(),
            filesystem_identity: filesystem_identity(&canonical)?,
            canonical_path: canonical,
            identity,
            primary_remote,
            default_branch,
            base_sha,
            version: 0,
        })
    }

    pub fn create_worktree(
        &self,
        repository: &Repository,
        changeset_id: Id,
    ) -> Result<Worktree, GitError> {
        let repository_path = self.validate_registered_repository(repository)?;
        let path = self.worktree_root.join(changeset_id.to_string());
        ensure_lexical_child(&self.worktree_root, &path)?;
        if path.exists() {
            return Err(GitError::WorktreeExists);
        }
        let output = self.git_output(
            &repository_path,
            [
                OsStr::new("worktree"),
                OsStr::new("add"),
                OsStr::new("--detach"),
                path.as_os_str(),
                OsStr::new(&repository.base_sha),
            ],
        )?;
        ensure_success(output, "create worktree")?;
        let canonical = fs::canonicalize(&path)?;
        ensure_canonical_child(&self.worktree_root, &canonical)?;
        let mut worktree = Worktree::creating(
            Id::new_v4(),
            changeset_id,
            canonical.clone(),
            filesystem_identity(&canonical)?,
            repository.base_sha.clone(),
        );
        worktree.ready().map_err(GitError::Domain)?;
        Ok(worktree)
    }

    pub fn write_approved_file(
        &self,
        repository: &Repository,
        worktree: &Worktree,
        approved: ApprovedAction,
        content: &[u8],
    ) -> Result<(), GitError> {
        if content.len() > limits::MAX_APPROVED_FILE_BYTES {
            return Err(GitError::ResourceLimit("approved file content"));
        }
        self.ensure_ready(worktree)?;
        self.validate_registered_repository(repository)?;
        let current_head = self.head_sha(worktree)?;
        if approved.repository_id() != repository.id
            || approved.changeset_id() != worktree.changeset_id()
            || approved.action() != ActionKind::WriteFile
            || approved.base_sha() != repository.base_sha
            || approved.head_sha() != current_head
            || approved.content_sha256() != content_digest(content)
        {
            return Err(GitError::ApprovalMismatch);
        }
        ensure_canonical_within(&self.worktree_root, &worktree.path)?;
        write_file_beneath(&worktree.path, approved.target_path().as_path(), content)
    }

    pub fn diff(&self, worktree: &Worktree, paths: &[RelativePath]) -> Result<String, GitError> {
        if paths.is_empty() || paths.len() > limits::MAX_CHANGED_FILES {
            return Err(GitError::ResourceLimit("changed file count"));
        }
        self.ensure_ready(worktree)?;
        ensure_canonical_within(&self.worktree_root, &worktree.path)?;

        let mut add_arguments = vec![
            OsString::from("add"),
            OsString::from("-N"),
            OsString::from("--"),
        ];
        add_arguments.extend(
            paths
                .iter()
                .map(|path| path.as_path().as_os_str().to_owned()),
        );
        let intent_to_add = self.git_output(&worktree.path, add_arguments)?;
        ensure_success(intent_to_add, "stage worktree paths for diff")?;

        let mut diff_arguments = vec![
            OsString::from("diff"),
            OsString::from("HEAD"),
            OsString::from("--no-ext-diff"),
            OsString::from("--no-textconv"),
            OsString::from("--binary"),
            OsString::from("--"),
        ];
        diff_arguments.extend(
            paths
                .iter()
                .map(|path| path.as_path().as_os_str().to_owned()),
        );
        let output = self.git_output_with_limit(
            &worktree.path,
            diff_arguments,
            limits::MAX_UNIFIED_DIFF_BYTES,
        )?;
        if output.stdout_truncated {
            return Err(GitError::ResourceLimit("unified diff"));
        }
        let output = ensure_success(output, "calculate diff")?;
        String::from_utf8(output.stdout).map_err(|_| GitError::NonUtf8Output)
    }

    pub fn head_sha(&self, worktree: &Worktree) -> Result<String, GitError> {
        self.ensure_ready(worktree)?;
        ensure_canonical_within(&self.worktree_root, &worktree.path)?;
        self.git_text(&worktree.path, ["rev-parse", "HEAD"])
    }

    pub fn reconciliation_state(
        &self,
        repository: &Repository,
        worktree: &Worktree,
    ) -> Result<ReconciliationState, GitError> {
        let repository_path = self.validate_registered_repository_for_cleanup(repository)?;
        self.ensure_ready(worktree)?;
        ensure_canonical_within(&self.worktree_root, &worktree.path)?;

        let worktree_root =
            fs::canonicalize(self.git_text(&worktree.path, ["rev-parse", "--show-toplevel"])?)?;
        if worktree_root != worktree.path {
            return Err(GitError::RepositoryIdentityChanged);
        }

        let repository_git_dir = fs::canonicalize(repository_path.join(".git"))?;
        let common_git_dir =
            PathBuf::from(self.git_text(&worktree.path, ["rev-parse", "--git-common-dir"])?);
        let common_git_dir = if common_git_dir.is_absolute() {
            common_git_dir
        } else {
            worktree.path.join(common_git_dir)
        };
        if fs::canonicalize(common_git_dir)? != repository_git_dir {
            return Err(GitError::RepositoryIdentityChanged);
        }

        Ok(ReconciliationState {
            repository_head_sha: self.git_text(&repository_path, ["rev-parse", "HEAD"])?,
            worktree_head_sha: self.git_text(&worktree.path, ["rev-parse", "HEAD"])?,
            worktree_dirty: !self
                .git_text(&worktree.path, ["status", "--porcelain"])?
                .is_empty(),
        })
    }

    pub fn cleanup_worktree(
        &self,
        repository: &Repository,
        worktree: &mut Worktree,
    ) -> Result<(), GitError> {
        if worktree.state() == domain::WorktreeState::Removed {
            return Ok(());
        }
        if worktree.state() != domain::WorktreeState::Ready {
            return Err(GitError::WorktreeNotReady);
        }
        let repository_path = self.validate_registered_repository_for_cleanup(repository)?;
        worktree.begin_removal().map_err(GitError::Domain)?;
        let result = (|| {
            let lexical = worktree.path.clone();
            ensure_lexical_child(&self.worktree_root, &lexical)?;
            if lexical.exists() {
                let canonical = fs::canonicalize(&lexical)?;
                ensure_canonical_child(&self.worktree_root, &canonical)?;
                let output = self.git_output(
                    &repository_path,
                    [
                        OsStr::new("worktree"),
                        OsStr::new("remove"),
                        OsStr::new("--force"),
                        canonical.as_os_str(),
                    ],
                )?;
                ensure_success(output, "remove worktree")?;
            }
            let prune = self.git_output(&repository_path, ["worktree", "prune"])?;
            ensure_success(prune, "prune worktrees")?;
            Ok::<(), GitError>(())
        })();
        match result {
            Ok(()) => {
                worktree.removed().map_err(GitError::Domain)?;
                Ok(())
            }
            Err(error) => {
                worktree.removal_failed().map_err(GitError::Domain)?;
                Err(error)
            }
        }
    }

    fn ensure_repository_root(&self, path: &Path) -> Result<(), GitError> {
        if self
            .repository_roots
            .iter()
            .any(|root| path.starts_with(root))
        {
            Ok(())
        } else {
            Err(GitError::RepositoryOutsideRoots)
        }
    }
    fn ensure_ready(&self, worktree: &Worktree) -> Result<(), GitError> {
        if worktree.state() != domain::WorktreeState::Ready {
            return Err(GitError::WorktreeNotReady);
        }
        if filesystem_identity(&worktree.path)? != worktree.filesystem_identity() {
            return Err(GitError::RepositoryIdentityChanged);
        }
        Ok(())
    }
    fn validate_registered_repository(&self, repository: &Repository) -> Result<PathBuf, GitError> {
        let current = fs::canonicalize(&repository.canonical_path)?;
        self.ensure_repository_root(&current)?;
        if current != repository.canonical_path {
            return Err(GitError::RepositoryIdentityChanged);
        }
        if filesystem_identity(&current)? != repository.filesystem_identity {
            return Err(GitError::RepositoryIdentityChanged);
        }
        if self.git_text(&current, ["rev-parse", "HEAD"])? != repository.base_sha {
            return Err(GitError::RepositoryIdentityChanged);
        }
        Ok(current)
    }
    fn validate_registered_repository_for_cleanup(
        &self,
        repository: &Repository,
    ) -> Result<PathBuf, GitError> {
        let current = fs::canonicalize(&repository.canonical_path)?;
        self.ensure_repository_root(&current)?;
        if current != repository.canonical_path
            || filesystem_identity(&current)? != repository.filesystem_identity
        {
            return Err(GitError::RepositoryIdentityChanged);
        }
        Ok(current)
    }
    fn git_output<I, S>(&self, repository: &Path, arguments: I) -> Result<CommandOutput, GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.git_output_with_limit(repository, arguments, limits::MAX_CAPTURED_STREAM_BYTES)
    }
    fn git_output_with_limit<I, S>(
        &self,
        repository: &Path,
        arguments: I,
        output_limit: usize,
    ) -> Result<CommandOutput, GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = git_command(&self.git_binary, repository);
        command.args(arguments);
        run_bounded_command(&mut command, output_limit)
    }
    fn git_text<I, S>(&self, repository: &Path, arguments: I) -> Result<String, GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = ensure_success(self.git_output(repository, arguments)?, "run git")?;
        Ok(String::from_utf8(output.stdout)
            .map_err(|_| GitError::NonUtf8Output)?
            .trim()
            .to_owned())
    }
    fn git_optional_text<I, S>(&self, repository: &Path, arguments: I) -> Option<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.git_text(repository, arguments)
            .ok()
            .filter(|value| !value.is_empty())
    }
}

pub fn content_digest(content: &[u8]) -> String {
    format!("{:x}", Sha256::digest(content))
}

#[derive(Debug)]
struct CommandOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdout_truncated: bool,
    stderr_truncated: bool,
}

fn run_bounded_command(
    command: &mut Command,
    output_limit: usize,
) -> Result<CommandOutput, GitError> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().ok_or(GitError::MissingPipe)?;
    let stderr = child.stderr.take().ok_or(GitError::MissingPipe)?;
    let stdout_reader = thread::spawn(move || read_bounded(stdout, output_limit));
    let stderr_reader = thread::spawn(move || read_bounded(stderr, output_limit));
    let status = child.wait()?;
    let (stdout, stdout_truncated) = stdout_reader
        .join()
        .map_err(|_| GitError::ReaderPanicked)??;
    let (stderr, stderr_truncated) = stderr_reader
        .join()
        .map_err(|_| GitError::ReaderPanicked)??;
    Ok(CommandOutput {
        status,
        stdout,
        stderr,
        stdout_truncated,
        stderr_truncated,
    })
}

fn read_bounded(mut reader: impl Read, limit: usize) -> Result<(Vec<u8>, bool), std::io::Error> {
    let mut captured = Vec::with_capacity(limit.min(8192));
    let mut buffer = [0_u8; 8192];
    let mut truncated = false;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(captured.len());
        captured.extend_from_slice(&buffer[..read.min(remaining)]);
        truncated |= read > remaining;
    }
    Ok((captured, truncated))
}

fn sanitize_remote(remote: &str) -> String {
    if let Some(scheme_end) = remote.find("://") {
        let scheme = &remote[..scheme_end + 3];
        let remainder = &remote[scheme_end + 3..];
        let authority_end = remainder.find(['/', '?', '#']).unwrap_or(remainder.len());
        let authority = &remainder[..authority_end];
        let host = authority
            .rsplit_once('@')
            .map_or(authority, |(_, host)| host);
        let path = &remainder[authority_end..];
        let path_end = path.find(['?', '#']).unwrap_or(path.len());
        return format!("{scheme}{host}{}", &path[..path_end]);
    }
    if let Some((user_host, path)) = remote.split_once(':') {
        if let Some((_, host)) = user_host.rsplit_once('@') {
            return format!("{host}:{path}");
        }
    }
    remote.split(['?', '#']).next().unwrap_or(remote).to_owned()
}
fn filesystem_identity(path: &Path) -> Result<String, GitError> {
    let metadata = fs::metadata(path)?;
    Ok(format!("{}:{}", metadata.dev(), metadata.ino()))
}

fn git_command(binary: &Path, repository: &Path) -> Command {
    let mut command = Command::new(binary);
    command
        .current_dir(repository)
        .env_clear()
        .args(["-c", "core.hooksPath=/dev/null"])
        .env("HOME", "/dev/null")
        .env("LC_ALL", "C")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("GIT_OPTIONAL_LOCKS", "0");
    command
}
fn resolve_git_binary() -> Result<PathBuf, GitError> {
    let path = std::env::var_os("PATH").ok_or(GitError::GitBinaryNotFound)?;
    for directory in std::env::split_paths(&path) {
        let candidate = directory.join("git");
        if candidate.is_file() {
            return Ok(fs::canonicalize(candidate)?);
        }
    }
    Err(GitError::GitBinaryNotFound)
}
fn ensure_success(
    output: CommandOutput,
    operation: &'static str,
) -> Result<CommandOutput, GitError> {
    if output.stdout_truncated || output.stderr_truncated {
        return Err(GitError::ResourceLimit("Git process output"));
    }
    if output.status.success() {
        Ok(output)
    } else {
        Err(GitError::Command {
            operation,
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        })
    }
}
fn ensure_lexical_child(root: &Path, candidate: &Path) -> Result<(), GitError> {
    if candidate.starts_with(root)
        && candidate != root
        && !candidate
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        Ok(())
    } else {
        Err(GitError::PathEscape)
    }
}
fn ensure_canonical_child(root: &Path, candidate: &Path) -> Result<(), GitError> {
    ensure_canonical_within(root, candidate)?;
    if fs::canonicalize(candidate)? == fs::canonicalize(root)? {
        return Err(GitError::SymlinkEscape);
    }
    Ok(())
}
fn ensure_canonical_within(root: &Path, candidate: &Path) -> Result<(), GitError> {
    let canonical_root = fs::canonicalize(root)?;
    let canonical_candidate = fs::canonicalize(candidate)?;
    if canonical_candidate.starts_with(&canonical_root) {
        Ok(())
    } else {
        Err(GitError::SymlinkEscape)
    }
}
fn write_file_beneath(root: &Path, relative: &Path, content: &[u8]) -> Result<(), GitError> {
    let parts = relative
        .components()
        .map(|component| match component {
            Component::Normal(value) => {
                CString::new(value.as_bytes()).map_err(|_| GitError::PathEscape)
            }
            _ => Err(GitError::PathEscape),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (file_name, directories) = parts.split_last().ok_or(GitError::PathEscape)?;
    let root_name = CString::new(root.as_os_str().as_bytes()).map_err(|_| GitError::PathEscape)?;
    let root_fd = unsafe {
        libc::open(
            root_name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if root_fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let mut directory = unsafe { OwnedFd::from_raw_fd(root_fd) };
    for part in directories {
        let mut next = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                part.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if next < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
            if unsafe { libc::mkdirat(directory.as_raw_fd(), part.as_ptr(), 0o700) } < 0
                && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
            {
                return Err(std::io::Error::last_os_error().into());
            }
            next = unsafe {
                libc::openat(
                    directory.as_raw_fd(),
                    part.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                )
            };
        }
        if next < 0 {
            return Err(GitError::SymlinkEscape);
        }
        directory = unsafe { OwnedFd::from_raw_fd(next) };
    }
    let temporary = CString::new(format!(".jet-black-{}.tmp", uuid::Uuid::new_v4()))
        .map_err(|_| GitError::PathEscape)?;
    let file_fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            temporary.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o600,
        )
    };
    if file_fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let mut file = std::fs::File::from(unsafe { OwnedFd::from_raw_fd(file_fd) });
    file.write_all(content)?;
    file.sync_all()?;
    if unsafe {
        libc::renameat(
            directory.as_raw_fd(),
            temporary.as_ptr(),
            directory.as_raw_fd(),
            file_name.as_ptr(),
        )
    } < 0
    {
        let error = std::io::Error::last_os_error();
        unsafe {
            libc::unlinkat(directory.as_raw_fd(), temporary.as_ptr(), 0);
        }
        return Err(error.into());
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum GitError {
    #[error("filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("selected path is not a Git worktree")]
    NotRepository,
    #[error("bare Git repositories are unsupported")]
    BareRepository,
    #[error("repository has no commit to use as a base")]
    MissingCommit,
    #[error("repository is outside configured roots")]
    RepositoryOutsideRoots,
    #[error("registered repository identity changed on disk")]
    RepositoryIdentityChanged,
    #[error("path escapes its configured boundary")]
    PathEscape,
    #[error("symlink escapes its configured boundary")]
    SymlinkEscape,
    #[error("worktree already exists")]
    WorktreeExists,
    #[error("approved action does not match the worktree mutation")]
    ApprovalMismatch,
    #[error("worktree is not ready for this operation")]
    WorktreeNotReady,
    #[error("repository contains uncommitted changes")]
    DirtyRepository,
    #[error("repository uses unsupported {0}")]
    UnsupportedRepository(&'static str),
    #[error("Git failed to {operation}: {stderr}")]
    Command {
        operation: &'static str,
        stderr: String,
    },
    #[error("Git returned non-UTF-8 output")]
    NonUtf8Output,
    #[error("Git executable was not found on PATH")]
    GitBinaryNotFound,
    #[error("Git process output pipe was unavailable")]
    MissingPipe,
    #[error("Git process output reader panicked")]
    ReaderPanicked,
    #[error("resource limit exceeded for {0}")]
    ResourceLimit(&'static str),
    #[error("worktree lifecycle failed: {0}")]
    Domain(#[from] domain::DomainError),
}
