use agents::{AgentProvider, ClaudeCodeProvider, MockProvider, ProposedFileChange, ProviderError};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use domain::{ChangesetState, RelativePath, RunState, WorktreeState, limits};
use execution::{
    CancellationToken, ProcessResult, ProcessSpec, ProcessStartIdentity, ProcessSupervisor,
    SupervisionState, TerminationReason,
};
use git::GitService;
use orchestration::{CommandOutcome, LocalOrchestrator, OrchestrationError};
use persistence::{ArtifactPolicy, ArtifactState, ArtifactStream, LocalArtifactStore, SqliteStore};
use protocol::{LocalCommand, SemanticEventKind};
use std::{
    collections::HashMap,
    fs,
    path::Path,
    process::Command,
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};
use tempfile::tempdir;

fn run_git(repository: &Path, arguments: &[&str]) {
    let output = Command::new("git")
        .current_dir(repository)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture_repository(root: &Path) -> std::path::PathBuf {
    let repository = root.join("repo");
    fs::create_dir(&repository).unwrap();
    run_git(&repository, &["init", "-q"]);
    run_git(&repository, &["config", "user.email", "test@example.com"]);
    run_git(&repository, &["config", "user.name", "Test"]);
    fs::write(repository.join("README.md"), "fixture\n").unwrap();
    run_git(&repository, &["add", "."]);
    run_git(&repository, &["commit", "-qm", "fixture"]);
    repository
}

fn build_runtime(root: &Path) -> LocalOrchestrator<MockProvider> {
    let store = SqliteStore::open(root.join("state.sqlite3")).unwrap();
    let git = GitService::new(vec![root.to_path_buf()], root.join("worktrees")).unwrap();
    LocalOrchestrator::new(
        store,
        git,
        MockProvider::deterministic(),
        Duration::from_secs(60),
    )
}

struct NoProposalProvider;

impl AgentProvider for NoProposalProvider {
    fn name(&self) -> &'static str {
        "no-proposal"
    }
    fn propose(&self, _: Option<&ProcessResult>) -> Result<ProposedFileChange, ProviderError> {
        panic!("approval resume must use the persisted proposal")
    }
    fn normalized_events(&self, _: &ProposedFileChange, _: &str) -> Vec<SemanticEventKind> {
        panic!("approval resume must not normalize provider events")
    }
}

struct MissingProposalEventProvider;

impl AgentProvider for MissingProposalEventProvider {
    fn name(&self) -> &'static str {
        "missing-proposal-event"
    }
    fn propose(
        &self,
        process_result: Option<&ProcessResult>,
    ) -> Result<ProposedFileChange, ProviderError> {
        MockProvider::deterministic().propose(process_result)
    }
    fn normalized_events(&self, _: &ProposedFileChange, _: &str) -> Vec<SemanticEventKind> {
        vec![SemanticEventKind::Text {
            text: "provider stopped before approval".to_owned(),
        }]
    }
}

struct OversizedProposalProvider;

impl AgentProvider for OversizedProposalProvider {
    fn name(&self) -> &'static str {
        "oversized-proposal"
    }
    fn propose(
        &self,
        process_result: Option<&ProcessResult>,
    ) -> Result<ProposedFileChange, ProviderError> {
        let mut change = MockProvider::deterministic().propose(process_result)?;
        change
            .content
            .resize(limits::MAX_APPROVED_FILE_BYTES + 1, b'x');
        change.proposal.content_sha256 = git::content_digest(&change.content);
        Ok(change)
    }
    fn normalized_events(&self, _: &ProposedFileChange, _: &str) -> Vec<SemanticEventKind> {
        panic!("oversized proposals must be rejected before event normalization")
    }
}

struct LongRunningProcessProvider;

impl AgentProvider for LongRunningProcessProvider {
    fn name(&self) -> &'static str {
        "long-running-process"
    }

    fn process_spec(&self, worktree_path: &Path) -> Option<ProcessSpec> {
        Some(ProcessSpec {
            program: "/bin/sh".to_owned(),
            arguments: vec![
                "-c".to_owned(),
                "/bin/sleep 30 & echo $! > child.pid; wait".to_owned(),
            ],
            environment: HashMap::new(),
            sensitive_environment_keys: Vec::new(),
            current_dir: Some(worktree_path.to_path_buf()),
            timeout: Duration::from_secs(60),
            output_limit: 1024,
        })
    }

    fn propose(
        &self,
        process_result: Option<&ProcessResult>,
    ) -> Result<ProposedFileChange, ProviderError> {
        MockProvider::deterministic().propose(process_result)
    }

    fn normalized_events(
        &self,
        change: &ProposedFileChange,
        digest: &str,
    ) -> Vec<SemanticEventKind> {
        MockProvider::deterministic().normalized_events(change, digest)
    }
}

struct ShortProcessProvider;

impl AgentProvider for ShortProcessProvider {
    fn name(&self) -> &'static str {
        "short-process"
    }

    fn process_spec(&self, _: &Path) -> Option<ProcessSpec> {
        Some(ProcessSpec {
            program: "/bin/sh".to_owned(),
            arguments: vec!["-c".to_owned(), "printf provider-complete".to_owned()],
            environment: HashMap::new(),
            sensitive_environment_keys: Vec::new(),
            current_dir: None,
            timeout: Duration::from_secs(5),
            output_limit: 1024,
        })
    }

    fn propose(
        &self,
        process_result: Option<&ProcessResult>,
    ) -> Result<ProposedFileChange, ProviderError> {
        MockProvider::deterministic().propose(process_result)
    }

    fn normalized_events(
        &self,
        change: &ProposedFileChange,
        digest: &str,
    ) -> Vec<SemanticEventKind> {
        MockProvider::deterministic().normalized_events(change, digest)
    }
}

#[derive(Clone, Copy)]
enum ArtifactFailureMode {
    ProcessFailure,
    OutputTruncation,
    ParserFailure,
}

