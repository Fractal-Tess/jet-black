use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fmt,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use thiserror::Error;
use uuid::Uuid;

const SUPERVISION_TOKEN_ENV: &str = "JET_BLACK_SUPERVISION_TOKEN";

#[derive(Debug, Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    fn cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalOutcome {
    Completed(i32),
    Cancelled,
    TimedOut,
    Failed,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProcessResult {
    pub outcome: TerminalOutcome,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessConfinement {
    Unconfined,
    LinuxFilesystem(LinuxFilesystemConfinement),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxFilesystemConfinement {
    pub workspace: PathBuf,
    pub writable_state: PathBuf,
    pub runtime_read_execute: Vec<PathBuf>,
    pub runtime_read_only: Vec<PathBuf>,
    pub runtime_read_write: Vec<PathBuf>,
}

#[derive(Clone)]
pub struct ProcessSpec {
    pub program: String,
    pub arguments: Vec<String>,
    pub environment: HashMap<String, String>,
    pub sensitive_environment_keys: Vec<String>,
    pub current_dir: Option<PathBuf>,
    pub timeout: Duration,
    pub output_limit: usize,
    pub confinement: ProcessConfinement,
}

impl fmt::Debug for ProcessSpec {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut environment_keys = self.environment.keys().collect::<Vec<_>>();
        environment_keys.sort();
        formatter
            .debug_struct("ProcessSpec")
            .field("program", &self.program)
            .field("arguments", &self.arguments)
            .field("environment_keys", &environment_keys)
            .field(
                "sensitive_environment_keys",
                &self.sensitive_environment_keys,
            )
            .field("current_dir", &self.current_dir)
            .field("timeout", &self.timeout)
            .field("output_limit", &self.output_limit)
            .field("confinement", &self.confinement)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupervisionCapability {
    Available { platform: String },
    Unavailable { platform: String, reason: String },
}

impl SupervisionCapability {
    pub const fn is_available(&self) -> bool {
        matches!(self, Self::Available { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupervisionState {
    Prepared,
    Running,
    Terminated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminationReason {
    Exited { code: i32 },
    Signaled { signal: i32 },
    Cancelled,
    TimedOut,
    Requested,
    LaunchFailed,
    IdentityMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessGroupIdentity {
    UnixProcessGroup { pgid: i32 },
    Job { identity: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessStartIdentity {
    Linux {
        boot_id: String,
        start_time_ticks: u64,
    },
    Platform {
        identity: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutableIdentity {
    pub path: PathBuf,
    pub device: u64,
    pub inode: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilesystemConfinementReport {
    LegacyUnconfined,
    Unconfined,
    Landlock { policy_sha256: String, abi: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkConfinementReport {
    NotOsConfined,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessConfinementReport {
    pub filesystem: FilesystemConfinementReport,
    pub network: NetworkConfinementReport,
}

impl Default for ProcessConfinementReport {
    fn default() -> Self {
        Self {
            filesystem: FilesystemConfinementReport::LegacyUnconfined,
            network: NetworkConfinementReport::NotOsConfined,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupervisionMetadata {
    pub supervision_id: Uuid,
    pub pid: u32,
    pub process_group: ProcessGroupIdentity,
    pub process_start: ProcessStartIdentity,
    pub executable: ExecutableIdentity,
    pub command_digest: String,
    pub environment_digest: String,
    #[serde(default)]
    pub confinement: ProcessConfinementReport,
    pub supervision_token: String,
    pub state: SupervisionState,
    pub termination_reason: Option<TerminationReason>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SupervisedProcessResult {
    pub result: ProcessResult,
    pub metadata: SupervisionMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminationStatus {
    Terminated,
    AlreadyExited,
    IdentityMismatch,
    Unsupported,
    ProcessesRemain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminationResult {
    pub status: TerminationStatus,
    pub term_signal_sent: bool,
    pub kill_signal_sent: bool,
    pub remaining_processes: Vec<u32>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessSupervisor;

impl ProcessSupervisor {
    pub const fn new() -> Self {
        Self
    }

    pub fn capability(&self) -> SupervisionCapability {
        supervision_capability()
    }

    pub fn prepare(
        &self,
        spec: &ProcessSpec,
    ) -> Result<(PreparedProcess, SupervisionMetadata), ExecutionError> {
        platform::prepare(spec)
    }

    pub fn wait(
        &self,
        process: RunningProcess,
        cancellation: &CancellationToken,
    ) -> Result<SupervisedProcessResult, ExecutionError> {
        process.wait(cancellation)
    }

    pub fn terminate(
        &self,
        metadata: &SupervisionMetadata,
    ) -> Result<TerminationResult, ExecutionError> {
        platform::terminate(metadata)
    }
}

pub fn supervision_capability() -> SupervisionCapability {
    platform::capability()
}

pub fn supervise(
    spec: &ProcessSpec,
    cancellation: &CancellationToken,
) -> Result<ProcessResult, ExecutionError> {
    if cancellation.cancelled() {
        return Ok(ProcessResult {
            outcome: TerminalOutcome::Cancelled,
            stdout: Vec::new(),
            stderr: Vec::new(),
            stdout_truncated: false,
            stderr_truncated: false,
        });
    }

    let supervisor = ProcessSupervisor;
    let (prepared, _) = supervisor.prepare(spec)?;
    let running = prepared.release()?;
    Ok(supervisor.wait(running, cancellation)?.result)
}

fn command_digest(
    spec: &ProcessSpec,
    executable: &Path,
    confinement: &ProcessConfinementReport,
) -> String {
    let mut digest = Sha256::new();
    update_digest_field(&mut digest, executable.as_os_str().as_encoded_bytes());
    for argument in &spec.arguments {
        update_digest_field(&mut digest, argument.as_bytes());
    }
    if let Some(current_dir) = &spec.current_dir {
        update_digest_field(&mut digest, current_dir.as_os_str().as_encoded_bytes());
    }
    match &confinement.filesystem {
        FilesystemConfinementReport::LegacyUnconfined => {
            update_digest_field(&mut digest, b"legacy-unconfined");
        }
        FilesystemConfinementReport::Unconfined => {
            update_digest_field(&mut digest, b"unconfined");
        }
        FilesystemConfinementReport::Landlock { policy_sha256, abi } => {
            update_digest_field(&mut digest, b"landlock");
            update_digest_field(&mut digest, &[*abi]);
            update_digest_field(&mut digest, policy_sha256.as_bytes());
        }
    }
    match confinement.network {
        NetworkConfinementReport::NotOsConfined => {
            update_digest_field(&mut digest, b"network-not-os-confined");
        }
    }
    hex_digest(digest.finalize())
}

fn environment_digest(environment: &HashMap<String, String>) -> String {
    let mut entries: Vec<_> = environment.iter().collect();
    entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
    let mut digest = Sha256::new();
    for (name, value) in entries {
        update_digest_field(&mut digest, name.as_bytes());
        update_digest_field(&mut digest, value.as_bytes());
    }
    hex_digest(digest.finalize())
}

fn update_digest_field(digest: &mut Sha256, value: &[u8]) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value);
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("process supervision is unavailable: {0}")]
    Unsupported(String),
    #[error("process I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("process argument or environment contains an interior NUL byte")]
    InteriorNul,
    #[error("environment variable name is invalid: {0}")]
    InvalidEnvironmentName(String),
    #[error("executable was not found: {0}")]
    ExecutableNotFound(String),
    #[error("prepared process setup failed")]
    ChildSetupFailed,
    #[error("process confinement is invalid: {0}")]
    InvalidConfinement(&'static str),
    #[error("process confinement is unsafe: {0}")]
    UnsafeConfinement(&'static str),
    #[error("process confinement setup failed")]
    ConfinementSetupFailed,
    #[error("process output pipe was unavailable")]
    MissingPipe,
    #[error("process output reader panicked")]
    ReaderPanicked,
    #[error("process-tree termination could not be verified; remaining processes: {0:?}")]
    ProcessTerminationUnverified(Vec<u32>),
    #[error("process identity could not be read")]
    ProcessIdentityUnavailable,
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use landlock::{
        ABI, Access, AccessFs, CompatLevel, Compatible, PathBeneath, PathFd, Ruleset, RulesetAttr,
        RulesetCreatedAttr, make_bitflags,
    };
    use std::{
        ffi::{CString, OsStr},
        fs::File,
        io::{Read, Write},
        os::{
            fd::{AsFd, AsRawFd, FromRawFd, OwnedFd, RawFd},
            unix::{ffi::OsStrExt, fs::MetadataExt},
        },
        ptr,
        thread::{self, JoinHandle},
        time::Instant,
    };

    type BoundedReadResult = Result<(Vec<u8>, bool), std::io::Error>;
    type ReaderHandle = JoinHandle<BoundedReadResult>;

    const POLL_INTERVAL: Duration = Duration::from_millis(10);
    const TERM_GRACE_PERIOD: Duration = Duration::from_millis(250);
    const KILL_VERIFICATION_PERIOD: Duration = Duration::from_secs(2);

    pub fn capability() -> SupervisionCapability {
        SupervisionCapability::Available {
            platform: "linux".to_owned(),
        }
    }

    pub struct PreparedProcess {
        pid: libc::pid_t,
        release_writer: Option<OwnedFd>,
        stdout: Option<File>,
        stderr: Option<File>,
        metadata: SupervisionMetadata,
        timeout: Duration,
        output_limit: usize,
        released: bool,
    }

    impl std::fmt::Debug for PreparedProcess {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter
                .debug_struct("PreparedProcess")
                .field("metadata", &self.metadata)
                .finish_non_exhaustive()
        }
    }

    impl PreparedProcess {
        pub fn metadata(&self) -> &SupervisionMetadata {
            &self.metadata
        }

        pub fn release(mut self) -> Result<RunningProcess, ExecutionError> {
            let release_writer = self
                .release_writer
                .take()
                .ok_or(ExecutionError::ChildSetupFailed)?;
            let mut release_writer = File::from(release_writer);
            if let Err(error) = release_writer.write_all(&[1]) {
                drop(release_writer);
                let _ = terminate_owned(&self.metadata);
                reap_blocking(self.pid)?;
                self.released = true;
                return Err(error.into());
            }
            drop(release_writer);

            self.metadata.state = SupervisionState::Running;
            let stdout = self.stdout.take().ok_or(ExecutionError::MissingPipe)?;
            let stderr = self.stderr.take().ok_or(ExecutionError::MissingPipe)?;
            let output_limit = self.output_limit;
            let stdout_reader = thread::spawn(move || read_bounded(stdout, output_limit));
            let stderr_reader = thread::spawn(move || read_bounded(stderr, output_limit));
            self.released = true;

            Ok(RunningProcess {
                pid: self.pid,
                metadata: self.metadata.clone(),
                timeout: self.timeout,
                stdout_reader: Some(stdout_reader),
                stderr_reader: Some(stderr_reader),
                finished: false,
            })
        }
    }

    impl Drop for PreparedProcess {
        fn drop(&mut self) {
            if self.released {
                return;
            }
            self.release_writer.take();
            let _ = terminate_owned(&self.metadata);
            let _ = reap_blocking(self.pid);
        }
    }

    pub struct RunningProcess {
        pid: libc::pid_t,
        metadata: SupervisionMetadata,
        timeout: Duration,
        stdout_reader: Option<ReaderHandle>,
        stderr_reader: Option<ReaderHandle>,
        finished: bool,
    }

    impl std::fmt::Debug for RunningProcess {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter
                .debug_struct("RunningProcess")
                .field("metadata", &self.metadata)
                .finish_non_exhaustive()
        }
    }

    impl RunningProcess {
        pub fn metadata(&self) -> &SupervisionMetadata {
            &self.metadata
        }

        pub fn wait(
            mut self,
            cancellation: &CancellationToken,
        ) -> Result<SupervisedProcessResult, ExecutionError> {
            let started = Instant::now();
            let (outcome, reason) = loop {
                if let Some(status) = try_wait(self.pid)? {
                    ensure_termination_verified(terminate_owned(&self.metadata)?)?;
                    break outcome_from_wait_status(status);
                }
                if cancellation.cancelled() {
                    ensure_termination_verified(terminate_owned(&self.metadata)?)?;
                    let _ = reap_blocking(self.pid)?;
                    break (TerminalOutcome::Cancelled, TerminationReason::Cancelled);
                }
                if started.elapsed() >= self.timeout {
                    ensure_termination_verified(terminate_owned(&self.metadata)?)?;
                    let _ = reap_blocking(self.pid)?;
                    break (TerminalOutcome::TimedOut, TerminationReason::TimedOut);
                }
                thread::sleep(POLL_INTERVAL);
            };

            self.finished = true;
            self.metadata.state = SupervisionState::Terminated;
            self.metadata.termination_reason = Some(reason);
            let (stdout, stdout_truncated) = join_reader(self.stdout_reader.take())?;
            let (stderr, stderr_truncated) = join_reader(self.stderr_reader.take())?;

            Ok(SupervisedProcessResult {
                result: ProcessResult {
                    outcome,
                    stdout,
                    stderr,
                    stdout_truncated,
                    stderr_truncated,
                },
                metadata: self.metadata.clone(),
            })
        }
    }

    impl Drop for RunningProcess {
        fn drop(&mut self) {
            if self.finished {
                return;
            }
            let _ = terminate_owned(&self.metadata);
            let _ = reap_blocking(self.pid);
            if let Some(reader) = self.stdout_reader.take() {
                let _ = reader.join();
            }
            if let Some(reader) = self.stderr_reader.take() {
                let _ = reader.join();
            }
            self.finished = true;
        }
    }

    struct CompiledConfinement {
        ruleset: Option<OwnedFd>,
        report: ProcessConfinementReport,
    }

    struct CanonicalFilesystemPolicy {
        workspace: PathBuf,
        writable_state: PathBuf,
        runtime_read_execute: Vec<PathBuf>,
        runtime_read_only: Vec<PathBuf>,
        runtime_read_write: Vec<PathBuf>,
    }

    fn compile_confinement(
        confinement: &ProcessConfinement,
    ) -> Result<CompiledConfinement, ExecutionError> {
        match confinement {
            ProcessConfinement::Unconfined => Ok(CompiledConfinement {
                ruleset: None,
                report: ProcessConfinementReport {
                    filesystem: FilesystemConfinementReport::Unconfined,
                    network: NetworkConfinementReport::NotOsConfined,
                },
            }),
            ProcessConfinement::LinuxFilesystem(policy) => compile_linux_filesystem(policy),
        }
    }

    fn compile_linux_filesystem(
        policy: &LinuxFilesystemConfinement,
    ) -> Result<CompiledConfinement, ExecutionError> {
        let policy = canonicalize_policy(policy)?;
        validate_policy(&policy)?;
        let policy_sha256 = policy_digest(&policy);
        let abi = ABI::V3;
        let all_access = AccessFs::from_all(abi);
        let read_only = make_bitflags!(AccessFs::{ReadFile | ReadDir});
        let read_execute = make_bitflags!(AccessFs::{ReadFile | ReadDir | Execute});
        let read_write = make_bitflags!(AccessFs::{ReadFile | WriteFile | Truncate});
        let writable = all_access & !AccessFs::Execute;
        let mut ruleset = Ruleset::default()
            .set_compatibility(CompatLevel::HardRequirement)
            .handle_access(all_access)
            .and_then(Ruleset::create)
            .map_err(|_| ExecutionError::ConfinementSetupFailed)?;
        add_path_rule(&mut ruleset, &policy.workspace, read_only)?;
        add_path_rule(&mut ruleset, &policy.writable_state, writable)?;
        for path in &policy.runtime_read_execute {
            add_path_rule(&mut ruleset, path, read_execute)?;
        }
        for path in &policy.runtime_read_only {
            add_path_rule(&mut ruleset, path, read_only)?;
        }
        for path in &policy.runtime_read_write {
            add_path_rule(&mut ruleset, path, read_write)?;
        }
        let ruleset: Option<OwnedFd> = ruleset.into();
        let ruleset = ruleset.ok_or(ExecutionError::ConfinementSetupFailed)?;
        Ok(CompiledConfinement {
            ruleset: Some(ruleset),
            report: ProcessConfinementReport {
                filesystem: FilesystemConfinementReport::Landlock {
                    policy_sha256,
                    abi: 3,
                },
                network: NetworkConfinementReport::NotOsConfined,
            },
        })
    }

    fn add_path_rule(
        ruleset: &mut landlock::RulesetCreated,
        path: &Path,
        access: landlock::BitFlags<AccessFs>,
    ) -> Result<(), ExecutionError> {
        let path_fd = PathFd::new(path).map_err(|_| ExecutionError::ConfinementSetupFailed)?;
        let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe { libc::fstat(path_fd.as_fd().as_raw_fd(), metadata.as_mut_ptr()) } != 0 {
            return Err(ExecutionError::ConfinementSetupFailed);
        }
        let metadata = unsafe { metadata.assume_init() };
        let is_directory = metadata.st_mode & libc::S_IFMT == libc::S_IFDIR;
        let access = if is_directory {
            access
        } else {
            access & !make_bitflags!(AccessFs::{ReadDir})
        };
        ruleset
            .add_rule(PathBeneath::new(path_fd, access))
            .map_err(|_| ExecutionError::ConfinementSetupFailed)?;
        Ok(())
    }

    fn canonicalize_policy(
        policy: &LinuxFilesystemConfinement,
    ) -> Result<CanonicalFilesystemPolicy, ExecutionError> {
        Ok(CanonicalFilesystemPolicy {
            workspace: canonicalize_directory(&policy.workspace, "workspace path is unavailable")?,
            writable_state: canonicalize_directory(
                &policy.writable_state,
                "writable state path is unavailable",
            )?,
            runtime_read_execute: canonicalize_paths(&policy.runtime_read_execute)?,
            runtime_read_only: canonicalize_paths(&policy.runtime_read_only)?,
            runtime_read_write: canonicalize_paths(&policy.runtime_read_write)?,
        })
    }

    fn canonicalize_directory(
        path: &Path,
        unavailable_message: &'static str,
    ) -> Result<PathBuf, ExecutionError> {
        let canonical = std::fs::canonicalize(path)
            .map_err(|_| ExecutionError::InvalidConfinement(unavailable_message))?;
        if !canonical.is_dir() {
            return Err(ExecutionError::InvalidConfinement(
                "workspace and writable state must be directories",
            ));
        }
        Ok(canonical)
    }

    fn canonicalize_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>, ExecutionError> {
        let mut canonical = Vec::with_capacity(paths.len());
        for path in paths {
            canonical.push(
                std::fs::canonicalize(path).map_err(|_| {
                    ExecutionError::InvalidConfinement("runtime path is unavailable")
                })?,
            );
        }
        canonical.sort_unstable();
        canonical.dedup();
        Ok(canonical)
    }

    fn validate_policy(policy: &CanonicalFilesystemPolicy) -> Result<(), ExecutionError> {
        let root = Path::new("/");
        let all_paths = std::iter::once(&policy.workspace)
            .chain(std::iter::once(&policy.writable_state))
            .chain(policy.runtime_read_execute.iter())
            .chain(policy.runtime_read_only.iter())
            .chain(policy.runtime_read_write.iter());
        if all_paths.into_iter().any(|path| path == root) {
            return Err(ExecutionError::UnsafeConfinement(
                "filesystem root cannot be granted",
            ));
        }
        if paths_overlap(&policy.workspace, &policy.writable_state) {
            return Err(ExecutionError::UnsafeConfinement(
                "workspace and writable state must not overlap",
            ));
        }
        for runtime in policy
            .runtime_read_execute
            .iter()
            .chain(policy.runtime_read_only.iter())
            .chain(policy.runtime_read_write.iter())
        {
            if paths_overlap(runtime, &policy.workspace)
                || paths_overlap(runtime, &policy.writable_state)
            {
                return Err(ExecutionError::UnsafeConfinement(
                    "runtime paths cannot overlap workspace or writable state",
                ));
            }
        }
        ensure_permission_classes_do_not_overlap(
            &policy.runtime_read_execute,
            &policy.runtime_read_only,
        )?;
        ensure_permission_classes_do_not_overlap(
            &policy.runtime_read_execute,
            &policy.runtime_read_write,
        )?;
        ensure_permission_classes_do_not_overlap(
            &policy.runtime_read_only,
            &policy.runtime_read_write,
        )?;
        Ok(())
    }

    fn ensure_permission_classes_do_not_overlap(
        left: &[PathBuf],
        right: &[PathBuf],
    ) -> Result<(), ExecutionError> {
        if left
            .iter()
            .any(|left| right.iter().any(|right| paths_overlap(left, right)))
        {
            return Err(ExecutionError::UnsafeConfinement(
                "runtime permission classes must not overlap",
            ));
        }
        Ok(())
    }

    fn paths_overlap(left: &Path, right: &Path) -> bool {
        left.starts_with(right) || right.starts_with(left)
    }

    fn policy_digest(policy: &CanonicalFilesystemPolicy) -> String {
        let mut digest = Sha256::new();
        update_digest_field(&mut digest, b"jet-black-landlock-filesystem-v3");
        update_policy_path(&mut digest, b"workspace-read-only", &policy.workspace);
        update_policy_path(
            &mut digest,
            b"writable-state-all-except-execute",
            &policy.writable_state,
        );
        for path in &policy.runtime_read_execute {
            update_policy_path(&mut digest, b"runtime-read-execute", path);
        }
        for path in &policy.runtime_read_only {
            update_policy_path(&mut digest, b"runtime-read-only", path);
        }
        for path in &policy.runtime_read_write {
            update_policy_path(&mut digest, b"runtime-read-write", path);
        }
        hex_digest(digest.finalize())
    }

    fn update_policy_path(digest: &mut Sha256, kind: &[u8], path: &Path) {
        update_digest_field(digest, kind);
        update_digest_field(digest, path.as_os_str().as_bytes());
    }

    pub fn prepare(
        spec: &ProcessSpec,
    ) -> Result<(PreparedProcess, SupervisionMetadata), ExecutionError> {
        let executable_path = resolve_executable(spec)?;
        let executable_metadata = std::fs::metadata(&executable_path)?;
        let executable = ExecutableIdentity {
            path: executable_path.clone(),
            device: executable_metadata.dev(),
            inode: executable_metadata.ino(),
        };
        let token = Uuid::new_v4().to_string();
        let CompiledConfinement {
            ruleset,
            report: confinement_report,
        } = compile_confinement(&spec.confinement)?;
        let child_state = ChildState::new(spec, &executable_path, &token, ruleset)?;

        let (release_reader, release_writer) = pipe()?;
        let (ready_reader, ready_writer) = pipe()?;
        let (stdout_reader, stdout_writer) = pipe()?;
        let (stderr_reader, stderr_writer) = pipe()?;
        let null = open_null()?;

        let pid = unsafe { libc::fork() };
        if pid < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        if pid == 0 {
            child_main(
                &child_state,
                release_reader.as_raw_fd(),
                ready_writer.as_raw_fd(),
                stdout_writer.as_raw_fd(),
                stderr_writer.as_raw_fd(),
                null.as_raw_fd(),
                &[
                    release_writer.as_raw_fd(),
                    ready_reader.as_raw_fd(),
                    stdout_reader.as_raw_fd(),
                    stderr_reader.as_raw_fd(),
                ],
            );
        }

        drop(release_reader);
        drop(ready_writer);
        drop(stdout_writer);
        drop(stderr_writer);
        drop(null);

        match wait_until_ready(ready_reader) {
            Ok(ChildReadyStatus::Ready) => {}
            Ok(ChildReadyStatus::ConfinementFailed) => {
                drop(release_writer);
                let _ = reap_blocking(pid);
                return Err(ExecutionError::ConfinementSetupFailed);
            }
            Ok(ChildReadyStatus::SetupFailed) => {
                drop(release_writer);
                let _ = reap_blocking(pid);
                return Err(ExecutionError::ChildSetupFailed);
            }
            Err(error) => {
                drop(release_writer);
                let _ = signal_pid(pid, libc::SIGKILL);
                let _ = reap_blocking(pid);
                return Err(error);
            }
        }

        let process_start = read_start_identity(pid)?.ok_or_else(|| {
            let _ = signal_pid(pid, libc::SIGKILL);
            let _ = reap_blocking(pid);
            ExecutionError::ProcessIdentityUnavailable
        })?;
        let metadata = SupervisionMetadata {
            supervision_id: Uuid::new_v4(),
            pid: pid as u32,
            process_group: ProcessGroupIdentity::UnixProcessGroup { pgid: pid },
            process_start,
            executable,
            command_digest: command_digest(spec, &executable_path, &confinement_report),
            environment_digest: environment_digest(&spec.environment),
            confinement: confinement_report,
            supervision_token: token,
            state: SupervisionState::Prepared,
            termination_reason: None,
        };
        let prepared = PreparedProcess {
            pid,
            release_writer: Some(release_writer),
            stdout: Some(File::from(stdout_reader)),
            stderr: Some(File::from(stderr_reader)),
            metadata: metadata.clone(),
            timeout: spec.timeout,
            output_limit: spec.output_limit,
            released: false,
        };
        Ok((prepared, metadata))
    }

    pub fn terminate(metadata: &SupervisionMetadata) -> Result<TerminationResult, ExecutionError> {
        let pid = metadata.pid as libc::pid_t;
        let Some(current_identity) = read_start_identity(pid)? else {
            return terminate_token_owned(metadata, None, TerminationStatus::AlreadyExited);
        };
        if current_identity != metadata.process_start {
            return terminate_token_owned(
                metadata,
                Some(metadata.pid),
                TerminationStatus::IdentityMismatch,
            );
        }
        terminate_owned(metadata)
    }

    struct ChildState {
        executable: CString,
        arguments: Vec<CString>,
        argument_pointers: Vec<*const libc::c_char>,
        environment: Vec<CString>,
        environment_pointers: Vec<*const libc::c_char>,
        current_dir: Option<CString>,
        confinement_ruleset: Option<OwnedFd>,
    }

    impl ChildState {
        fn new(
            spec: &ProcessSpec,
            executable: &Path,
            token: &str,
            confinement_ruleset: Option<OwnedFd>,
        ) -> Result<Self, ExecutionError> {
            let executable = os_string_to_cstring(executable.as_os_str())?;
            let mut arguments = Vec::with_capacity(spec.arguments.len() + 1);
            arguments.push(executable.clone());
            for argument in &spec.arguments {
                arguments.push(
                    CString::new(argument.as_bytes()).map_err(|_| ExecutionError::InteriorNul)?,
                );
            }
            let mut argument_pointers: Vec<_> =
                arguments.iter().map(|value| value.as_ptr()).collect();
            argument_pointers.push(ptr::null());

            let mut environment_entries: Vec<_> = spec.environment.iter().collect();
            environment_entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
            let mut environment = Vec::with_capacity(environment_entries.len() + 1);
            for (name, value) in environment_entries {
                if name.is_empty() || name.contains('=') {
                    return Err(ExecutionError::InvalidEnvironmentName(name.clone()));
                }
                environment.push(
                    CString::new(format!("{name}={value}"))
                        .map_err(|_| ExecutionError::InteriorNul)?,
                );
            }
            environment.push(
                CString::new(format!("{SUPERVISION_TOKEN_ENV}={token}"))
                    .map_err(|_| ExecutionError::InteriorNul)?,
            );
            let mut environment_pointers: Vec<_> =
                environment.iter().map(|value| value.as_ptr()).collect();
            environment_pointers.push(ptr::null());
            let current_dir = spec
                .current_dir
                .as_deref()
                .map(|path| os_string_to_cstring(path.as_os_str()))
                .transpose()?;

            Ok(Self {
                executable,
                arguments,
                argument_pointers,
                environment,
                environment_pointers,
                current_dir,
                confinement_ruleset,
            })
        }
    }

    fn child_main(
        state: &ChildState,
        release_reader: RawFd,
        ready_writer: RawFd,
        stdout_writer: RawFd,
        stderr_writer: RawFd,
        null: RawFd,
        close_fds: &[RawFd],
    ) -> ! {
        let _keep_allocations_alive = (&state.arguments, &state.environment);
        for fd in close_fds {
            unsafe {
                libc::close(*fd);
            }
        }
        if unsafe { libc::setpgid(0, 0) } != 0
            || unsafe { libc::dup2(null, libc::STDIN_FILENO) } < 0
            || unsafe { libc::dup2(stdout_writer, libc::STDOUT_FILENO) } < 0
            || unsafe { libc::dup2(stderr_writer, libc::STDERR_FILENO) } < 0
        {
            unsafe { libc::_exit(126) };
        }
        unsafe {
            libc::close(null);
            libc::close(stdout_writer);
            libc::close(stderr_writer);
        }
        let ruleset_fd = state.confinement_ruleset.as_ref().map(AsRawFd::as_raw_fd);
        if !close_unlisted_fds(release_reader, ready_writer, ruleset_fd) {
            unsafe { libc::_exit(126) };
        }
        if let Some(current_dir) = &state.current_dir {
            if unsafe { libc::chdir(current_dir.as_ptr()) } != 0 {
                unsafe { libc::_exit(126) };
            }
        }
        if let Some(ruleset) = &state.confinement_ruleset {
            if !apply_landlock(ruleset.as_raw_fd()) {
                let _ = write_child_byte(ready_writer, 2);
                unsafe { libc::_exit(126) };
            }
        }
        if !write_child_byte(ready_writer, 1) {
            unsafe { libc::_exit(126) };
        }
        unsafe {
            libc::close(ready_writer);
        }
        if !read_child_release(release_reader) {
            unsafe { libc::_exit(125) };
        }
        unsafe {
            libc::close(release_reader);
            libc::execve(
                state.executable.as_ptr(),
                state.argument_pointers.as_ptr(),
                state.environment_pointers.as_ptr(),
            );
            libc::_exit(127);
        }
    }

    fn close_unlisted_fds(
        release_reader: RawFd,
        ready_writer: RawFd,
        ruleset_fd: Option<RawFd>,
    ) -> bool {
        let mut retained = [release_reader, ready_writer, ruleset_fd.unwrap_or(-1)];
        retained.sort_unstable();
        let mut first = 3_u32;
        for fd in retained {
            let Ok(fd) = u32::try_from(fd) else {
                continue;
            };
            if fd < first {
                continue;
            }
            if first < fd && !close_fd_range(first, fd - 1) {
                return false;
            }
            first = fd.saturating_add(1);
        }
        first == u32::MAX || close_fd_range(first, u32::MAX)
    }

    fn close_fd_range(first: u32, last: u32) -> bool {
        loop {
            if unsafe { libc::syscall(libc::SYS_close_range, first, last, 0_u32) } == 0 {
                return true;
            }
            if std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                return false;
            }
        }
    }

    fn apply_landlock(ruleset_fd: RawFd) -> bool {
        if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
            return false;
        }
        unsafe { libc::syscall(libc::SYS_landlock_restrict_self, ruleset_fd, 0_u32) == 0 }
    }

    fn read_child_release(fd: RawFd) -> bool {
        let mut byte = 0_u8;
        loop {
            let read = unsafe { libc::read(fd, (&mut byte as *mut u8).cast(), 1) };
            if read == 1 {
                return byte == 1;
            }
            if read == 0 {
                return false;
            }
            if std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                return false;
            }
        }
    }

    fn write_child_byte(fd: RawFd, byte: u8) -> bool {
        loop {
            let written = unsafe { libc::write(fd, (&byte as *const u8).cast(), 1) };
            if written == 1 {
                return true;
            }
            if std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                return false;
            }
        }
    }

    enum ChildReadyStatus {
        Ready,
        ConfinementFailed,
        SetupFailed,
    }

    fn wait_until_ready(reader: OwnedFd) -> Result<ChildReadyStatus, ExecutionError> {
        let mut reader = File::from(reader);
        let mut byte = [0_u8; 1];
        loop {
            match reader.read(&mut byte) {
                Ok(1) if byte[0] == 1 => return Ok(ChildReadyStatus::Ready),
                Ok(1) if byte[0] == 2 => return Ok(ChildReadyStatus::ConfinementFailed),
                Ok(_) => return Ok(ChildReadyStatus::SetupFailed),
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error.into()),
            }
        }
    }

    fn resolve_executable(spec: &ProcessSpec) -> Result<PathBuf, ExecutionError> {
        let requested = Path::new(&spec.program);
        if requested.components().count() > 1 || requested.is_absolute() {
            let path = if requested.is_absolute() {
                requested.to_path_buf()
            } else {
                spec.current_dir
                    .as_deref()
                    .unwrap_or_else(|| Path::new("."))
                    .join(requested)
            };
            return std::fs::canonicalize(&path)
                .map_err(|_| ExecutionError::ExecutableNotFound(spec.program.clone()));
        }

        let search_path = spec
            .environment
            .get("PATH")
            .map(OsStr::new)
            .unwrap_or_else(|| OsStr::new("/usr/bin:/bin"));
        for directory in std::env::split_paths(search_path) {
            let candidate = directory.join(requested);
            if candidate.is_file() {
                return std::fs::canonicalize(candidate)
                    .map_err(|_| ExecutionError::ExecutableNotFound(spec.program.clone()));
            }
        }
        Err(ExecutionError::ExecutableNotFound(spec.program.clone()))
    }

    fn os_string_to_cstring(value: &OsStr) -> Result<CString, ExecutionError> {
        CString::new(value.as_bytes()).map_err(|_| ExecutionError::InteriorNul)
    }

    fn pipe() -> Result<(OwnedFd, OwnedFd), ExecutionError> {
        let mut fds = [-1; 2];
        if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
    }

    fn open_null() -> Result<OwnedFd, ExecutionError> {
        let path = c"/dev/null";
        let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }

    fn read_start_identity(
        pid: libc::pid_t,
    ) -> Result<Option<ProcessStartIdentity>, ExecutionError> {
        let Some(stat) = read_process_stat(pid)? else {
            return Ok(None);
        };
        let boot_id = std::fs::read_to_string("/proc/sys/kernel/random/boot_id")?
            .trim()
            .to_owned();
        Ok(Some(ProcessStartIdentity::Linux {
            boot_id,
            start_time_ticks: stat.start_time_ticks,
        }))
    }

    struct ProcessStat {
        state: u8,
        process_group: i32,
        start_time_ticks: u64,
    }

    fn read_process_stat(pid: libc::pid_t) -> Result<Option<ProcessStat>, ExecutionError> {
        let path = format!("/proc/{pid}/stat");
        let stat = match std::fs::read(path) {
            Ok(stat) => stat,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let closing_parenthesis = stat
            .iter()
            .rposition(|byte| *byte == b')')
            .ok_or(ExecutionError::ProcessIdentityUnavailable)?;
        let fields = std::str::from_utf8(
            stat.get(closing_parenthesis + 2..)
                .ok_or(ExecutionError::ProcessIdentityUnavailable)?,
        )
        .map_err(|_| ExecutionError::ProcessIdentityUnavailable)?
        .split_ascii_whitespace()
        .collect::<Vec<_>>();
        if fields.len() <= 19 {
            return Err(ExecutionError::ProcessIdentityUnavailable);
        }
        Ok(Some(ProcessStat {
            state: fields[0]
                .as_bytes()
                .first()
                .copied()
                .ok_or(ExecutionError::ProcessIdentityUnavailable)?,
            process_group: fields[2]
                .parse()
                .map_err(|_| ExecutionError::ProcessIdentityUnavailable)?,
            start_time_ticks: fields[19]
                .parse()
                .map_err(|_| ExecutionError::ProcessIdentityUnavailable)?,
        }))
    }

    fn terminate_token_owned(
        metadata: &SupervisionMetadata,
        excluded_pid: Option<u32>,
        empty_status: TerminationStatus,
    ) -> Result<TerminationResult, ExecutionError> {
        let mut remaining_processes =
            token_processes_excluding(&metadata.supervision_token, excluded_pid)?;
        if remaining_processes.is_empty() {
            return Ok(TerminationResult {
                status: empty_status,
                term_signal_sent: false,
                kill_signal_sent: false,
                remaining_processes,
            });
        }

        let term_signal_sent = signal_processes(&remaining_processes, libc::SIGTERM)?;
        let term_deadline = Instant::now() + TERM_GRACE_PERIOD;
        while Instant::now() < term_deadline {
            remaining_processes =
                token_processes_excluding(&metadata.supervision_token, excluded_pid)?;
            if remaining_processes.is_empty() {
                return Ok(TerminationResult {
                    status: if empty_status == TerminationStatus::IdentityMismatch {
                        empty_status
                    } else {
                        TerminationStatus::Terminated
                    },
                    term_signal_sent,
                    kill_signal_sent: false,
                    remaining_processes,
                });
            }
            thread::sleep(POLL_INTERVAL);
        }

        let kill_signal_sent = signal_processes(&remaining_processes, libc::SIGKILL)?;
        let kill_deadline = Instant::now() + KILL_VERIFICATION_PERIOD;
        loop {
            remaining_processes =
                token_processes_excluding(&metadata.supervision_token, excluded_pid)?;
            if remaining_processes.is_empty() || Instant::now() >= kill_deadline {
                break;
            }
            thread::sleep(POLL_INTERVAL);
        }
        Ok(TerminationResult {
            status: if remaining_processes.is_empty() {
                if empty_status == TerminationStatus::IdentityMismatch {
                    empty_status
                } else {
                    TerminationStatus::Terminated
                }
            } else {
                TerminationStatus::ProcessesRemain
            },
            term_signal_sent,
            kill_signal_sent,
            remaining_processes,
        })
    }

    fn terminate_owned(
        metadata: &SupervisionMetadata,
    ) -> Result<TerminationResult, ExecutionError> {
        let pgid = match metadata.process_group {
            ProcessGroupIdentity::UnixProcessGroup { pgid } => pgid,
            ProcessGroupIdentity::Job { .. } => {
                return Ok(TerminationResult {
                    status: TerminationStatus::Unsupported,
                    term_signal_sent: false,
                    kill_signal_sent: false,
                    remaining_processes: Vec::new(),
                });
            }
        };

        let mut term_signal_sent = signal_group(pgid, libc::SIGTERM)?;
        term_signal_sent |= signal_token_processes(&metadata.supervision_token, libc::SIGTERM)?;
        let term_deadline = Instant::now() + TERM_GRACE_PERIOD;
        while Instant::now() < term_deadline {
            if supervised_processes(metadata)?.is_empty() {
                return Ok(TerminationResult {
                    status: TerminationStatus::Terminated,
                    term_signal_sent,
                    kill_signal_sent: false,
                    remaining_processes: Vec::new(),
                });
            }
            thread::sleep(POLL_INTERVAL);
        }

        let mut kill_signal_sent = signal_group(pgid, libc::SIGKILL)?;
        kill_signal_sent |= signal_token_processes(&metadata.supervision_token, libc::SIGKILL)?;
        let kill_deadline = Instant::now() + KILL_VERIFICATION_PERIOD;
        let remaining_processes = loop {
            let remaining = supervised_processes(metadata)?;
            if remaining.is_empty() || Instant::now() >= kill_deadline {
                break remaining;
            }
            thread::sleep(POLL_INTERVAL);
        };
        Ok(TerminationResult {
            status: if remaining_processes.is_empty() {
                TerminationStatus::Terminated
            } else {
                TerminationStatus::ProcessesRemain
            },
            term_signal_sent,
            kill_signal_sent,
            remaining_processes,
        })
    }

    fn ensure_termination_verified(result: TerminationResult) -> Result<(), ExecutionError> {
        match result.status {
            TerminationStatus::Terminated | TerminationStatus::AlreadyExited => Ok(()),
            TerminationStatus::IdentityMismatch
            | TerminationStatus::Unsupported
            | TerminationStatus::ProcessesRemain => Err(
                ExecutionError::ProcessTerminationUnverified(result.remaining_processes),
            ),
        }
    }

    fn supervised_processes(metadata: &SupervisionMetadata) -> Result<Vec<u32>, ExecutionError> {
        let pgid = match metadata.process_group {
            ProcessGroupIdentity::UnixProcessGroup { pgid } => pgid,
            ProcessGroupIdentity::Job { .. } => return Ok(Vec::new()),
        };
        let mut processes = process_group_members(pgid)?;
        processes.extend(token_processes(&metadata.supervision_token)?);
        processes.sort_unstable();
        processes.dedup();
        Ok(processes)
    }

    fn process_group_members(pgid: i32) -> Result<Vec<u32>, ExecutionError> {
        let mut processes = Vec::new();
        for pid in proc_pids()? {
            let Some(stat) = read_process_stat(pid)? else {
                continue;
            };
            if stat.process_group == pgid && stat.state != b'Z' {
                processes.push(pid as u32);
            }
        }
        Ok(processes)
    }

    fn token_processes(token: &str) -> Result<Vec<u32>, ExecutionError> {
        token_processes_excluding(token, None)
    }

    fn token_processes_excluding(
        token: &str,
        excluded_pid: Option<u32>,
    ) -> Result<Vec<u32>, ExecutionError> {
        let marker = format!("{SUPERVISION_TOKEN_ENV}={token}\0");
        let mut processes = Vec::new();
        for pid in proc_pids()? {
            if pid == std::process::id() as i32 || excluded_pid == Some(pid as u32) {
                continue;
            }
            let Some(stat) = read_process_stat(pid)? else {
                continue;
            };
            if stat.state == b'Z' {
                continue;
            }
            let environment = match std::fs::read(format!("/proc/{pid}/environ")) {
                Ok(environment) => environment,
                Err(_) => continue,
            };
            if environment
                .windows(marker.len())
                .any(|window| window == marker.as_bytes())
            {
                processes.push(pid as u32);
            }
        }
        Ok(processes)
    }

    fn proc_pids() -> Result<Vec<libc::pid_t>, ExecutionError> {
        let mut pids = Vec::new();
        for entry in std::fs::read_dir("/proc")? {
            let entry = entry?;
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if let Ok(pid) = name.parse() {
                pids.push(pid);
            }
        }
        Ok(pids)
    }

    fn signal_token_processes(token: &str, signal: i32) -> Result<bool, ExecutionError> {
        signal_processes(&token_processes(token)?, signal)
    }

    fn signal_processes(processes: &[u32], signal: i32) -> Result<bool, ExecutionError> {
        let mut sent = false;
        for pid in processes {
            sent |= signal_pid(*pid as i32, signal)?;
        }
        Ok(sent)
    }

    fn signal_group(pgid: i32, signal: i32) -> Result<bool, ExecutionError> {
        signal_pid(-pgid, signal)
    }

    fn signal_pid(pid: i32, signal: i32) -> Result<bool, ExecutionError> {
        if unsafe { libc::kill(pid, signal) } == 0 {
            return Ok(true);
        }
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            return Ok(false);
        }
        Err(error.into())
    }

    fn try_wait(pid: libc::pid_t) -> Result<Option<i32>, ExecutionError> {
        let mut status = 0;
        loop {
            let result = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
            if result == pid {
                return Ok(Some(status));
            }
            if result == 0 {
                return Ok(None);
            }
            if result < 0 {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                if error.raw_os_error() == Some(libc::ECHILD) {
                    return Ok(Some(0));
                }
                return Err(error.into());
            }
        }
    }

    fn reap_blocking(pid: libc::pid_t) -> Result<i32, ExecutionError> {
        let mut status = 0;
        loop {
            let result = unsafe { libc::waitpid(pid, &mut status, 0) };
            if result == pid {
                return Ok(status);
            }
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            if error.raw_os_error() == Some(libc::ECHILD) {
                return Ok(status);
            }
            return Err(error.into());
        }
    }

    fn outcome_from_wait_status(status: i32) -> (TerminalOutcome, TerminationReason) {
        if libc::WIFEXITED(status) {
            let code = libc::WEXITSTATUS(status);
            let outcome = if code == 0 {
                TerminalOutcome::Completed(0)
            } else {
                TerminalOutcome::Failed
            };
            return (outcome, TerminationReason::Exited { code });
        }
        if libc::WIFSIGNALED(status) {
            return (
                TerminalOutcome::Failed,
                TerminationReason::Signaled {
                    signal: libc::WTERMSIG(status),
                },
            );
        }
        (TerminalOutcome::Failed, TerminationReason::LaunchFailed)
    }

    fn read_bounded(mut reader: File, limit: usize) -> Result<(Vec<u8>, bool), std::io::Error> {
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

    fn join_reader(reader: Option<ReaderHandle>) -> Result<(Vec<u8>, bool), ExecutionError> {
        reader
            .ok_or(ExecutionError::MissingPipe)?
            .join()
            .map_err(|_| ExecutionError::ReaderPanicked)?
            .map_err(ExecutionError::from)
    }
}

#[cfg(not(target_os = "linux"))]
mod platform {
    use super::*;

    fn unavailable() -> SupervisionCapability {
        SupervisionCapability::Unavailable {
            platform: std::env::consts::OS.to_owned(),
            reason: "durable process identity and descendant discovery are not implemented"
                .to_owned(),
        }
    }

    pub fn capability() -> SupervisionCapability {
        unavailable()
    }

    #[derive(Debug)]
    pub struct PreparedProcess;

    impl PreparedProcess {
        pub fn metadata(&self) -> &SupervisionMetadata {
            unreachable!("unsupported platforms cannot prepare processes")
        }

        pub fn release(self) -> Result<RunningProcess, ExecutionError> {
            Err(ExecutionError::Unsupported(format!("{:?}", unavailable())))
        }
    }

    #[derive(Debug)]
    pub struct RunningProcess;

    impl RunningProcess {
        pub fn metadata(&self) -> &SupervisionMetadata {
            unreachable!("unsupported platforms cannot run supervised processes")
        }

        pub fn wait(
            self,
            _cancellation: &CancellationToken,
        ) -> Result<SupervisedProcessResult, ExecutionError> {
            Err(ExecutionError::Unsupported(format!("{:?}", unavailable())))
        }
    }

    pub fn prepare(
        _spec: &ProcessSpec,
    ) -> Result<(PreparedProcess, SupervisionMetadata), ExecutionError> {
        Err(ExecutionError::Unsupported(format!("{:?}", unavailable())))
    }

    pub fn terminate(_metadata: &SupervisionMetadata) -> Result<TerminationResult, ExecutionError> {
        Ok(TerminationResult {
            status: TerminationStatus::Unsupported,
            term_signal_sent: false,
            kill_signal_sent: false,
            remaining_processes: Vec::new(),
        })
    }
}

pub use platform::{PreparedProcess, RunningProcess};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_spec_debug_redacts_environment_values() {
        let spec = ProcessSpec {
            program: "provider".to_owned(),
            arguments: vec!["run".to_owned()],
            environment: HashMap::from([("PROVIDER_API_KEY".to_owned(), "top-secret".to_owned())]),
            sensitive_environment_keys: vec!["PROVIDER_API_KEY".to_owned()],
            current_dir: None,
            timeout: Duration::from_secs(1),
            output_limit: 1024,
            confinement: ProcessConfinement::Unconfined,
        };

        let debug = format!("{spec:?}");
        assert!(debug.contains("PROVIDER_API_KEY"));
        assert!(!debug.contains("top-secret"));
    }

    #[test]
    fn legacy_metadata_defaults_to_unconfined_report() {
        let metadata = SupervisionMetadata {
            supervision_id: Uuid::nil(),
            pid: 1,
            process_group: ProcessGroupIdentity::UnixProcessGroup { pgid: 1 },
            process_start: ProcessStartIdentity::Platform {
                identity: "legacy".to_owned(),
            },
            executable: ExecutableIdentity {
                path: PathBuf::from("/legacy"),
                device: 0,
                inode: 0,
            },
            command_digest: "command".to_owned(),
            environment_digest: "environment".to_owned(),
            confinement: ProcessConfinementReport::default(),
            supervision_token: "token".to_owned(),
            state: SupervisionState::Terminated,
            termination_reason: None,
        };
        let mut serialized = serde_json::to_value(metadata).unwrap();
        serialized.as_object_mut().unwrap().remove("confinement");

        let restored: SupervisionMetadata = serde_json::from_value(serialized).unwrap();

        assert_eq!(restored.confinement, ProcessConfinementReport::default());
    }
}
