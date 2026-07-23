use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use config::{ProviderKind, PublicBootstrap};
use domain::{
    Changeset, ChangesetMutationKind, Id, Repository, Run, RunState, WorktreeState,
    limits::MAX_COMMAND_BODY_BYTES,
};
use futures_util::StreamExt;
use protocol::{
    CommandResult, Envelope, EventCursor, EventPage, HistoryResponse, LocalCommand,
    LocalCommandResponse, MutationPreview, MutationResult, OrderedRunEvent, RecoveryResponse,
    ResponseEnvelope, RunArtifactSegmentMetadata, RunArtifactSegmentResponse, RunArtifactStream,
    RunArtifactSummary, RunArtifactsDeletedResponse, RunArtifactsResponse, RunSnapshot,
    SemanticEventKind, StructuredError,
};
use serde::Deserialize;
use server::{Runtime, StandaloneServer};
use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;
use tower::ServiceExt;

const AUTHORITY: &str = "127.0.0.1:43170";
const ORIGIN: &str = "http://127.0.0.1:43170";

struct FixtureRuntime {
    event_requests: Mutex<Vec<(u64, usize)>>,
    snapshot: RunSnapshot,
}

impl Default for FixtureRuntime {
    fn default() -> Self {
        let repository = Repository {
            id: Id::new_v4(),
            filesystem_identity: "fixture".to_owned(),
            git_directory_identity: "git-fixture".to_owned(),
            canonical_path: PathBuf::from("registered-repository"),
            identity: "fixture-repository".to_owned(),
            primary_remote: None,
            default_branch: "main".to_owned(),
            base_sha: "base".to_owned(),
            version: 0,
        };
        let changeset = Changeset::new(repository.id, repository.base_sha.clone());
        let run = Run::new(changeset.id);
        let event = OrderedRunEvent {
            run_id: run.id,
            sequence: 1,
            event: SemanticEventKind::Text {
                text: "fixture event".to_owned(),
            },
        };
        Self {
            event_requests: Mutex::new(Vec::new()),
            snapshot: RunSnapshot {
                repository,
                changeset,
                run,
                worktree: None,
                checkpoint: None,
                pending_approval: None,
                findings: Vec::new(),
                events: vec![event],
            },
        }
    }
}