impl ArtifactFailureMode {
    fn command(self) -> &'static str {
        match self {
            Self::ProcessFailure => "printf process-failed; exit 7",
            Self::OutputTruncation => "printf 0123456789",
            Self::ParserFailure => "printf parser-failed",
        }
    }

    fn output_limit(self) -> usize {
        match self {
            Self::OutputTruncation => 4,
            Self::ProcessFailure | Self::ParserFailure => 1024,
        }
    }

    fn expected_output(self) -> &'static [u8] {
        match self {
            Self::ProcessFailure => b"process-failed",
            Self::OutputTruncation => b"0123",
            Self::ParserFailure => b"parser-failed",
        }
    }

    fn assert_error(self, error: OrchestrationError) {
        match (self, error) {
            (
                Self::ProcessFailure,
                OrchestrationError::ProviderProcessFailed(execution::TerminalOutcome::Failed),
            )
            | (Self::OutputTruncation, OrchestrationError::ProviderProcessOutputTruncated)
            | (Self::ParserFailure, OrchestrationError::Provider(ProviderError::InvalidResponse)) =>
                {}
            (_, unexpected) => panic!("unexpected orchestration error: {unexpected:?}"),
        }
    }
}

struct ArtifactFailureProvider {
    mode: ArtifactFailureMode,
}

impl AgentProvider for ArtifactFailureProvider {
    fn name(&self) -> &'static str {
        "artifact-failure"
    }

    fn process_spec(&self, _: &Path) -> Option<ProcessSpec> {
        Some(ProcessSpec {
            program: "/bin/sh".to_owned(),
            arguments: vec!["-c".to_owned(), self.mode.command().to_owned()],
            environment: HashMap::new(),
            sensitive_environment_keys: Vec::new(),
            current_dir: None,
            timeout: Duration::from_secs(5),
            output_limit: self.mode.output_limit(),
        })
    }

    fn propose(
        &self,
        process_result: Option<&ProcessResult>,
    ) -> Result<ProposedFileChange, ProviderError> {
        if matches!(self.mode, ArtifactFailureMode::ParserFailure) {
            return Err(ProviderError::InvalidResponse);
        }
        MockProvider::deterministic().propose(process_result)
    }

    fn normalized_events(
        &self,
        change: &ProposedFileChange,
        digest: &str,
    ) -> Vec<SemanticEventKind> {
        MockProvider::deterministic().normalized_events(change, digest)
    }
}

struct SensitiveOutputProvider;

impl AgentProvider for SensitiveOutputProvider {
    fn name(&self) -> &'static str {
        "sensitive-output"
    }

    fn process_spec(&self, _: &Path) -> Option<ProcessSpec> {
        Some(ProcessSpec {
            program: "/bin/sh".to_owned(),
            arguments: vec![
                "-c".to_owned(),
                "printf '%s:%s' \"$JET_BLACK_TEST_SECRET\" \"$JET_BLACK_SUPERVISION_TOKEN\""
                    .to_owned(),
            ],
            environment: HashMap::from([(
                "JET_BLACK_TEST_SECRET".to_owned(),
                "provider-secret".to_owned(),
            )]),
            sensitive_environment_keys: vec!["JET_BLACK_TEST_SECRET".to_owned()],
            current_dir: None,
            timeout: Duration::from_secs(5),
            output_limit: 1024,
        })
    }

    fn propose(
        &self,
        process_result: Option<&ProcessResult>,
    ) -> Result<ProposedFileChange, ProviderError> {
        MockProvider::deterministic().propose(process_result)
    }

    fn normalized_events(
        &self,
        change: &ProposedFileChange,
        digest: &str,
    ) -> Vec<SemanticEventKind> {
        MockProvider::deterministic().normalized_events(change, digest)
    }
}

#[test]
fn command_boundary_completes_and_recovers_a_digest_approved_run() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let runtime = build_runtime(directory.path());

    let repository = match runtime
        .handle(LocalCommand::RegisterRepository {
            path: repository_path,
        })
        .unwrap()
    {
        CommandOutcome::RepositoryRegistered(repository) => repository,
        outcome => panic!("unexpected outcome: {outcome:?}"),
    };
    assert!(matches!(
        runtime.handle(LocalCommand::CreateChangeset {
            repository_id: repository.id,
            base_sha: "stale".to_owned(),
            ticket: None,
        }),
        Err(OrchestrationError::StaleBase)
    ));

    let changeset = match runtime
        .handle(LocalCommand::CreateChangeset {
            repository_id: repository.id,
            base_sha: repository.base_sha.clone(),
            ticket: None,
        })
        .unwrap()
    {
        CommandOutcome::ChangesetCreated(changeset) => changeset,
        outcome => panic!("unexpected outcome: {outcome:?}"),
    };
    let started = match runtime
        .handle(LocalCommand::StartRun {
            changeset_id: changeset.id,
        })
        .unwrap()
    {
        CommandOutcome::RunStarted(started) => started,
        outcome => panic!("unexpected outcome: {outcome:?}"),
    };

    let mut tampered_scope = started.approval_request.scope.clone();
    tampered_scope.proposal.target_path = RelativePath::parse("tampered.txt").unwrap();
    assert!(matches!(
        runtime.handle(LocalCommand::RespondToApproval {
            run_id: started.run_id,
            scope: tampered_scope,
            approved: true,
        }),
        Err(OrchestrationError::ApprovalMismatch)
    ));

    let completed = match runtime
        .handle(LocalCommand::RespondToApproval {
            run_id: started.run_id,
            scope: started.approval_request.scope,
            approved: true,
        })
        .unwrap()
    {
        CommandOutcome::RunCompleted(completed) => completed,
        outcome => panic!("unexpected outcome: {outcome:?}"),
    };
    assert_eq!(completed.run.state(), RunState::Completed);
    assert!(completed.checkpoint.diff.contains("approved local change"));

    let events = runtime.events(started.run_id).unwrap();
    assert_eq!(
        events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        (1..=9).collect::<Vec<_>>()
    );

    drop(runtime);
    let restarted = build_runtime(directory.path());
    let recovery = restarted.recover().unwrap();
    assert!(recovery.terminal.contains(&started.run_id));

    let checkpoint = restarted
        .handle(LocalCommand::GetCheckpoint {
            run_id: started.run_id,
        })
        .unwrap();
    assert!(matches!(checkpoint, CommandOutcome::Checkpoint(_)));
    let diff = restarted
        .handle(LocalCommand::GetDiff {
            changeset_id: changeset.id,
        })
        .unwrap();
    assert!(matches!(diff, CommandOutcome::Diff(_)));

    let worktree_path = directory
        .path()
        .join("worktrees")
        .join(changeset.id.to_string());
    let removed = restarted.cleanup_changeset(changeset.id).unwrap();
    assert_eq!(removed.state(), WorktreeState::Removed);
    assert!(!worktree_path.exists());
    assert_eq!(
        restarted.cleanup_changeset(changeset.id).unwrap().state(),
        WorktreeState::Removed
    );
}

