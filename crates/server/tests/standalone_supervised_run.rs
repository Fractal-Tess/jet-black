#![cfg(target_os = "linux")]

use agents::{AgentProvider, MockProvider, ProposedFileChange, ProviderError};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use config::{ProviderKind, PublicBootstrap};
use domain::{ChangesetState, Id, RunState, WorktreeState, limits::MAX_COMMAND_BODY_BYTES};
use execution::{
    LinuxFilesystemConfinement, ProcessConfinement, ProcessResult, ProcessSpec, SupervisionState,
    TerminationReason,
};
use git::GitService;
use orchestration::LocalOrchestrator;
use persistence::{ArtifactPolicy, LocalArtifactStore, SqliteStore};
use protocol::{
    CommandResult, Envelope, LocalCommand, LocalCommandResponse, MutationResult, OrderedRunEvent,
    ResponseEnvelope, ReviewCheckKind, ReviewCheckStatus, SemanticEventKind,
};
use review::ReviewOptions;
use serde::Deserialize;
use server::{Runtime, StandaloneServer};
use std::{
    collections::HashMap,
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::Duration,
};
use tokio::time::{Instant, sleep, timeout};
use tower::ServiceExt;

const AUTHORITY: &str = "127.0.0.1:43171";
const ORIGIN: &str = "http://127.0.0.1:43171";
const PROVIDER_OUTPUT: &str = "supervised-provider-output";

struct StandaloneGateProvider {
    state_directory: PathBuf,
    outside_read_path: PathBuf,
    outside_write_path: PathBuf,
}

impl AgentProvider for StandaloneGateProvider {
    fn name(&self) -> &'static str {
        "standalone-gate"
    }

    fn requires_process_confinement(&self) -> bool {
        true
    }

    fn process_spec(&self, worktree_path: &Path) -> Option<ProcessSpec> {
        Some(ProcessSpec {
            program: "/bin/sh".to_owned(),
            arguments: vec![
                "-c".to_owned(),
                "if IFS= read -r value < \"$1\"; then exit 81; fi; if (printf blocked > \"$2\") 2>/dev/null; then exit 82; fi; printf state > \"$3\"; \"$5\" --version >/dev/null 2>&1 || exit 84; \"$5\" rev-parse --git-common-dir > \"$3.git-common\" 2>/dev/null || exit 83; printf %s \"$4\"".to_owned(),
                "jet-black-gate".to_owned(),
                self.outside_read_path.to_string_lossy().into_owned(),
                self.outside_write_path.to_string_lossy().into_owned(),
                self.state_directory
                    .join("provider-marker")
                    .to_string_lossy()
                    .into_owned(),
                PROVIDER_OUTPUT.to_owned(),
                command_path("git").to_string_lossy().into_owned(),
            ],
            environment: HashMap::new(),
            sensitive_environment_keys: Vec::new(),
            current_dir: None,
            timeout: Duration::from_secs(5),
            output_limit: 1024,
            confinement: ProcessConfinement::LinuxFilesystem(LinuxFilesystemConfinement {
                workspace: worktree_path.to_path_buf(),
                writable_state: self.state_directory.clone(),
                runtime_read_execute: runtime_read_execute_paths(),
                runtime_read_only: Vec::new(),
                runtime_read_write: vec![PathBuf::from("/dev/null")],
            }),
        })
    }

    fn propose(
        &self,
        process_result: Option<&ProcessResult>,
    ) -> Result<ProposedFileChange, ProviderError> {
        let process_result = process_result.ok_or(ProviderError::MissingProcessResult)?;
        if process_result.stdout != PROVIDER_OUTPUT.as_bytes() {
            return Err(ProviderError::InvalidResponse);
        }
        MockProvider::deterministic().propose(Some(process_result))
    }

    fn normalized_events(
        &self,
        change: &ProposedFileChange,
        digest: &str,
    ) -> Vec<SemanticEventKind> {
        MockProvider::deterministic().normalized_events(change, digest)
    }
}

#[derive(Deserialize)]
struct ExchangeResponse {
    csrf_token: String,
}