impl Runtime for FixtureRuntime {
    fn dispatch(&self, command: LocalCommand) -> Result<LocalCommandResponse, StructuredError> {
        match command {
            LocalCommand::GetRecovery => Ok(LocalCommandResponse::Recovery(RecoveryResponse {
                actions: Vec::new(),
            })),
            LocalCommand::GetEvents { run_id, .. } => Ok(LocalCommandResponse::Events(EventPage {
                events: self.snapshot.events.clone(),
                next_cursor: EventCursor {
                    run_id,
                    after_sequence: 1,
                },
            })),
            LocalCommand::GetSnapshot { .. } => Ok(LocalCommandResponse::Snapshot(Box::new(
                self.snapshot.clone(),
            ))),
            LocalCommand::GetHistory { changeset_id, .. } => {
                Ok(LocalCommandResponse::History(HistoryResponse {
                    changeset_id,
                    runs: vec![self.snapshot.run.clone()],
                }))
            }
            LocalCommand::PreviewCommit {
                changeset_id,
                expected_version,
                expected_head_sha,
            } => Ok(LocalCommandResponse::MutationPreview(MutationPreview {
                kind: ChangesetMutationKind::Commit,
                changeset_id,
                checkpoint_id: Id::new_v4(),
                expected_version,
                expected_head_sha,
                manifest_sha256: "b".repeat(64),
                confirmation_digest: "a".repeat(64),
            })),
            LocalCommand::PreviewDiscard {
                changeset_id,
                expected_version,
                expected_head_sha,
            } => Ok(LocalCommandResponse::MutationPreview(MutationPreview {
                kind: ChangesetMutationKind::Discard,
                changeset_id,
                checkpoint_id: Id::new_v4(),
                expected_version,
                expected_head_sha,
                manifest_sha256: "b".repeat(64),
                confirmation_digest: "c".repeat(64),
            })),
            LocalCommand::CommitChangeset {
                confirmation_digest,
                ..
            } if confirmation_digest != "a".repeat(64) => Err(StructuredError {
                code: "mutation_confirmation_mismatch".to_owned(),
                message: "confirmation did not match the exact mutation preview".to_owned(),
                retryable: false,
            }),
            LocalCommand::CommitChangeset {
                confirmation_digest,
                ..
            } => Ok(LocalCommandResponse::MutationCompleted(
                MutationResult::Commit {
                    confirmation_digest,
                    checkpoint_id: Id::new_v4(),
                    manifest_sha256: "b".repeat(64),
                    changeset: self.snapshot.changeset.clone(),
                    worktree_state: WorktreeState::Removed,
                    resulting_head_sha: "d".repeat(40),
                    app_ref: format!("refs/jet-black/changesets/{}", self.snapshot.changeset.id),
                },
            )),
            LocalCommand::DiscardChangeset {
                confirmation_digest,
                ..
            } => Ok(LocalCommandResponse::MutationCompleted(
                MutationResult::Discard {
                    confirmation_digest,
                    checkpoint_id: Id::new_v4(),
                    manifest_sha256: "b".repeat(64),
                    changeset: self.snapshot.changeset.clone(),
                    worktree_state: WorktreeState::Removed,
                },
            )),
            LocalCommand::GetRunArtifacts { run_id } => {
                Ok(LocalCommandResponse::RunArtifacts(RunArtifactsResponse {
                    run_id,
                    artifacts: vec![RunArtifactSummary {
                        artifact_id: Id::new_v4(),
                        changeset_id: Id::new_v4(),
                        run_id,
                        supervision_id: Id::new_v4(),
                        stream: RunArtifactStream::Stdout,
                        source_bytes: 4,
                        stored_bytes: 4,
                        segments: vec![RunArtifactSegmentMetadata {
                            sequence: 0,
                            stored_bytes: 4,
                            sha256: "segment-sha".to_owned(),
                        }],
                        sha256: "artifact-sha".to_owned(),
                        redacted: true,
                        process_truncated: false,
                        quota_limited: false,
                        created_at_unix_ms: 100,
                        updated_at_unix_ms: 101,
                        expires_at_unix_ms: 200,
                    }],
                }))
            }
            LocalCommand::ReadRunArtifactSegment {
                segment_sequence: u32::MAX,
                ..
            } => Err(StructuredError {
                code: "artifact_integrity_failed".to_owned(),
                message: "artifact content failed integrity verification".to_owned(),
                retryable: false,
            }),
            LocalCommand::ReadRunArtifactSegment {
                run_id,
                artifact_id,
                segment_sequence,
            } => Ok(LocalCommandResponse::RunArtifactSegment(
                RunArtifactSegmentResponse {
                    run_id,
                    artifact_id,
                    stream: RunArtifactStream::Stdout,
                    segment: RunArtifactSegmentMetadata {
                        sequence: segment_sequence,
                        stored_bytes: 4,
                        sha256: "segment-sha".to_owned(),
                    },
                    artifact_sha256: "artifact-sha".to_owned(),
                    content_base64: "ZGF0YQ==".to_owned(),
                },
            )),
            LocalCommand::DeleteRunArtifacts { run_id } if run_id.is_nil() => {
                Err(StructuredError {
                    code: "artifact_run_active".to_owned(),
                    message: "run artifacts cannot be deleted while the run is active".to_owned(),
                    retryable: false,
                })
            }
            LocalCommand::DeleteRunArtifacts { run_id } => Ok(
                LocalCommandResponse::RunArtifactsDeleted(RunArtifactsDeletedResponse {
                    run_id,
                    deleted_count: 1,
                }),
            ),
            _ => Err(StructuredError {
                code: "unsupported".to_owned(),
                message: "unsupported fixture command".to_owned(),
                retryable: false,
            }),
        }
    }

    fn run_state(&self, _run_id: Id) -> Result<RunState, StructuredError> {
        Ok(RunState::Running)
    }

    fn events_after(
        &self,
        run_id: Id,
        after_sequence: u64,
        limit: usize,
    ) -> Result<EventPage, StructuredError> {
        self.event_requests
            .lock()
            .unwrap()
            .push((after_sequence, limit));
        let sequence = after_sequence + 1;
        Ok(EventPage {
            events: vec![OrderedRunEvent {
                run_id,
                sequence,
                event: SemanticEventKind::Text {
                    text: "fixture event".to_owned(),
                },
            }],
            next_cursor: EventCursor {
                run_id,
                after_sequence: sequence,
            },
        })
    }
}