#[test]
fn rejected_approval_interrupts_without_mutating_and_can_be_cleaned_up() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let runtime = build_runtime(directory.path());
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let started = runtime.start_run(changeset.id).unwrap();

    let rejected = runtime
        .reject_approval(started.run_id, &started.approval_request.scope)
        .unwrap();
    assert_eq!(rejected.state(), RunState::Interrupted);
    assert!(
        !directory
            .path()
            .join("worktrees")
            .join(changeset.id.to_string())
            .join("jet-black-approved.txt")
            .exists()
    );

    let removed = runtime.cleanup_changeset(changeset.id).unwrap();
    assert_eq!(removed.state(), WorktreeState::Removed);
}

#[test]
fn approval_resume_uses_the_persisted_provider_change() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let runtime = build_runtime(directory.path());
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let started = runtime.start_run(changeset.id).unwrap();
    drop(runtime);

    let restarted = LocalOrchestrator::new(
        SqliteStore::open(directory.path().join("state.sqlite3")).unwrap(),
        GitService::new(
            vec![directory.path().to_path_buf()],
            directory.path().join("worktrees"),
        )
        .unwrap(),
        NoProposalProvider,
        Duration::from_secs(60),
    );
    let completed = restarted
        .approve_and_complete(started.run_id, &started.approval_request.scope)
        .unwrap();
    assert_eq!(completed.run.state(), RunState::Completed);
    assert!(completed.checkpoint.diff.contains("approved local change"));
}

#[test]
fn provider_contract_failure_recovers_changeset_and_cleans_worktree() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let store_path = directory.path().join("state.sqlite3");
    let runtime = LocalOrchestrator::new(
        SqliteStore::open(&store_path).unwrap(),
        GitService::new(
            vec![directory.path().to_path_buf()],
            directory.path().join("worktrees"),
        )
        .unwrap(),
        MissingProposalEventProvider,
        Duration::from_secs(60),
    );
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();

    assert!(matches!(
        runtime.start_run(changeset.id),
        Err(OrchestrationError::ProviderContract(_))
    ));
    let store = SqliteStore::open(&store_path).unwrap();
    assert_eq!(
        store.changeset(changeset.id).unwrap().unwrap().state(),
        ChangesetState::Recoverable
    );
    assert_eq!(
        store
            .worktree_for_changeset(changeset.id)
            .unwrap()
            .unwrap()
            .state(),
        WorktreeState::Removed
    );
    assert!(
        !directory
            .path()
            .join("worktrees")
            .join(changeset.id.to_string())
            .exists()
    );
    assert!(matches!(
        runtime.start_run(changeset.id),
        Err(OrchestrationError::ProviderContract(_))
    ));
}

#[test]
fn oversized_provider_change_is_rejected_and_compensated() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let store_path = directory.path().join("state.sqlite3");
    let runtime = LocalOrchestrator::new(
        SqliteStore::open(&store_path).unwrap(),
        GitService::new(
            vec![directory.path().to_path_buf()],
            directory.path().join("worktrees"),
        )
        .unwrap(),
        OversizedProposalProvider,
        Duration::from_secs(60),
    );
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();

    assert!(matches!(
        runtime.start_run(changeset.id),
        Err(OrchestrationError::ResourceLimit("approved file content"))
    ));
    let store = SqliteStore::open(&store_path).unwrap();
    assert_eq!(
        store.changeset(changeset.id).unwrap().unwrap().state(),
        ChangesetState::Recoverable
    );
    assert_eq!(
        store
            .worktree_for_changeset(changeset.id)
            .unwrap()
            .unwrap()
            .state(),
        WorktreeState::Removed
    );
}

#[test]
fn repeated_interrupt_does_not_append_an_event() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let runtime = build_runtime(directory.path());
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let started = runtime.start_run(changeset.id).unwrap();

    let interrupted = runtime.interrupt_run(started.run_id).unwrap();
    assert_eq!(interrupted.state(), RunState::Interrupted);
    let event_count = runtime.events(started.run_id).unwrap().len();

    let interrupted_again = runtime.interrupt_run(started.run_id).unwrap();
    assert_eq!(interrupted_again.state(), RunState::Interrupted);
    assert_eq!(runtime.events(started.run_id).unwrap().len(), event_count);

    runtime.cleanup_changeset(changeset.id).unwrap();
}

#[test]
fn interrupt_terminates_supervised_process_tree_and_compensates_run() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let store_path = directory.path().join("state.sqlite3");
    let store = SqliteStore::open(&store_path).unwrap();
    let runtime = Arc::new(LocalOrchestrator::with_process_supervision(
        store.clone(),
        GitService::new(
            vec![directory.path().to_path_buf()],
            directory.path().join("worktrees"),
        )
        .unwrap(),
        LongRunningProcessProvider,
        Duration::from_secs(60),
        Duration::from_secs(120),
        execution::ProcessSupervisor::new(),
    ));
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let start_runtime = Arc::clone(&runtime);
    let start_thread = thread::spawn(move || start_runtime.start_run(changeset.id));

    let deadline = Instant::now() + Duration::from_secs(5);
    let running_record = loop {
        let active = store.nonterminated_process_supervisions().unwrap();
        if let Some(record) = active
            .into_iter()
            .find(|record| record.metadata.state == SupervisionState::Running)
        {
            break record;
        }
        assert!(Instant::now() < deadline, "provider process did not start");
        thread::sleep(Duration::from_millis(10));
    };
    let child_pid_path = directory
        .path()
        .join("worktrees")
        .join(changeset.id.to_string())
        .join("child.pid");
    let child_pid = loop {
        if let Ok(contents) = fs::read_to_string(&child_pid_path)
            && let Ok(pid) = contents.trim().parse::<u32>()
        {
            break pid;
        }
        assert!(
            Instant::now() < deadline,
            "provider child pid was not recorded"
        );
        thread::sleep(Duration::from_millis(10));
    };

    let interrupted = runtime.interrupt_run(running_record.run_id).unwrap();
    assert_eq!(interrupted.state(), RunState::Interrupted);
    assert!(matches!(
        start_thread.join().unwrap(),
        Err(OrchestrationError::RunInterruptedDuringExecution)
    ));

    let terminated = store
        .latest_process_supervision_for_run(running_record.run_id)
        .unwrap()
        .unwrap();
    assert_eq!(terminated.metadata.state, SupervisionState::Terminated);
    assert_eq!(
        terminated.metadata.termination_reason,
        Some(TerminationReason::Requested)
    );
    assert!(!Path::new(&format!("/proc/{}", terminated.metadata.pid)).exists());
    assert!(!Path::new(&format!("/proc/{child_pid}")).exists());
    assert_eq!(
        store.run(running_record.run_id).unwrap().unwrap().state(),
        RunState::Interrupted
    );
    assert_eq!(
        store.changeset(changeset.id).unwrap().unwrap().state(),
        ChangesetState::Recoverable
    );
    assert!(store.mutation_lease(changeset.id).unwrap().is_none());
    assert!(
        store
            .nonterminated_process_supervisions()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        store
            .worktree_for_changeset(changeset.id)
            .unwrap()
            .unwrap()
            .state(),
        WorktreeState::Removed
    );
}

