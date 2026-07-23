use domain::{
    ActionKind, ActionProposal, ApprovalScope, Changeset, Checkpoint, RelativePath, Repository,
    Run, RunState,
};
use protocol::{
    ApprovalRequest, ApprovalResponse, CommandResult, DiffResponse, Envelope, EventCursor,
    EventPage, HistoryResponse, LocalCommand, LocalCommandResponse, OrderedRunEvent,
    PROTOCOL_VERSION, RecoveryAction, RecoveryResponse, ResponseEnvelope,
    RunArtifactSegmentMetadata, RunArtifactSegmentResponse, RunArtifactStream, RunArtifactSummary,
    RunArtifactsDeletedResponse, RunArtifactsResponse, RunSnapshot, SemanticEventKind,
    StructuredError,
};
use std::path::PathBuf;

#[test]
fn protocol_fixture_round_trips_with_version() {
    let fixture = Envelope::new(LocalCommand::InterruptRun {
        run_id: uuid::Uuid::new_v4(),
    });
    let json = serde_json::to_string(&fixture).unwrap();
    let decoded: Envelope<LocalCommand> = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.version, PROTOCOL_VERSION);
    assert_eq!(decoded, fixture);
    assert!(decoded.validate_version().is_ok());
    let response = ResponseEnvelope::success(decoded.request_id, "ok");
    assert!(matches!(response.result, CommandResult::Ok("ok")));
    let unsupported = Envelope {
        version: "9.9".into(),
        request_id: uuid::Uuid::new_v4(),
        payload: LocalCommand::InterruptRun {
            run_id: uuid::Uuid::new_v4(),
        },
    };
    assert!(unsupported.validate_version().is_err());
}

