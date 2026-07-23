use domain::{Changeset, ChangesetState, Finding, RelativePath, Repository, Worktree, limits};
use execution::{CancellationToken, ProcessSpec, TerminalOutcome};
use git::{GitService, GitStatusSnapshot, content_digest};
use persistence::SqliteStore;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use thiserror::Error;

const MAX_REVIEW_CHECKS: usize = 5;
const MAX_REVIEW_TIMEOUT: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReviewCheckKind {
    Format,
    Typecheck,
    Test,
    SecretScan,
    DependencyAudit,
}

impl ReviewCheckKind {
    fn category(self) -> &'static str {
        match self {
            Self::Format => "review_format",
            Self::Typecheck => "review_typecheck",
            Self::Test => "review_test",
            Self::SecretScan => "review_secret_scan",
            Self::DependencyAudit => "review_dependency_audit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewCheckStatus {
    Passed,
    Failed,
    Unavailable,
    TimedOut,
    Mutated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewCheckResult {
    pub kind: ReviewCheckKind,
    pub status: ReviewCheckStatus,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewReport {
    pub changeset_id: domain::Id,
    pub head_sha: String,
    pub changed_paths: Vec<RelativePath>,
    pub unified_diff: String,
    pub checks: Vec<ReviewCheckResult>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone)]
pub struct ReviewOptions {
    pub timeout: Duration,
    pub output_limit: usize,
    pub allow_unsandboxed_checks: bool,
}

impl Default for ReviewOptions {
    fn default() -> Self {
        Self {
            timeout: MAX_REVIEW_TIMEOUT,
            output_limit: limits::MAX_SEMANTIC_TEXT_BYTES,
            allow_unsandboxed_checks: false,
        }
    }
}

#[derive(Debug, Clone)]
struct ReviewPrograms {
    cargo: Option<PathBuf>,
    cargo_deny: Option<PathBuf>,
    gitleaks: Option<PathBuf>,
    search_path: String,
}

pub struct ReviewService {
    store: SqliteStore,
    git: GitService,
    programs: ReviewPrograms,
    options: ReviewOptions,
}

impl ReviewService {
    pub fn discover(
        store: SqliteStore,
        git: GitService,
        options: ReviewOptions,
    ) -> Result<Self, ReviewError> {
        let search_path = std::env::var("PATH").map_err(|_| ReviewError::MissingSearchPath)?;
        Self::with_search_path(store, git, options, &search_path)
    }

    pub fn with_search_path(
        store: SqliteStore,
        git: GitService,
        options: ReviewOptions,
        search_path: &str,
    ) -> Result<Self, ReviewError> {
        validate_options(&options)?;
        let search_path = normalize_search_path(search_path)?;
        Ok(Self {
            store,
            git,
            programs: ReviewPrograms {
                cargo: resolve_named_binary(&search_path, "cargo")?,
                cargo_deny: resolve_named_binary(&search_path, "cargo-deny")?,
                gitleaks: resolve_named_binary(&search_path, "gitleaks")?,
                search_path,
            },
            options,
        })
    }

    pub fn review(
        &self,
        repository: &Repository,
        changeset: &Changeset,
        worktree: &Worktree,
        requested_checks: &[ReviewCheckKind],
    ) -> Result<ReviewReport, ReviewError> {
        validate_review_request(repository, changeset, worktree, requested_checks)?;
        let initial = self.git.status_snapshot(repository, worktree)?;
        if initial.head_sha != changeset.head_sha() {
            return Err(ReviewError::HeadChanged);
        }
        let changed_paths = initial.changed_paths();
        if changed_paths.is_empty() {
            return Err(ReviewError::NoChangedPaths);
        }
        validate_changed_files(worktree, &changed_paths)?;
        let unified_diff = self.git.diff(worktree, &changed_paths)?;
        if !self.snapshot_matches(
            repository,
            worktree,
            &initial,
            &changed_paths,
            &unified_diff,
        )? {
            return Err(ReviewError::WorktreeChanged);
        }
        let blob_identity = content_digest(unified_diff.as_bytes());

        let mut checks = Vec::with_capacity(requested_checks.len());
        let mut findings = Vec::new();
        let mut mutation_detected = false;
        for kind in requested_checks {
            let result = self.run_check(
                *kind,
                repository,
                worktree,
                &initial,
                &changed_paths,
                &unified_diff,
            )?;
            if result.status != ReviewCheckStatus::Passed {
                findings.push(finding_for_result(changeset.id, &blob_identity, &result));
            }
            mutation_detected = result.status == ReviewCheckStatus::Mutated;
            checks.push(result);
            if mutation_detected {
                break;
            }
        }
        if !mutation_detected
            && !self.snapshot_matches(
                repository,
                worktree,
                &initial,
                &changed_paths,
                &unified_diff,
            )?
        {
            return Err(ReviewError::WorktreeChanged);
        }
        let findings = self.store.save_findings(&findings)?;

        Ok(ReviewReport {
            changeset_id: changeset.id,
            head_sha: initial.head_sha,
            changed_paths,
            unified_diff,
            checks,
            findings,
        })
    }

    fn run_check(
        &self,
        kind: ReviewCheckKind,
        repository: &Repository,
        worktree: &Worktree,
        initial: &GitStatusSnapshot,
        changed_paths: &[RelativePath],
        initial_diff: &str,
    ) -> Result<ReviewCheckResult, ReviewError> {
        let spec = match self.check_spec(kind, &worktree.path) {
            Ok(spec) => spec,
            Err(evidence) => {
                return Ok(ReviewCheckResult {
                    kind,
                    status: ReviewCheckStatus::Unavailable,
                    evidence,
                });
            }
        };
        let process = execution::supervise(&spec, &CancellationToken::default())?;
        let truncated = process.stdout_truncated || process.stderr_truncated;
        let mut status = if truncated {
            ReviewCheckStatus::Failed
        } else {
            match process.outcome {
                TerminalOutcome::Completed(0) => ReviewCheckStatus::Passed,
                TerminalOutcome::TimedOut => ReviewCheckStatus::TimedOut,
                TerminalOutcome::Completed(_)
                | TerminalOutcome::Cancelled
                | TerminalOutcome::Failed => ReviewCheckStatus::Failed,
            }
        };
        let diagnostic = process_diagnostic(process.outcome, truncated);
        let mut evidence = bounded_evidence(
            diagnostic.as_deref(),
            &process.stdout,
            &process.stderr,
            self.options.output_limit,
        );
        if !self.snapshot_matches(repository, worktree, initial, changed_paths, initial_diff)? {
            status = ReviewCheckStatus::Mutated;
            evidence = prepend_bounded(
                "check changed the worktree or HEAD",
                &evidence,
                self.options.output_limit,
            );
        }
        Ok(ReviewCheckResult {
            kind,
            status,
            evidence,
        })
    }

    fn snapshot_matches(
        &self,
        repository: &Repository,
        worktree: &Worktree,
        initial: &GitStatusSnapshot,
        changed_paths: &[RelativePath],
        initial_diff: &str,
    ) -> Result<bool, ReviewError> {
        if self.git.status_snapshot(repository, worktree)? != *initial {
            return Ok(false);
        }
        Ok(self.git.diff(worktree, changed_paths)? == initial_diff)
    }

    fn check_spec(&self, kind: ReviewCheckKind, worktree: &Path) -> Result<ProcessSpec, String> {
        if !self.options.allow_unsandboxed_checks {
            return Err(
                "check requires explicit trust because local sandboxing is unavailable".to_owned(),
            );
        }
        let (program, arguments) = match kind {
            ReviewCheckKind::Format => (
                require_program(self.programs.cargo.as_ref(), "cargo")?,
                vec!["fmt", "--all", "--", "--check"],
            ),
            ReviewCheckKind::Typecheck => (
                require_program(self.programs.cargo.as_ref(), "cargo")?,
                vec!["check", "--workspace", "--all-targets", "--all-features"],
            ),
            ReviewCheckKind::Test => (
                require_program(self.programs.cargo.as_ref(), "cargo")?,
                vec!["test", "--workspace", "--all-features"],
            ),
            ReviewCheckKind::SecretScan => (
                require_program(self.programs.gitleaks.as_ref(), "gitleaks")?,
                vec!["detect", "--no-banner", "--redact", "."],
            ),
            ReviewCheckKind::DependencyAudit => (
                require_program(self.programs.cargo_deny.as_ref(), "cargo-deny")?,
                vec!["check"],
            ),
        };
        Ok(ProcessSpec {
            program: program.to_string_lossy().into_owned(),
            arguments: arguments.into_iter().map(str::to_owned).collect(),
            environment: minimal_environment(&self.programs.search_path),
            sensitive_environment_keys: Vec::new(),
            current_dir: Some(worktree.to_path_buf()),
            timeout: self.options.timeout,
            output_limit: self.options.output_limit,
        })
    }
}

fn validate_options(options: &ReviewOptions) -> Result<(), ReviewError> {
    if options.timeout.is_zero()
        || options.timeout > MAX_REVIEW_TIMEOUT
        || options.output_limit == 0
        || options.output_limit > limits::MAX_SEMANTIC_TEXT_BYTES
    {
        return Err(ReviewError::InvalidOptions);
    }
    Ok(())
}

fn validate_review_request(
    repository: &Repository,
    changeset: &Changeset,
    worktree: &Worktree,
    requested_checks: &[ReviewCheckKind],
) -> Result<(), ReviewError> {
    if changeset.state() != ChangesetState::Reviewable
        || changeset.repository_id() != repository.id
        || worktree.changeset_id() != changeset.id
    {
        return Err(ReviewError::InvalidReviewState);
    }
    if requested_checks.len() > MAX_REVIEW_CHECKS {
        return Err(ReviewError::TooManyChecks);
    }
    let unique: HashSet<_> = requested_checks.iter().copied().collect();
    if unique.len() != requested_checks.len() {
        return Err(ReviewError::DuplicateCheck);
    }
    Ok(())
}

fn validate_changed_files(worktree: &Worktree, paths: &[RelativePath]) -> Result<(), ReviewError> {
    for path in paths {
        match fs::symlink_metadata(worktree.path.join(path.as_path())) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(ReviewError::UnsupportedChangedPath(path.clone()));
            }
            Ok(metadata) if metadata.len() > limits::MAX_APPROVED_FILE_BYTES as u64 => {
                return Err(ReviewError::ChangedFileTooLarge(path.clone()));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn normalize_search_path(search_path: &str) -> Result<String, ReviewError> {
    let directories = std::env::split_paths(search_path).collect::<Vec<_>>();
    if directories.is_empty() || directories.iter().any(|directory| !directory.is_absolute()) {
        return Err(ReviewError::UnsafeSearchPath);
    }
    std::env::join_paths(directories)
        .map_err(|_| ReviewError::UnsafeSearchPath)?
        .into_string()
        .map_err(|_| ReviewError::UnsafeSearchPath)
}

fn resolve_named_binary(search_path: &str, name: &str) -> Result<Option<PathBuf>, ReviewError> {
    for directory in std::env::split_paths(search_path) {
        let candidate = directory.join(name);
        if candidate.is_file() {
            return Ok(Some(fs::canonicalize(candidate)?));
        }
    }
    Ok(None)
}

fn minimal_environment(search_path: &str) -> HashMap<String, String> {
    HashMap::from([
        ("PATH".to_owned(), search_path.to_owned()),
        ("HOME".to_owned(), "/dev/null".to_owned()),
        ("LC_ALL".to_owned(), "C".to_owned()),
        ("GIT_TERMINAL_PROMPT".to_owned(), "0".to_owned()),
        ("GIT_CONFIG_NOSYSTEM".to_owned(), "1".to_owned()),
        ("GIT_CONFIG_GLOBAL".to_owned(), "/dev/null".to_owned()),
        ("GIT_ATTR_NOSYSTEM".to_owned(), "1".to_owned()),
        ("GIT_OPTIONAL_LOCKS".to_owned(), "0".to_owned()),
    ])
}

fn require_program<'a>(program: Option<&'a PathBuf>, name: &str) -> Result<&'a PathBuf, String> {
    program.ok_or_else(|| format!("{name} is not installed"))
}

fn process_diagnostic(outcome: TerminalOutcome, truncated: bool) -> Option<String> {
    if truncated {
        return Some("process output exceeded the review limit".to_owned());
    }
    match outcome {
        TerminalOutcome::Completed(0) => None,
        TerminalOutcome::Completed(code) => Some(format!("process exited with status {code}")),
        TerminalOutcome::Cancelled => Some("process was cancelled".to_owned()),
        TerminalOutcome::TimedOut => Some("process exceeded the review timeout".to_owned()),
        TerminalOutcome::Failed => Some("process supervision failed".to_owned()),
    }
}

fn bounded_evidence(
    diagnostic: Option<&str>,
    stdout: &[u8],
    stderr: &[u8],
    limit: usize,
) -> String {
    let mut evidence = Vec::with_capacity(
        limit.min(
            diagnostic
                .map_or(0, str::len)
                .saturating_add(stdout.len())
                .saturating_add(stderr.len())
                .saturating_add(2),
        ),
    );
    if let Some(diagnostic) = diagnostic {
        append_bytes(&mut evidence, diagnostic.as_bytes(), limit);
    }
    if !evidence.is_empty() && (!stdout.is_empty() || !stderr.is_empty()) {
        append_bytes(&mut evidence, b"\n", limit);
    }
    append_bytes(&mut evidence, stdout, limit);
    if !stdout.is_empty() && !stderr.is_empty() {
        append_bytes(&mut evidence, b"\n", limit);
    }
    append_bytes(&mut evidence, stderr, limit);
    String::from_utf8_lossy(&evidence).into_owned()
}

fn prepend_bounded(additional: &str, existing: &str, limit: usize) -> String {
    let mut evidence =
        Vec::with_capacity(limit.min(existing.len().saturating_add(additional.len() + 1)));
    append_bytes(&mut evidence, additional.as_bytes(), limit);
    if !existing.is_empty() {
        append_bytes(&mut evidence, b"\n", limit);
    }
    append_bytes(&mut evidence, existing.as_bytes(), limit);
    String::from_utf8_lossy(&evidence).into_owned()
}

fn append_bytes(target: &mut Vec<u8>, source: &[u8], limit: usize) {
    let remaining = limit.saturating_sub(target.len());
    target.extend_from_slice(&source[..source.len().min(remaining)]);
}

fn finding_for_result(
    changeset_id: domain::Id,
    blob_identity: &str,
    result: &ReviewCheckResult,
) -> Finding {
    let severity = match result.status {
        ReviewCheckStatus::Mutated | ReviewCheckStatus::TimedOut => "error",
        ReviewCheckStatus::Failed | ReviewCheckStatus::Unavailable => "warning",
        ReviewCheckStatus::Passed => "info",
    };
    Finding::new(
        changeset_id,
        None,
        blob_identity.to_owned(),
        result.kind.category().to_owned(),
        severity.to_owned(),
        format!("{:?} check {:?}", result.kind, result.status),
        result.evidence.clone(),
    )
}

#[derive(Debug, Error)]
pub enum ReviewError {
    #[error("review options exceed local safety limits")]
    InvalidOptions,
    #[error("PATH is unavailable for fixed review tool discovery")]
    MissingSearchPath,
    #[error("review PATH must contain only absolute directories")]
    UnsafeSearchPath,
    #[error("changeset and worktree are not in a reviewable state")]
    InvalidReviewState,
    #[error("changeset HEAD changed before review")]
    HeadChanged,
    #[error("worktree changed while the review snapshot was being prepared")]
    WorktreeChanged,
    #[error("review requires at least one changed path")]
    NoChangedPaths,
    #[error("too many review checks were requested")]
    TooManyChecks,
    #[error("a review check was requested more than once")]
    DuplicateCheck,
    #[error("changed path is not a regular file: {path}", path = .0.as_str())]
    UnsupportedChangedPath(RelativePath),
    #[error("changed file exceeds the review size limit: {path}", path = .0.as_str())]
    ChangedFileTooLarge(RelativePath),
    #[error("filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Git review operation failed: {0}")]
    Git(#[from] git::GitError),
    #[error("review process execution failed: {0}")]
    Execution(#[from] execution::ExecutionError),
    #[error("finding persistence failed: {0}")]
    Persistence(#[from] persistence::PersistenceError),
}