#[test]
fn successful_provider_process_holds_lease_for_exact_approval_then_releases_it() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let store_path = directory.path().join("state.sqlite3");
    let store = SqliteStore::open(&store_path).unwrap();
    let runtime = LocalOrchestrator::with_process_supervision(
        store.clone(),
        GitService::new(
            vec![directory.path().to_path_buf()],
            directory.path().join("worktrees"),
        )
        .unwrap(),
        ShortProcessProvider,
        Duration::from_secs(60),
        Duration::from_secs(120),
        execution::ProcessSupervisor::new(),
    );
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let started = runtime.start_run(changeset.id).unwrap();

    let terminated = store
        .latest_process_supervision_for_run(started.run_id)
        .unwrap()
        .unwrap();
    assert_eq!(terminated.metadata.state, SupervisionState::Terminated);
    assert_eq!(
        terminated.metadata.termination_reason,
        Some(TerminationReason::Exited { code: 0 })
    );
    let lease = store.mutation_lease(changeset.id).unwrap().unwrap();
    assert_eq!(lease.run_id, started.run_id);
    assert_eq!(
        store.run(started.run_id).unwrap().unwrap().state(),
        RunState::AwaitingApproval
    );

    let mut tampered = started.approval_request.scope.clone();
    tampered.proposal.target_path = RelativePath::parse("not-approved.txt").unwrap();
    assert!(matches!(
        runtime.approve_and_complete(started.run_id, &tampered),
        Err(OrchestrationError::ApprovalMismatch)
    ));
    assert_eq!(store.mutation_lease(changeset.id).unwrap(), Some(lease));

    let completed = runtime
        .approve_and_complete(started.run_id, &started.approval_request.scope)
        .unwrap();
    assert_eq!(completed.run.state(), RunState::Completed);
    assert!(store.mutation_lease(changeset.id).unwrap().is_none());

    let rejected_changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let rejected_started = runtime.start_run(rejected_changeset.id).unwrap();
    assert!(
        store
            .mutation_lease(rejected_changeset.id)
            .unwrap()
            .is_some()
    );
    let rejected = runtime
        .reject_approval(
            rejected_started.run_id,
            &rejected_started.approval_request.scope,
        )
        .unwrap();
    assert_eq!(rejected.state(), RunState::Interrupted);
    assert!(
        store
            .mutation_lease(rejected_changeset.id)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        store
            .changeset(rejected_changeset.id)
            .unwrap()
            .unwrap()
            .state(),
        ChangesetState::Recoverable
    );
}