fn run_git(repository: &Path, arguments: &[&str]) -> String {
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
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn fixture_repository(root: &Path) -> (PathBuf, String) {
    let repository = root.join("repository");
    fs::create_dir(&repository).unwrap();
    run_git(&repository, &["init", "-q"]);
    run_git(&repository, &["config", "user.email", "test@example.com"]);
    run_git(&repository, &["config", "user.name", "Test"]);
    fs::write(repository.join("README.md"), "fixture\n").unwrap();
    run_git(&repository, &["add", "."]);
    run_git(&repository, &["commit", "-qm", "fixture"]);
    let base_sha = run_git(&repository, &["rev-parse", "HEAD"]);
    (repository, base_sha)
}

fn command_path(name: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
        .and_then(|candidate| fs::canonicalize(candidate).ok())
        .unwrap()
}

fn runtime_read_execute_paths() -> Vec<PathBuf> {
    if fs::canonicalize("/bin/sh").is_ok_and(|path| path.starts_with("/nix/store")) {
        return vec![PathBuf::from("/nix/store")];
    }
    [
        "/bin",
        "/lib",
        "/lib64",
        "/usr/bin",
        "/usr/lib",
        "/usr/lib64",
    ]
    .into_iter()
    .map(PathBuf::from)
    .filter(|path| path.exists())
    .collect()
}

fn compose_runtime(
    root: &Path,
    database_path: &Path,
) -> (Arc<LocalOrchestrator<StandaloneGateProvider>>, SqliteStore) {
    let state_directory = root.join("provider-state");
    fs::create_dir_all(&state_directory).unwrap();
    let store = SqliteStore::open(database_path).unwrap();
    let git = GitService::new(vec![root.to_path_buf()], root.join("worktrees")).unwrap();
    let artifact_store = LocalArtifactStore::new(
        store.clone(),
        root.join("artifacts"),
        ArtifactPolicy::default(),
    )
    .unwrap();
    let runtime = LocalOrchestrator::new(
        store.clone(),
        git,
        StandaloneGateProvider {
            state_directory,
            outside_read_path: root.join("outside-secret"),
            outside_write_path: root.join("outside-write"),
        },
        Duration::from_secs(60),
    )
    .with_artifact_store(artifact_store)
    .with_review_options(ReviewOptions::default(), "")
    .unwrap();
    (Arc::new(runtime), store)
}

fn test_server(runtime: Arc<dyn Runtime>) -> StandaloneServer {
    StandaloneServer::new(
        AUTHORITY.parse::<SocketAddr>().unwrap(),
        PublicBootstrap {
            profile: "standalone".to_owned(),
            version: "test".to_owned(),
            protocol_version: protocol::PROTOCOL_VERSION.to_owned(),
            enabled_features: vec!["local_execution".to_owned()],
            provider_availability: vec![ProviderKind::Mock],
        },
        runtime,
    )
    .unwrap()
}

async fn exchange(server: &StandaloneServer) -> (Router, String, String) {
    let router = server.router();
    let response = router
        .clone()
        .oneshot(
            Request::post("/api/session/exchange")
                .header(header::HOST, AUTHORITY)
                .header(header::ORIGIN, ORIGIN)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"token":"{}"}}"#,
                    server.launch_token().expose()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Strict"));
    let session_cookie = cookie.split(';').next().unwrap().to_owned();
    let body = to_bytes(response.into_body(), MAX_COMMAND_BODY_BYTES)
        .await
        .unwrap();
    let exchange: ExchangeResponse = serde_json::from_slice(&body).unwrap();
    (router, session_cookie, exchange.csrf_token)
}

async fn authenticated_command(
    router: &Router,
    cookie: &str,
    csrf: &str,
    command: LocalCommand,
) -> LocalCommandResponse {
    let response = router
        .clone()
        .oneshot(
            Request::post("/api/commands")
                .header(header::HOST, AUTHORITY)
                .header(header::ORIGIN, ORIGIN)
                .header(header::COOKIE, cookie)
                .header("x-csrf-token", csrf)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&Envelope::new(command)).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), MAX_COMMAND_BODY_BYTES)
        .await
        .unwrap();
    let envelope: ResponseEnvelope<LocalCommandResponse> = serde_json::from_slice(&body).unwrap();
    match envelope.result {
        CommandResult::Ok(response) => response,
        CommandResult::Error(error) => panic!("command failed: {error:?}"),
    }
}

async fn wait_for_approval(
    router: &Router,
    cookie: &str,
    csrf: &str,
    run_id: Id,
) -> protocol::RunSnapshot {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut poll_interval = Duration::from_millis(25);
    loop {
        let response =
            authenticated_command(router, cookie, csrf, LocalCommand::GetSnapshot { run_id }).await;
        let LocalCommandResponse::Snapshot(snapshot) = response else {
            panic!("expected run snapshot");
        };
        if snapshot.run.state() == RunState::AwaitingApproval && snapshot.pending_approval.is_some()
        {
            return *snapshot;
        }
        assert!(
            Instant::now() < deadline,
            "run did not reach approval: {:?}",
            snapshot.run
        );
        sleep(poll_interval).await;
        poll_interval = (poll_interval * 2).min(Duration::from_millis(250));
    }
}