fn fixture_server(runtime: Arc<FixtureRuntime>) -> StandaloneServer {
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

#[derive(Deserialize)]
struct ExchangeResponse {
    csrf_token: String,
}

async fn exchange(server: &StandaloneServer) -> (Router, String, String) {
    let router = server.router();
    let request = Request::post("/api/session/exchange")
        .header(header::HOST, AUTHORITY)
        .header(header::ORIGIN, ORIGIN)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            r#"{{"token":"{}"}}"#,
            server.launch_token().expose()
        )))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
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
) -> (String, ResponseEnvelope<LocalCommandResponse>) {
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
    let text = String::from_utf8(body.to_vec()).unwrap();
    let envelope = serde_json::from_str(&text).unwrap();
    (text, envelope)
}

#[tokio::test]
async fn binds_an_ephemeral_loopback_listener_before_server_creation() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    assert!(address.ip().is_loopback());
    assert_ne!(address.port(), 0);
    let server = StandaloneServer::new(
        address,
        PublicBootstrap {
            profile: "standalone".to_owned(),
            version: "test".to_owned(),
            protocol_version: protocol::PROTOCOL_VERSION.to_owned(),
            enabled_features: vec!["local_execution".to_owned()],
            provider_availability: vec![ProviderKind::Mock],
        },
        Arc::new(FixtureRuntime::default()),
    );
    assert!(server.is_ok());
}

#[tokio::test]
async fn bootstrap_rejects_foreign_hosts_and_exposes_no_launch_secret() {
    let server = fixture_server(Arc::new(FixtureRuntime::default()));
    let router = server.router();
    let rejected = router
        .clone()
        .oneshot(
            Request::get("/api/bootstrap")
                .header(header::HOST, "attacker.invalid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::FORBIDDEN);

    let response = router
        .oneshot(
            Request::get("/api/bootstrap")
                .header(header::HOST, AUTHORITY)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), MAX_COMMAND_BODY_BYTES)
        .await
        .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(!text.contains(server.launch_token().expose()));
    assert!(!text.contains('/'));
}

#[tokio::test]
async fn exchange_is_same_origin_single_use_and_body_limited() {
    let server = fixture_server(Arc::new(FixtureRuntime::default()));
    let missing_origin = server
        .router()
        .oneshot(
            Request::post("/api/session/exchange")
                .header(header::HOST, AUTHORITY)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"token":"{}"}}"#,
                    server.launch_token().expose()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_origin.status(), StatusCode::FORBIDDEN);

    let (router, _, _) = exchange(&server).await;
    let reused = router
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
    assert_eq!(reused.status(), StatusCode::UNAUTHORIZED);

    let oversized = router
        .oneshot(
            Request::post("/api/session/exchange")
                .header(header::HOST, AUTHORITY)
                .header(header::ORIGIN, ORIGIN)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("x".repeat(MAX_COMMAND_BODY_BYTES + 1)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn commands_require_session_origin_and_csrf() {
    let server = fixture_server(Arc::new(FixtureRuntime::default()));
    let (router, cookie, csrf) = exchange(&server).await;
    let run_id = Id::new_v4();
    let envelope = Envelope::new(LocalCommand::GetRunArtifacts { run_id });
    let body = serde_json::to_vec(&envelope).unwrap();

    let missing_session = router
        .clone()
        .oneshot(
            Request::post("/api/commands")
                .header(header::HOST, AUTHORITY)
                .header(header::ORIGIN, ORIGIN)
                .header("x-csrf-token", &csrf)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_session.status(), StatusCode::UNAUTHORIZED);

    let missing_origin = router
        .clone()
        .oneshot(
            Request::post("/api/commands")
                .header(header::HOST, AUTHORITY)
                .header(header::COOKIE, &cookie)
                .header("x-csrf-token", &csrf)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_origin.status(), StatusCode::FORBIDDEN);

    let missing_csrf = router
        .clone()
        .oneshot(
            Request::post("/api/commands")
                .header(header::HOST, AUTHORITY)
                .header(header::ORIGIN, ORIGIN)
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_csrf.status(), StatusCode::UNAUTHORIZED);

    let (_, response) = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::GetRunArtifacts { run_id },
    )
    .await;
    assert!(matches!(
        response.result,
        CommandResult::Ok(LocalCommandResponse::RunArtifacts(RunArtifactsResponse {
            run_id: response_run_id,
            ..
        })) if response_run_id == run_id
    ));
}

#[tokio::test]
async fn authenticated_snapshot_history_and_event_commands_are_typed() {
    let runtime = Arc::new(FixtureRuntime::default());
    let run_id = runtime.snapshot.run.id;
    let changeset_id = runtime.snapshot.changeset.id;
    let server = fixture_server(Arc::clone(&runtime));
    let (router, cookie, csrf) = exchange(&server).await;

    let (_, events) = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::GetEvents {
            run_id,
            after_sequence: 0,
            limit: 10,
        },
    )
    .await;
    assert!(matches!(
        events.result,
        CommandResult::Ok(LocalCommandResponse::Events(page))
            if page.events.len() == 1
                && page.events[0].run_id == run_id
                && page.next_cursor.after_sequence == 1
    ));

    let (_, snapshot) = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::GetSnapshot { run_id },
    )
    .await;
    assert!(matches!(
        snapshot.result,
        CommandResult::Ok(LocalCommandResponse::Snapshot(snapshot))
            if snapshot.run.id == run_id
                && snapshot.changeset.id == changeset_id
                && snapshot.events.len() == 1
    ));

    let (_, history) = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::GetHistory {
            changeset_id,
            limit: 10,
        },
    )
    .await;
    assert!(matches!(
        history.result,
        CommandResult::Ok(LocalCommandResponse::History(history))
            if history.changeset_id == changeset_id
                && history.runs.len() == 1
                && history.runs[0].id == run_id
    ));
}