#[test]
fn provider_artifacts_redact_credentials_and_supervision_identity() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let store = SqliteStore::open(directory.path().join("state.sqlite3")).unwrap();
    let artifact_root = directory.path().join("artifacts");
    let artifact_store = LocalArtifactStore::new(
        store.clone(),
        &artifact_root,
        ArtifactPolicy {
            segment_bytes: 8,
            per_run_bytes: 1024,
            total_bytes: 1024,
            retention: Duration::from_secs(60),
        },
    )
    .unwrap();
    let runtime = LocalOrchestrator::with_process_supervision(
        store.clone(),
        GitService::new(
            vec![directory.path().to_path_buf()],
            directory.path().join("worktrees"),
        )
        .unwrap(),
        SensitiveOutputProvider,
        Duration::from_secs(60),
        Duration::from_secs(120),
        execution::ProcessSupervisor::new(),
    )
    .with_artifact_store(artifact_store.clone());
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let started = runtime.start_run(changeset.id).unwrap();
    let supervision = store
        .latest_process_supervision_for_run(started.run_id)
        .unwrap()
        .unwrap();
    let artifacts = artifact_store.artifacts_for_run(started.run_id).unwrap();

    assert_eq!(artifacts.len(), 1);
    let artifact = &artifacts[0];
    assert_eq!(artifact.changeset_id, changeset.id);
    assert_eq!(artifact.run_id, started.run_id);
    assert_eq!(artifact.supervision_id, supervision.metadata.supervision_id);
    assert_eq!(artifact.stream, ArtifactStream::Stdout);
    let mut persisted = Vec::new();
    for segment in &artifact.segments {
        persisted.extend(
            fs::read(
                artifact_root
                    .join(artifact.id.to_string())
                    .join(format!("{:08}.segment", segment.sequence)),
            )
            .unwrap(),
        );
    }
    assert_eq!(persisted, b"[REDACTED]:[REDACTED]");
    assert!(
        !persisted
            .windows(b"provider-secret".len())
            .any(|bytes| { bytes == b"provider-secret" })
    );
    assert!(
        !persisted
            .windows(supervision.metadata.supervision_token.len())
            .any(|bytes| bytes == supervision.metadata.supervision_token.as_bytes())
    );

    let events = runtime.events(started.run_id).unwrap();
    let summaries = match runtime
        .handle(LocalCommand::GetRunArtifacts {
            run_id: started.run_id,
        })
        .unwrap()
    {
        CommandOutcome::RunArtifacts(response) => response.artifacts,
        outcome => panic!("unexpected outcome: {outcome:?}"),
    };
    assert_eq!(summaries.len(), 1);
    let summary = &summaries[0];
    assert_eq!(summary.artifact_id, artifact.id);
    assert_eq!(summary.stored_bytes, artifact.stored_bytes as u64);

    let mut downloaded = Vec::new();
    for segment in &summary.segments {
        let response = match runtime
            .handle(LocalCommand::ReadRunArtifactSegment {
                run_id: started.run_id,
                artifact_id: summary.artifact_id,
                segment_sequence: segment.sequence,
            })
            .unwrap()
        {
            CommandOutcome::RunArtifactSegment(response) => response,
            outcome => panic!("unexpected outcome: {outcome:?}"),
        };
        assert_eq!(response.segment, *segment);
        downloaded.extend(STANDARD.decode(response.content_base64).unwrap());
    }
    assert_eq!(downloaded, persisted);

    let corrupt_segment = artifact_root
        .join(artifact.id.to_string())
        .join("00000001.segment");
    fs::write(&corrupt_segment, b"corrupt!").unwrap();
    assert!(matches!(
        runtime.handle(LocalCommand::ReadRunArtifactSegment {
            run_id: started.run_id,
            artifact_id: artifact.id,
            segment_sequence: 0,
        }),
        Err(OrchestrationError::ArtifactIntegrity)
    ));

    let other_changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let other_started = runtime.start_run(other_changeset.id).unwrap();
    assert!(matches!(
        runtime.handle(LocalCommand::ReadRunArtifactSegment {
            run_id: other_started.run_id,
            artifact_id: artifact.id,
            segment_sequence: 0,
        }),
        Err(OrchestrationError::NotFound("artifact"))
    ));
    runtime
        .reject_approval(other_started.run_id, &other_started.approval_request.scope)
        .unwrap();
    runtime.delete_run_artifacts(other_started.run_id).unwrap();

    assert!(matches!(
        runtime.handle(LocalCommand::DeleteRunArtifacts {
            run_id: started.run_id,
        }),
        Err(OrchestrationError::ArtifactDeletionRequiresTerminalRun)
    ));
    assert_eq!(runtime.events(started.run_id).unwrap(), events);

    runtime
        .reject_approval(started.run_id, &started.approval_request.scope)
        .unwrap();
    let deleted = match runtime
        .handle(LocalCommand::DeleteRunArtifacts {
            run_id: started.run_id,
        })
        .unwrap()
    {
        CommandOutcome::RunArtifactsDeleted(response) => response.deleted_count,
        outcome => panic!("unexpected outcome: {outcome:?}"),
    };
    assert_eq!(deleted, 1);
    let deleted_again = match runtime
        .handle(LocalCommand::DeleteRunArtifacts {
            run_id: started.run_id,
        })
        .unwrap()
    {
        CommandOutcome::RunArtifactsDeleted(response) => response.deleted_count,
        outcome => panic!("unexpected outcome: {outcome:?}"),
    };
    assert_eq!(deleted_again, 0);
    assert!(
        artifact_store
            .artifacts_for_run(started.run_id)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn artifact_commands_require_a_configured_store() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let runtime = build_runtime(directory.path());
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let started = runtime.start_run(changeset.id).unwrap();

    assert!(matches!(
        runtime.handle(LocalCommand::GetRunArtifacts {
            run_id: started.run_id,
        }),
        Err(OrchestrationError::ArtifactStoreUnavailable)
    ));

    runtime
        .reject_approval(started.run_id, &started.approval_request.scope)
        .unwrap();
}

#[test]
fn provider_artifacts_survive_process_and_parser_failures() {
    for mode in [
        ArtifactFailureMode::ProcessFailure,
        ArtifactFailureMode::OutputTruncation,
        ArtifactFailureMode::ParserFailure,
    ] {
        let directory = tempdir().unwrap();
        let repository_path = fixture_repository(directory.path());
        let store = SqliteStore::open(directory.path().join("state.sqlite3")).unwrap();
        let artifact_root = directory.path().join("artifacts");
        let artifact_store = LocalArtifactStore::new(
            store.clone(),
            &artifact_root,
            ArtifactPolicy {
                segment_bytes: 4,
                per_run_bytes: 1024,
                total_bytes: 1024,
                retention: Duration::from_secs(60),
            },
        )
        .unwrap();
        let runtime = LocalOrchestrator::with_process_supervision(
            store.clone(),
            GitService::new(
                vec![directory.path().to_path_buf()],
                directory.path().join("worktrees"),
            )
            .unwrap(),
            ArtifactFailureProvider { mode },
            Duration::from_secs(60),
            Duration::from_secs(120),
            execution::ProcessSupervisor::new(),
        )
        .with_artifact_store(artifact_store.clone());
        let repository = runtime.register_repository(&repository_path).unwrap();
        let changeset = runtime
            .create_changeset(repository.id, &repository.base_sha, None)
            .unwrap();

        mode.assert_error(runtime.start_run(changeset.id).unwrap_err());

        let run = store.runs().unwrap().pop().unwrap();
        assert_eq!(run.state(), RunState::Failed);
        assert_eq!(
            store.changeset(changeset.id).unwrap().unwrap().state(),
            ChangesetState::Recoverable
        );
        let supervision = store
            .latest_process_supervision_for_run(run.id)
            .unwrap()
            .unwrap();
        if matches!(mode, ArtifactFailureMode::ProcessFailure) {
            assert_eq!(
                supervision.metadata.termination_reason,
                Some(TerminationReason::Exited { code: 7 })
            );
        }
        let artifacts = artifact_store.artifacts_for_run(run.id).unwrap();
        assert_eq!(artifacts.len(), 1);
        let artifact = &artifacts[0];
        assert_eq!(artifact.state, ArtifactState::Complete);
        assert_eq!(artifact.stream, ArtifactStream::Stdout);
        assert_eq!(artifact.changeset_id, changeset.id);
        assert_eq!(artifact.supervision_id, supervision.metadata.supervision_id);
        assert_eq!(
            artifact.process_truncated,
            matches!(mode, ArtifactFailureMode::OutputTruncation)
        );
        assert!(!artifact.quota_limited);

        let mut persisted = Vec::new();
        for segment in &artifact.segments {
            persisted.extend(
                fs::read(
                    artifact_root
                        .join(artifact.id.to_string())
                        .join(format!("{:08}.segment", segment.sequence)),
                )
                .unwrap(),
            );
        }
        assert_eq!(persisted, mode.expected_output());
    }
}