#[test]
fn all_local_message_shapes_round_trip() {
    let id = uuid::Uuid::new_v4();
    let scope = ApprovalScope {
        repository_id: id,
        changeset_id: id,
        base_sha: "base".into(),
        head_sha: "head".into(),
        proposal: ActionProposal {
            action: ActionKind::WriteFile,
            target_path: RelativePath::parse("file").unwrap(),
            content_sha256: "sha".into(),
        },
        expires_at_unix_ms: 100,
    };
    let changeset = Changeset::new(id, "base".into());
    let repository = Repository {
        id,
        filesystem_identity: "fixture".into(),
        git_directory_identity: "git-fixture".into(),
        canonical_path: PathBuf::from("/repo"),
        identity: "repo".into(),
        primary_remote: None,
        default_branch: "main".into(),
        base_sha: "base".into(),
        version: 0,
    };
    let run = Run::new(changeset.id);
    let checkpoint = Checkpoint {
        id,
        run_id: run.id,
        base_sha: "base".into(),
        head_sha: "head".into(),
        diff: "diff".into(),
    };
    let event = OrderedRunEvent {
        run_id: run.id,
        sequence: 1,
        event: SemanticEventKind::Lifecycle {
            state: RunState::Running,
        },
    };
    let error = StructuredError {
        code: "error".into(),
        message: "message".into(),
        retryable: false,
    };
    let approval_request = ApprovalRequest {
        run_id: run.id,
        scope: scope.clone(),
        digest: scope.digest(),
    };
    let approval_response = ApprovalResponse {
        run_id: run.id,
        digest: scope.digest(),
        approved: true,
    };
    let diff = DiffResponse {
        changeset_id: changeset.id,
        unified_diff: "diff".into(),
    };
    let recovery_action = RecoveryAction {
        aggregate_kind: "changeset".into(),
        aggregate_id: changeset.id,
        action: "orphan_worktree_quarantined".into(),
        detail: "unpersisted worktree was quarantined".into(),
    };
    let recovery = RecoveryResponse {
        actions: vec![recovery_action.clone()],
    };
    let recovery_command = LocalCommand::GetRecovery;
    let artifact_id = uuid::Uuid::new_v4();
    let segment = RunArtifactSegmentMetadata {
        sequence: 0,
        stored_bytes: 4,
        sha256: "segment-sha".into(),
    };
    let artifact = RunArtifactSummary {
        artifact_id,
        changeset_id: changeset.id,
        run_id: run.id,
        supervision_id: uuid::Uuid::new_v4(),
        stream: RunArtifactStream::Stdout,
        source_bytes: 4,
        stored_bytes: 4,
        segments: vec![segment.clone()],
        sha256: "artifact-sha".into(),
        redacted: true,
        process_truncated: false,
        quota_limited: false,
        created_at_unix_ms: 100,
        updated_at_unix_ms: 101,
        expires_at_unix_ms: 200,
    };
    let artifact_commands = [
        LocalCommand::GetRunArtifacts { run_id: run.id },
        LocalCommand::ReadRunArtifactSegment {
            run_id: run.id,
            artifact_id,
            segment_sequence: 0,
        },
        LocalCommand::DeleteRunArtifacts { run_id: run.id },
        LocalCommand::GetEvents {
            run_id: run.id,
            after_sequence: 0,
            limit: 10,
        },
        LocalCommand::GetSnapshot { run_id: run.id },
        LocalCommand::GetHistory {
            changeset_id: changeset.id,
            limit: 10,
        },
    ];
    let artifact_responses = [
        LocalCommandResponse::RunArtifacts(RunArtifactsResponse {
            run_id: run.id,
            artifacts: vec![artifact.clone()],
        }),
        LocalCommandResponse::RunArtifactSegment(RunArtifactSegmentResponse {
            run_id: run.id,
            artifact_id,
            stream: RunArtifactStream::Stdout,
            segment: segment.clone(),
            artifact_sha256: artifact.sha256.clone(),
            content_base64: "ZGF0YQ==".into(),
        }),
        LocalCommandResponse::RunArtifactsDeleted(RunArtifactsDeletedResponse {
            run_id: run.id,
            deleted_count: 1,
        }),
        LocalCommandResponse::Events(EventPage {
            events: vec![event.clone()],
            next_cursor: EventCursor {
                run_id: run.id,
                after_sequence: event.sequence,
            },
        }),
        LocalCommandResponse::Snapshot(Box::new(RunSnapshot {
            repository: repository.clone(),
            changeset: changeset.clone(),
            run: run.clone(),
            worktree: None,
            checkpoint: Some(checkpoint.clone()),
            findings: Vec::new(),
            events: vec![event.clone()],
        })),
        LocalCommandResponse::History(HistoryResponse {
            changeset_id: changeset.id,
            runs: vec![run.clone()],
        }),
    ];
    let values = vec![
        serde_json::to_value(repository.clone()).unwrap(),
        serde_json::to_value(changeset.clone()).unwrap(),
        serde_json::to_value(run.clone()).unwrap(),
        serde_json::to_value(scope.clone()).unwrap(),
        serde_json::to_value(checkpoint.clone()).unwrap(),
        serde_json::to_value(event.clone()).unwrap(),
        serde_json::to_value(error.clone()).unwrap(),
        serde_json::to_value(approval_request.clone()).unwrap(),
        serde_json::to_value(approval_response.clone()).unwrap(),
        serde_json::to_value(diff.clone()).unwrap(),
        serde_json::to_value(recovery_action.clone()).unwrap(),
        serde_json::to_value(recovery.clone()).unwrap(),
        serde_json::to_value(recovery_command.clone()).unwrap(),
    ];
    assert_eq!(
        serde_json::from_value::<Repository>(values[0].clone()).unwrap(),
        repository
    );
    assert_eq!(
        serde_json::from_value::<Changeset>(values[1].clone()).unwrap(),
        changeset
    );
    assert_eq!(
        serde_json::from_value::<Run>(values[2].clone()).unwrap(),
        run
    );
    assert_eq!(
        serde_json::from_value::<ApprovalScope>(values[3].clone()).unwrap(),
        scope
    );
    assert_eq!(
        serde_json::from_value::<Checkpoint>(values[4].clone()).unwrap(),
        checkpoint
    );
    assert_eq!(
        serde_json::from_value::<OrderedRunEvent>(values[5].clone()).unwrap(),
        event
    );
    assert_eq!(
        serde_json::from_value::<StructuredError>(values[6].clone()).unwrap(),
        error
    );
    assert_eq!(
        serde_json::from_value::<ApprovalRequest>(values[7].clone()).unwrap(),
        approval_request
    );
    assert_eq!(
        serde_json::from_value::<ApprovalResponse>(values[8].clone()).unwrap(),
        approval_response
    );
    assert_eq!(
        serde_json::from_value::<DiffResponse>(values[9].clone()).unwrap(),
        diff
    );
    assert_eq!(
        serde_json::from_value::<RecoveryAction>(values[10].clone()).unwrap(),
        recovery_action
    );
    assert_eq!(
        serde_json::from_value::<RecoveryResponse>(values[11].clone()).unwrap(),
        recovery
    );
    assert_eq!(
        serde_json::from_value::<LocalCommand>(values[12].clone()).unwrap(),
        recovery_command
    );
    for command in artifact_commands {
        let value = serde_json::to_value(&command).unwrap();
        assert_eq!(
            serde_json::from_value::<LocalCommand>(value).unwrap(),
            command
        );
    }
    for response in artifact_responses {
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(
            serde_json::from_value::<LocalCommandResponse>(value).unwrap(),
            response
        );
    }
    let public_artifact_json = serde_json::to_string(&artifact).unwrap();
    assert!(!public_artifact_json.contains("\"path\""));
    assert!(!public_artifact_json.contains("\"root\""));
    assert!(!public_artifact_json.contains("\"filename\""));
}