async fn replay_events(router: &Router, cookie: &str, run_id: Id) -> Vec<OrderedRunEvent> {
    let response = router
        .clone()
        .oneshot(
            Request::get(format!("/api/runs/{run_id}/events?after_sequence=0"))
                .header(header::HOST, AUTHORITY)
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/event-stream"
    );
    let body = timeout(
        Duration::from_secs(2),
        to_bytes(response.into_body(), MAX_COMMAND_BODY_BYTES),
    )
    .await
    .expect("terminal event stream did not close")
    .unwrap();
    String::from_utf8(body.to_vec())
        .unwrap()
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(|data| serde_json::from_str(data).unwrap())
        .collect()
}

#[tokio::test]
async fn standalone_supervised_run_survives_restart_and_finalizes_exact_revision() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fs::write(root.join("outside-secret"), "must remain inaccessible\n").unwrap();
    let database_path = root.join("standalone.sqlite3");
    let (repository_path, base_sha) = fixture_repository(root);
    let (runtime, store) = compose_runtime(root, &database_path);
    let initial_recovery = runtime.recover().unwrap();
    assert!(initial_recovery.actions.is_empty());

    let server = test_server(runtime.clone());
    let (router, cookie, csrf) = exchange(&server).await;

    let response = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::RegisterRepository {
            path: repository_path.clone(),
        },
    )
    .await;
    let LocalCommandResponse::RepositoryRegistered(repository) = response else {
        panic!("expected registered repository");
    };
    assert_eq!(repository.base_sha, base_sha);

    let response = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::CreateChangeset {
            repository_id: repository.id,
            base_sha: base_sha.clone(),
            ticket: None,
        },
    )
    .await;
    let LocalCommandResponse::ChangesetCreated(changeset) = response else {
        panic!("expected created changeset");
    };

    let response = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::StartRun {
            changeset_id: changeset.id,
        },
    )
    .await;
    let LocalCommandResponse::RunStarted(started) = response else {
        panic!("expected started run");
    };
    assert_eq!(started.changeset_id, changeset.id);
    assert!(started.approval_id.is_none());
    assert!(started.approval_request.is_none());

    let approval_snapshot = wait_for_approval(&router, &cookie, &csrf, started.run_id).await;
    let worktree_path = approval_snapshot.worktree.as_ref().unwrap().path.clone();

    let supervision = store
        .latest_process_supervision_for_run(started.run_id)
        .unwrap()
        .unwrap();
    assert_eq!(supervision.metadata.state, SupervisionState::Terminated);
    assert_eq!(
        supervision.metadata.termination_reason,
        Some(TerminationReason::Exited { code: 0 })
    );
    let approval = approval_snapshot.pending_approval.clone().unwrap();
    assert!(matches!(
        supervision.metadata.confinement.filesystem,
        execution::FilesystemConfinementReport::Landlock { abi: 3, .. }
    ));
    assert_eq!(
        supervision.metadata.confinement.network,
        execution::NetworkConfinementReport::NotOsConfined
    );
    assert_eq!(
        fs::read(root.join("outside-secret")).unwrap(),
        b"must remain inaccessible\n"
    );
    assert!(!root.join("outside-write").exists());
    assert_eq!(
        fs::read(root.join("provider-state/provider-marker")).unwrap(),
        b"state"
    );
    let git_common_directory =
        fs::read_to_string(root.join("provider-state/provider-marker.git-common")).unwrap();
    assert_eq!(
        fs::canonicalize(git_common_directory.trim()).unwrap(),
        fs::canonicalize(repository_path.join(".git")).unwrap()
    );

    let response = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::GetRunArtifacts {
            run_id: started.run_id,
        },
    )
    .await;
    let LocalCommandResponse::RunArtifacts(artifacts) = response else {
        panic!("expected run artifacts");
    };
    let stdout = artifacts
        .artifacts
        .iter()
        .find(|artifact| artifact.stream == protocol::RunArtifactStream::Stdout)
        .unwrap();
    assert_eq!(stdout.source_bytes, PROVIDER_OUTPUT.len() as u64);
    let mut artifact_content = Vec::new();
    for metadata in &stdout.segments {
        let response = authenticated_command(
            &router,
            &cookie,
            &csrf,
            LocalCommand::ReadRunArtifactSegment {
                run_id: started.run_id,
                artifact_id: stdout.artifact_id,
                segment_sequence: metadata.sequence,
            },
        )
        .await;
        let LocalCommandResponse::RunArtifactSegment(segment) = response else {
            panic!("expected artifact segment");
        };
        artifact_content.extend(STANDARD.decode(segment.content_base64).unwrap());
    }
    assert_eq!(artifact_content, PROVIDER_OUTPUT.as_bytes());

    let response = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::RespondToApproval {
            run_id: started.run_id,
            scope: approval.scope,
            approved: true,
        },
    )
    .await;
    let LocalCommandResponse::RunCompleted(completed) = response else {
        panic!("expected completed run");
    };
    assert_eq!(completed.run.state(), RunState::Completed);
    assert_eq!(completed.changeset.state(), ChangesetState::Reviewable);
    assert!(completed.checkpoint.diff.contains("jet-black-approved.txt"));

    let events = replay_events(&router, &cookie, started.run_id).await;
    assert!(!events.is_empty());
    assert!(
        events
            .windows(2)
            .all(|pair| pair[1].sequence == pair[0].sequence + 1)
    );
    assert!(events.iter().any(|event| matches!(
        event.event,
        SemanticEventKind::Lifecycle {
            state: RunState::Completed
        }
    )));

    let reviewable = completed.changeset;
    let response = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::ReviewChangeset {
            changeset_id: reviewable.id,
            expected_version: reviewable.version(),
            expected_head_sha: reviewable.head_sha().to_owned(),
            checks: vec![ReviewCheckKind::SecretScan],
        },
    )
    .await;
    let LocalCommandResponse::ReviewCompleted(report) = response else {
        panic!("expected review report");
    };
    assert_eq!(report.changeset_version, reviewable.version());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].status, ReviewCheckStatus::Unavailable);
    assert_eq!(report.findings.len(), 1);

    drop(router);
    drop(server);
    drop(runtime);
    drop(store);

    let (restarted_runtime, _) = compose_runtime(root, &database_path);
    let recovery = restarted_runtime.recover().unwrap();
    assert!(recovery.terminal.contains(&started.run_id));
    assert!(!recovery.recoverable.contains(&started.run_id));
    assert!(!recovery.interrupted.contains(&started.run_id));
    assert!(!recovery.failed.contains(&started.run_id));

    let restarted_server = test_server(restarted_runtime);
    let (restarted_router, restarted_cookie, restarted_csrf) = exchange(&restarted_server).await;
    let response = authenticated_command(
        &restarted_router,
        &restarted_cookie,
        &restarted_csrf,
        LocalCommand::GetSnapshot {
            run_id: started.run_id,
        },
    )
    .await;
    let LocalCommandResponse::Snapshot(snapshot) = response else {
        panic!("expected recovered snapshot");
    };
    assert_eq!(snapshot.run.state(), RunState::Completed);
    assert_eq!(snapshot.changeset, reviewable);
    assert_eq!(snapshot.findings, report.findings);

    let response = authenticated_command(
        &restarted_router,
        &restarted_cookie,
        &restarted_csrf,
        LocalCommand::PreviewCommit {
            changeset_id: reviewable.id,
            expected_version: reviewable.version(),
            expected_head_sha: reviewable.head_sha().to_owned(),
        },
    )
    .await;
    let LocalCommandResponse::MutationPreview(preview) = response else {
        panic!("expected commit preview");
    };
    let commit_command = LocalCommand::CommitChangeset {
        changeset_id: reviewable.id,
        expected_version: preview.expected_version,
        expected_head_sha: preview.expected_head_sha.clone(),
        confirmation_digest: preview.confirmation_digest.clone(),
    };
    let response = authenticated_command(
        &restarted_router,
        &restarted_cookie,
        &restarted_csrf,
        commit_command.clone(),
    )
    .await;
    let LocalCommandResponse::MutationCompleted(result) = response else {
        panic!("expected commit result");
    };
    let MutationResult::Commit {
        changeset: committed_changeset,
        worktree_state,
        resulting_head_sha,
        app_ref,
        ..
    } = &result
    else {
        panic!("expected committed mutation");
    };
    assert_eq!(committed_changeset.state(), ChangesetState::Committed);
    assert_eq!(committed_changeset.head_sha(), resulting_head_sha);
    assert_eq!(*worktree_state, WorktreeState::Removed);
    assert!(!worktree_path.exists());
    assert_eq!(
        run_git(&repository_path, &["rev-parse", app_ref]),
        *resulting_head_sha
    );
    let parent_ref = format!("{resulting_head_sha}^");
    assert_eq!(
        run_git(&repository_path, &["rev-parse", &parent_ref]),
        preview.expected_head_sha
    );
    let approved_file = format!("{resulting_head_sha}:jet-black-approved.txt");
    assert_eq!(
        run_git(&repository_path, &["show", &approved_file]),
        "approved local change"
    );

    let replay = authenticated_command(
        &restarted_router,
        &restarted_cookie,
        &restarted_csrf,
        commit_command,
    )
    .await;
    assert_eq!(replay, LocalCommandResponse::MutationCompleted(result));
}