#[test]
fn startup_reconciliation_interrupts_clean_pending_run_and_is_idempotent() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let store_path = directory.path().join("state.sqlite3");
    let runtime = build_runtime(directory.path());
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let started = runtime.start_run(changeset.id).unwrap();
    drop(runtime);

    let restarted = build_runtime(directory.path());
    let first = restarted.recover().unwrap();
    assert_eq!(first.interrupted, vec![started.run_id]);
    assert_eq!(first.recoverable, vec![started.run_id]);
    assert_eq!(first.actions.len(), 3);
    assert_eq!(
        SqliteStore::open(&store_path)
            .unwrap()
            .run(started.run_id)
            .unwrap()
            .unwrap()
            .state(),
        RunState::Interrupted
    );
    assert_eq!(
        SqliteStore::open(&store_path)
            .unwrap()
            .changeset(changeset.id)
            .unwrap()
            .unwrap()
            .state(),
        ChangesetState::Recoverable
    );
    assert!(
        SqliteStore::open(&store_path)
            .unwrap()
            .mutation_lease(changeset.id)
            .unwrap()
            .is_none()
    );
    assert!(
        !directory
            .path()
            .join("worktrees")
            .join(changeset.id.to_string())
            .exists()
    );

    let second = restarted.recover().unwrap();
    assert_eq!(second.terminal, vec![started.run_id]);
    assert!(second.actions.is_empty());
    let response = restarted.handle(LocalCommand::GetRecovery).unwrap();
    assert!(matches!(
        response,
        CommandOutcome::Recovery(recovery) if recovery.actions == first.actions
    ));
}

#[test]
fn startup_reconciliation_removes_an_unpersisted_clean_worktree() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let store = SqliteStore::open(directory.path().join("state.sqlite3")).unwrap();
    let runtime = build_runtime(directory.path());
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let git = GitService::new(
        vec![directory.path().to_path_buf()],
        directory.path().join("worktrees"),
    )
    .unwrap();
    let worktree = git.create_worktree(&repository, changeset.id).unwrap();
    let worktree_path = worktree.path.clone();
    assert!(
        store
            .worktree_for_changeset(changeset.id)
            .unwrap()
            .is_none()
    );

    let recovery = runtime.recover().unwrap();

    assert!(!worktree_path.exists());
    assert_eq!(
        store.changeset(changeset.id).unwrap().unwrap().state(),
        ChangesetState::Created
    );
    assert!(
        recovery
            .actions
            .iter()
            .any(|action| { action.action == "orphan_worktree_removed" })
    );
}

#[test]
fn startup_reconciliation_quarantines_an_unpersisted_dirty_worktree() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let store = SqliteStore::open(directory.path().join("state.sqlite3")).unwrap();
    let runtime = build_runtime(directory.path());
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let git = GitService::new(
        vec![directory.path().to_path_buf()],
        directory.path().join("worktrees"),
    )
    .unwrap();
    let worktree = git.create_worktree(&repository, changeset.id).unwrap();
    fs::write(worktree.path.join("unpersisted.txt"), "dirty\n").unwrap();

    let recovery = runtime.recover().unwrap();

    assert!(worktree.path.exists());
    assert_eq!(
        store.changeset(changeset.id).unwrap().unwrap().state(),
        ChangesetState::Failed
    );
    assert_eq!(
        store
            .worktree_for_changeset(changeset.id)
            .unwrap()
            .unwrap()
            .state(),
        WorktreeState::Quarantined
    );
    assert!(
        recovery
            .actions
            .iter()
            .any(|action| { action.action == "orphan_worktree_quarantined" })
    );
}

#[test]
fn concurrent_startup_reconciliation_is_idempotent_across_runtime_instances() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let store_path = directory.path().join("state.sqlite3");
    let runtime = build_runtime(directory.path());
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let started = runtime.start_run(changeset.id).unwrap();
    drop(runtime);

    let barrier = Arc::new(Barrier::new(2));
    let root = directory.path().to_path_buf();
    let handles = (0..2)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let root = root.clone();
            thread::spawn(move || {
                let runtime = build_runtime(&root);
                barrier.wait();
                runtime.recover()
            })
        })
        .collect::<Vec<_>>();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();

    assert!(results.iter().all(Result::is_ok), "{results:?}");
    let store = SqliteStore::open(&store_path).unwrap();
    assert_eq!(
        store.run(started.run_id).unwrap().unwrap().state(),
        RunState::Interrupted
    );
    assert_eq!(
        store.changeset(changeset.id).unwrap().unwrap().state(),
        ChangesetState::Recoverable
    );
    assert!(store.mutation_lease(changeset.id).unwrap().is_none());
    assert_eq!(store.recovery_actions().unwrap().len(), 3);
}

#[test]
fn startup_reconciliation_releases_a_terminal_reviewable_changeset_lease() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let store = SqliteStore::open(directory.path().join("state.sqlite3")).unwrap();
    let runtime = build_runtime(directory.path());
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let started = runtime.start_run(changeset.id).unwrap();
    let completed = runtime
        .approve_and_complete(started.run_id, &started.approval_request.scope)
        .unwrap();
    let worktree_path = directory
        .path()
        .join("worktrees")
        .join(changeset.id.to_string());
    store
        .acquire_mutation_lease(changeset.id, started.run_id, 1, Duration::from_secs(60))
        .unwrap();

    let recovery = runtime.recover().unwrap();

    assert_eq!(recovery.terminal, vec![started.run_id]);
    assert_eq!(
        store.changeset(changeset.id).unwrap().unwrap().state(),
        ChangesetState::Reviewable
    );
    assert_eq!(completed.run.state(), RunState::Completed);
    assert!(worktree_path.exists());
    assert!(store.mutation_lease(changeset.id).unwrap().is_none());
    assert!(
        recovery
            .actions
            .iter()
            .any(|action| action.action == "lease_released")
    );
}