#[tokio::test]
async fn authenticated_artifact_commands_return_path_free_typed_responses() {
    let server = fixture_server(Arc::new(FixtureRuntime::default()));
    let (router, cookie, csrf) = exchange(&server).await;
    let run_id = Id::new_v4();

    let (list_json, listed) = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::GetRunArtifacts { run_id },
    )
    .await;
    assert!(!list_json.contains("\"path\""));
    assert!(!list_json.contains("\"root\""));
    assert!(!list_json.contains("\"filename\""));
    let artifact_id = match listed.result {
        CommandResult::Ok(LocalCommandResponse::RunArtifacts(response)) => {
            assert_eq!(response.run_id, run_id);
            assert_eq!(response.artifacts.len(), 1);
            response.artifacts[0].artifact_id
        }
        result => panic!("unexpected result: {result:?}"),
    };

    let (_, segment) = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::ReadRunArtifactSegment {
            run_id,
            artifact_id,
            segment_sequence: 0,
        },
    )
    .await;
    assert!(matches!(
        segment.result,
        CommandResult::Ok(LocalCommandResponse::RunArtifactSegment(response))
            if response.run_id == run_id
                && response.artifact_id == artifact_id
                && response.content_base64 == "ZGF0YQ=="
    ));

    let (_, deleted) = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::DeleteRunArtifacts { run_id },
    )
    .await;
    assert!(matches!(
        deleted.result,
        CommandResult::Ok(LocalCommandResponse::RunArtifactsDeleted(response))
            if response.run_id == run_id && response.deleted_count == 1
    ));

    let (integrity_json, integrity) = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::ReadRunArtifactSegment {
            run_id,
            artifact_id,
            segment_sequence: u32::MAX,
        },
    )
    .await;
    assert!(!integrity_json.contains('/'));
    assert!(matches!(
        integrity.result,
        CommandResult::Error(StructuredError { code, retryable: false, .. })
            if code == "artifact_integrity_failed"
    ));

    let (_, active_delete) = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::DeleteRunArtifacts { run_id: Id::nil() },
    )
    .await;
    assert!(matches!(
        active_delete.result,
        CommandResult::Error(StructuredError { code, retryable: false, .. })
            if code == "artifact_run_active"
    ));
}