#[test]
fn startup_reconciliation_recovers_active_changeset_for_terminal_run() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let store_path = directory.path().join("state.sqlite3");
    let runtime = build_runtime(directory.path());
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let started = runtime.start_run(changeset.id).unwrap();
    drop(runtime);

    let store = SqliteStore::open(&store_path).unwrap();
    let mut run = store.run(started.run_id).unwrap().unwrap();
    run.interrupt().unwrap();
    store
        .persist_run_transition(
            &run,
            SemanticEventKind::Lifecycle {
                state: RunState::Interrupted,
            },
        )
        .unwrap();
    assert_eq!(
        store.changeset(changeset.id).unwrap().unwrap().state(),
        ChangesetState::Active
    );
    assert!(store.mutation_lease(changeset.id).unwrap().is_some());
    drop(store);

    let restarted = build_runtime(directory.path());
    let first = restarted.recover().unwrap();
    assert_eq!(first.terminal, vec![started.run_id]);
    assert_eq!(first.recoverable, vec![started.run_id]);

    let store = SqliteStore::open(&store_path).unwrap();
    assert_eq!(
        store.changeset(changeset.id).unwrap().unwrap().state(),
        ChangesetState::Recoverable
    );
    assert!(store.mutation_lease(changeset.id).unwrap().is_none());
    assert!(
        !directory
            .path()
            .join("worktrees")
            .join(changeset.id.to_string())
            .exists()
    );
    drop(store);

    let second = restarted.recover().unwrap();
    assert!(second.actions.is_empty());
    assert_eq!(
        SqliteStore::open(&store_path)
            .unwrap()
            .recovery_actions()
            .unwrap()
            .into_iter()
            .map(|record| record.action)
            .collect::<Vec<_>>(),
        first.actions
    );
}

#[test]
fn startup_reconciliation_terminates_a_surviving_persisted_process_tree() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let store_path = directory.path().join("state.sqlite3");
    let store = SqliteStore::open(&store_path).unwrap();
    let runtime = build_runtime(directory.path());
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let started = runtime.start_run(changeset.id).unwrap();
    let worktree_path = directory
        .path()
        .join("worktrees")
        .join(changeset.id.to_string());

    let supervisor = ProcessSupervisor::new();
    let spec = ProcessSpec {
        program: "/bin/sh".to_owned(),
        arguments: vec![
            "-c".to_owned(),
            "/bin/sleep 30 & echo $! > child.pid; wait".to_owned(),
        ],
        environment: HashMap::new(),
        sensitive_environment_keys: Vec::new(),
        current_dir: Some(worktree_path.clone()),
        timeout: Duration::from_secs(60),
        output_limit: 1024,
    };
    let (prepared, prepared_metadata) = supervisor.prepare(&spec).unwrap();
    store
        .insert_prepared_process_supervision(started.run_id, &prepared_metadata, 1)
        .unwrap();
    let running = prepared.release().unwrap();
    store
        .mark_process_supervision_running(started.run_id, running.metadata(), 2)
        .unwrap();
    let root_pid = running.metadata().pid;
    let wait_thread =
        thread::spawn(move || supervisor.wait(running, &CancellationToken::default()));

    let deadline = Instant::now() + Duration::from_secs(5);
    let child_pid = loop {
        if let Ok(contents) = fs::read_to_string(worktree_path.join("child.pid"))
            && let Ok(pid) = contents.trim().parse::<u32>()
        {
            break pid;
        }
        assert!(
            Instant::now() < deadline,
            "provider child pid was not recorded"
        );
        thread::sleep(Duration::from_millis(10));
    };
    drop(runtime);

    let restarted = build_runtime(directory.path());
    let recovery = restarted.recover().unwrap();
    wait_thread.join().unwrap().unwrap();
    assert!(!Path::new(&format!("/proc/{root_pid}")).exists());
    assert!(!Path::new(&format!("/proc/{child_pid}")).exists());
    assert_eq!(
        store
            .latest_process_supervision_for_run(started.run_id)
            .unwrap()
            .unwrap()
            .metadata
            .state,
        SupervisionState::Terminated
    );
    assert!(
        recovery
            .actions
            .iter()
            .any(|action| action.action == "process_terminated")
    );
}

#[test]
#[cfg(target_os = "linux")]
fn startup_reconciliation_terminates_a_prepared_process_before_release() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let store = SqliteStore::open(directory.path().join("state.sqlite3")).unwrap();
    let runtime = build_runtime(directory.path());
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let started = runtime.start_run(changeset.id).unwrap();
    let marker = directory.path().join("prepared-acted");
    let supervisor = ProcessSupervisor::new();
    let spec = ProcessSpec {
        program: "/bin/sh".to_owned(),
        arguments: vec![
            "-c".to_owned(),
            format!("echo acted > '{}'", marker.display()),
        ],
        environment: HashMap::new(),
        sensitive_environment_keys: Vec::new(),
        current_dir: None,
        timeout: Duration::from_secs(60),
        output_limit: 1024,
    };
    let (prepared, metadata) = supervisor.prepare(&spec).unwrap();
    store
        .insert_prepared_process_supervision(started.run_id, &metadata, 1)
        .unwrap();

    let recovery = runtime.recover().unwrap();

    assert!(!marker.exists());
    drop(prepared);
    assert!(!Path::new(&format!("/proc/{}", metadata.pid)).exists());
    assert_eq!(
        store
            .latest_process_supervision_for_run(started.run_id)
            .unwrap()
            .unwrap()
            .metadata
            .state,
        SupervisionState::Terminated
    );
    assert!(
        recovery
            .actions
            .iter()
            .any(|action| action.action == "process_terminated")
    );
}

#[test]
#[cfg(target_os = "linux")]
fn startup_reconciliation_terminalizes_an_already_exited_process_record() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let store = SqliteStore::open(directory.path().join("state.sqlite3")).unwrap();
    let runtime = build_runtime(directory.path());
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let started = runtime.start_run(changeset.id).unwrap();
    let supervisor = ProcessSupervisor::new();
    let spec = ProcessSpec {
        program: "/bin/sh".to_owned(),
        arguments: vec!["-c".to_owned(), "exit 0".to_owned()],
        environment: HashMap::new(),
        sensitive_environment_keys: Vec::new(),
        current_dir: None,
        timeout: Duration::from_secs(5),
        output_limit: 1024,
    };
    let (prepared, prepared_metadata) = supervisor.prepare(&spec).unwrap();
    store
        .insert_prepared_process_supervision(started.run_id, &prepared_metadata, 1)
        .unwrap();
    let running = prepared.release().unwrap();
    store
        .mark_process_supervision_running(started.run_id, running.metadata(), 2)
        .unwrap();
    let pid = running.metadata().pid;
    supervisor
        .wait(running, &CancellationToken::default())
        .unwrap();
    assert!(!Path::new(&format!("/proc/{pid}")).exists());

    let recovery = runtime.recover().unwrap();

    assert!(recovery.actions.iter().any(|action| {
        action.action == "process_terminated" && action.detail.contains("already exited")
    }));
    assert_eq!(
        store
            .latest_process_supervision_for_run(started.run_id)
            .unwrap()
            .unwrap()
            .metadata
            .termination_reason,
        Some(TerminationReason::Requested)
    );
}