#[tokio::test]
async fn authenticated_mutation_commands_return_typed_path_free_results() {
    let runtime = Arc::new(FixtureRuntime::default());
    let changeset_id = runtime.snapshot.changeset.id;
    let expected_version = runtime.snapshot.changeset.version();
    let expected_head_sha = runtime.snapshot.changeset.head_sha().to_owned();
    let server = fixture_server(runtime);
    let (router, cookie, csrf) = exchange(&server).await;

    let (_, commit_preview) = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::PreviewCommit {
            changeset_id,
            expected_version,
            expected_head_sha: expected_head_sha.clone(),
        },
    )
    .await;
    assert!(matches!(
        commit_preview.result,
        CommandResult::Ok(LocalCommandResponse::MutationPreview(MutationPreview {
            kind: ChangesetMutationKind::Commit,
            changeset_id: response_changeset_id,
            confirmation_digest,
            ..
        })) if response_changeset_id == changeset_id && confirmation_digest == "a".repeat(64)
    ));

    let (_, mismatch) = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::CommitChangeset {
            changeset_id,
            expected_version,
            expected_head_sha: expected_head_sha.clone(),
            confirmation_digest: "0".repeat(64),
        },
    )
    .await;
    assert!(matches!(
        mismatch.result,
        CommandResult::Error(StructuredError { code, retryable: false, .. })
            if code == "mutation_confirmation_mismatch"
    ));

    let (commit_json, commit) = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::CommitChangeset {
            changeset_id,
            expected_version,
            expected_head_sha: expected_head_sha.clone(),
            confirmation_digest: "a".repeat(64),
        },
    )
    .await;
    assert!(!commit_json.contains("canonical_path"));
    assert!(!commit_json.contains("worktrees/"));
    assert!(matches!(
        commit.result,
        CommandResult::Ok(LocalCommandResponse::MutationCompleted(
            MutationResult::Commit {
                worktree_state: WorktreeState::Removed,
                resulting_head_sha,
                app_ref,
                ..
            }
        )) if resulting_head_sha == "d".repeat(40)
            && app_ref == format!("refs/jet-black/changesets/{changeset_id}")
    ));

    let (_, discard_preview) = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::PreviewDiscard {
            changeset_id,
            expected_version,
            expected_head_sha: expected_head_sha.clone(),
        },
    )
    .await;
    assert!(matches!(
        discard_preview.result,
        CommandResult::Ok(LocalCommandResponse::MutationPreview(MutationPreview {
            kind: ChangesetMutationKind::Discard,
            confirmation_digest,
            ..
        })) if confirmation_digest == "c".repeat(64)
    ));

    let (discard_json, discard) = authenticated_command(
        &router,
        &cookie,
        &csrf,
        LocalCommand::DiscardChangeset {
            changeset_id,
            expected_version,
            expected_head_sha,
            confirmation_digest: "c".repeat(64),
        },
    )
    .await;
    assert!(!discard_json.contains("canonical_path"));
    assert!(!discard_json.contains("worktrees/"));
    assert!(matches!(
        discard.result,
        CommandResult::Ok(LocalCommandResponse::MutationCompleted(
            MutationResult::Discard {
                worktree_state: WorktreeState::Removed,
                ..
            }
        ))
    ));
}

#[tokio::test]
async fn authenticated_recovery_and_sse_replay_use_bounded_cursors() {
    let runtime = Arc::new(FixtureRuntime::default());
    let server = fixture_server(Arc::clone(&runtime));
    let (router, cookie, _) = exchange(&server).await;

    let recovery = router
        .clone()
        .oneshot(
            Request::get("/api/recovery")
                .header(header::HOST, AUTHORITY)
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(recovery.status(), StatusCode::OK);

    let run_id = Id::new_v4();
    let response = router
        .oneshot(
            Request::get(format!("/api/runs/{run_id}/events?after_sequence=7"))
                .header(header::HOST, AUTHORITY)
                .header(header::COOKIE, cookie)
                .header("last-event-id", "9")
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
    let mut stream = response.into_body().into_data_stream();
    let chunk = tokio::time::timeout(std::time::Duration::from_secs(1), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let text = String::from_utf8(chunk.to_vec()).unwrap();
    assert!(text.contains("id: 10"));
    assert_eq!(*runtime.event_requests.lock().unwrap(), vec![(9, 100)]);
}