#[test]
#[cfg(target_os = "linux")]
fn startup_reconciliation_does_not_signal_a_pid_with_mismatched_identity() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let store = SqliteStore::open(directory.path().join("state.sqlite3")).unwrap();
    let runtime = build_runtime(directory.path());
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let started = runtime.start_run(changeset.id).unwrap();
    let supervisor = ProcessSupervisor::new();
    let spec = ProcessSpec {
        program: "/bin/sh".to_owned(),
        arguments: vec!["-c".to_owned(), "sleep 30".to_owned()],
        environment: HashMap::new(),
        sensitive_environment_keys: Vec::new(),
        current_dir: None,
        timeout: Duration::from_secs(60),
        output_limit: 1024,
    };
    let (prepared, mut metadata) = supervisor.prepare(&spec).unwrap();
    let pid = metadata.pid;
    metadata.process_start = ProcessStartIdentity::Platform {
        identity: "stale-start-identity".to_owned(),
    };
    store
        .insert_prepared_process_supervision(started.run_id, &metadata, 1)
        .unwrap();

    let recovery = runtime.recover().unwrap();

    assert!(Path::new(&format!("/proc/{pid}")).exists());
    assert!(recovery.actions.iter().any(|action| {
        action.action == "process_terminated" && action.detail.contains("no signal was sent")
    }));
    assert_eq!(
        store
            .latest_process_supervision_for_run(started.run_id)
            .unwrap()
            .unwrap()
            .metadata
            .termination_reason,
        Some(TerminationReason::IdentityMismatch)
    );
    assert!(store.mutation_lease(changeset.id).unwrap().is_none());
    drop(prepared);
    assert!(!Path::new(&format!("/proc/{pid}")).exists());
}

#[test]
fn startup_reconciliation_quarantines_repository_head_movement() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let store = SqliteStore::open(directory.path().join("state.sqlite3")).unwrap();
    let runtime = build_runtime(directory.path());
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let started = runtime.start_run(changeset.id).unwrap();
    fs::write(repository_path.join("README.md"), "advanced\n").unwrap();
    run_git(&repository_path, &["add", "README.md"]);
    run_git(&repository_path, &["commit", "-qm", "advance"]);

    let recovery = runtime.recover().unwrap();

    assert_eq!(recovery.interrupted, vec![started.run_id]);
    assert_eq!(
        store.changeset(changeset.id).unwrap().unwrap().state(),
        ChangesetState::Divergent
    );
    assert_eq!(
        store
            .worktree_for_changeset(changeset.id)
            .unwrap()
            .unwrap()
            .state(),
        WorktreeState::Quarantined
    );
    assert!(store.mutation_lease(changeset.id).unwrap().is_none());
    assert!(recovery.actions.iter().any(|action| {
        action.action == "quarantined" && action.detail == "registered repository HEAD changed"
    }));
}

#[test]
fn startup_reconciliation_quarantines_unapproved_worktree_changes() {
    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let store_path = directory.path().join("state.sqlite3");
    let runtime = build_runtime(directory.path());
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();
    let started = runtime.start_run(changeset.id).unwrap();
    let worktree_path = directory
        .path()
        .join("worktrees")
        .join(changeset.id.to_string());
    fs::write(worktree_path.join("unapproved.txt"), "unapproved\n").unwrap();
    drop(runtime);

    let restarted = build_runtime(directory.path());
    let recovery = restarted.recover().unwrap();
    assert_eq!(recovery.interrupted, vec![started.run_id]);
    assert!(recovery.recoverable.is_empty());
    let store = SqliteStore::open(&store_path).unwrap();
    assert_eq!(
        store.changeset(changeset.id).unwrap().unwrap().state(),
        ChangesetState::Divergent
    );
    assert_eq!(
        store
            .worktree_for_changeset(changeset.id)
            .unwrap()
            .unwrap()
            .state(),
        WorktreeState::Quarantined
    );
    assert!(worktree_path.exists());
    assert!(store.mutation_lease(changeset.id).unwrap().is_none());
    assert!(
        recovery
            .actions
            .iter()
            .any(|action| action.action == "quarantined")
    );
}

#[cfg(unix)]
#[test]
fn fake_claude_provider_completes_the_approved_orchestration_flow() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempdir().unwrap();
    let repository_path = fixture_repository(directory.path());
    let executable = directory.path().join("claude");
    fs::write(
        &executable,
        r#"#!/bin/sh
printf '%s' '{"is_error":false,"structured_output":{"content":"generated read-only\n"}}'
"#,
    )
    .unwrap();
    let mut permissions = fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&executable, permissions).unwrap();
    let provider = ClaudeCodeProvider::discover(
        &directory.path().to_string_lossy(),
        Some("fixture-key".to_owned()),
        &directory.path().join("claude-state"),
        None,
        Duration::from_secs(2),
    )
    .unwrap();
    let store = SqliteStore::open(directory.path().join("state.sqlite3")).unwrap();
    let runtime = LocalOrchestrator::new(
        store.clone(),
        GitService::new(
            vec![directory.path().to_path_buf()],
            directory.path().join("worktrees"),
        )
        .unwrap(),
        provider,
        Duration::from_secs(60),
    );
    let repository = runtime.register_repository(&repository_path).unwrap();
    let changeset = runtime
        .create_changeset(repository.id, &repository.base_sha, None)
        .unwrap();

    let started = runtime.start_run(changeset.id).unwrap();
    assert_eq!(
        started.approval_request.scope.proposal.target_path.as_str(),
        "jet-black-claude-approved.txt"
    );
    let completed = runtime
        .approve_and_complete(started.run_id, &started.approval_request.scope)
        .unwrap();
    assert_eq!(completed.run.state(), RunState::Completed);
    let worktree = store.worktree_for_changeset(changeset.id).unwrap().unwrap();
    assert_eq!(
        fs::read(worktree.path.join("jet-black-claude-approved.txt")).unwrap(),
        b"generated read-only\n"
    );
}
