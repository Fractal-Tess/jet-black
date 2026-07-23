use crate::OrderedRunEvent;
use domain::{
    Approval, ApprovalScope, Changeset, ChangesetMutationKind, Checkpoint, Finding, Id, Repository,
    Run, TicketRef, Worktree, WorktreeState,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum LocalCommand {
    RegisterRepository {
        path: PathBuf,
    },
    CreateChangeset {
        repository_id: Id,
        base_sha: String,
        #[serde(default)]
        ticket: Option<TicketRef>,
    },
    StartRun {
        changeset_id: Id,
    },
    InterruptRun {
        run_id: Id,
    },
    RespondToApproval {
        run_id: Id,
        scope: ApprovalScope,
        approved: bool,
    },
    GetCheckpoint {
        run_id: Id,
    },
    GetDiff {
        changeset_id: Id,
    },
    GetEvents {
        run_id: Id,
        #[ts(type = "number")]
        after_sequence: u64,
        limit: u32,
    },
    GetSnapshot {
        run_id: Id,
    },
    GetHistory {
        changeset_id: Id,
        limit: u32,
    },
    GetRecovery,
    GetFindings {
        changeset_id: Id,
    },
    GetRunArtifacts {
        run_id: Id,
    },
    ReadRunArtifactSegment {
        run_id: Id,
        artifact_id: Id,
        segment_sequence: u32,
    },
    DeleteRunArtifacts {
        run_id: Id,
    },
    PreviewCommit {
        changeset_id: Id,
        #[ts(type = "number")]
        expected_version: u64,
        expected_head_sha: String,
    },
    CommitChangeset {
        changeset_id: Id,
        #[ts(type = "number")]
        expected_version: u64,
        expected_head_sha: String,
        confirmation_digest: String,
    },
    PreviewDiscard {
        changeset_id: Id,
        #[ts(type = "number")]
        expected_version: u64,
        expected_head_sha: String,
    },
    DiscardChangeset {
        changeset_id: Id,
        #[ts(type = "number")]
        expected_version: u64,
        expected_head_sha: String,
        confirmation_digest: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RunStartedResponse {
    pub run_id: Id,
    pub changeset_id: Id,
    pub worktree_id: Id,
    pub approval_id: Option<Id>,
    pub approval_request: Option<ApprovalRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RunCompletedResponse {
    pub run: Run,
    pub changeset: Changeset,
    pub checkpoint: Checkpoint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum LocalCommandResponse {
    RepositoryRegistered(Repository),
    ChangesetCreated(Changeset),
    RunStarted(RunStartedResponse),
    RunInterrupted(Run),
    ApprovalRejected(Run),
    RunCompleted(RunCompletedResponse),
    Checkpoint(CheckpointResponse),
    Diff(DiffResponse),
    Events(crate::EventPage),
    Snapshot(Box<RunSnapshot>),
    History(HistoryResponse),
    Recovery(RecoveryResponse),
    Findings(FindingsResponse),
    RunArtifacts(RunArtifactsResponse),
    RunArtifactSegment(RunArtifactSegmentResponse),
    RunArtifactsDeleted(RunArtifactsDeletedResponse),
    MutationPreview(MutationPreview),
    MutationCompleted(MutationResult),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ApprovalRequest {
    pub run_id: Id,
    pub scope: ApprovalScope,
    pub digest: String,
}

impl From<&Approval> for ApprovalRequest {
    fn from(approval: &Approval) -> Self {
        Self {
            run_id: approval.run_id(),
            scope: approval.scope().clone(),
            digest: approval.digest().to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ApprovalResponse {
    pub run_id: Id,
    pub digest: String,
    pub approved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CheckpointResponse {
    pub checkpoint: Checkpoint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DiffResponse {
    pub changeset_id: Id,
    pub unified_diff: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RunSnapshot {
    pub repository: Repository,
    pub changeset: Changeset,
    pub run: Run,
    pub worktree: Option<Worktree>,
    pub checkpoint: Option<Checkpoint>,
    pub pending_approval: Option<ApprovalRequest>,
    pub findings: Vec<Finding>,
    pub events: Vec<OrderedRunEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct HistoryResponse {
    pub changeset_id: Id,
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct FindingsResponse {
    pub changeset_id: Id,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RecoveryAction {
    pub aggregate_kind: String,
    pub aggregate_id: Id,
    pub action: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RecoveryResponse {
    pub actions: Vec<RecoveryAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum RunArtifactStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RunArtifactSegmentMetadata {
    pub sequence: u32,
    #[ts(type = "number")]
    pub stored_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RunArtifactSummary {
    pub artifact_id: Id,
    pub changeset_id: Id,
    pub run_id: Id,
    pub supervision_id: Id,
    pub stream: RunArtifactStream,
    #[ts(type = "number")]
    pub source_bytes: u64,
    #[ts(type = "number")]
    pub stored_bytes: u64,
    pub segments: Vec<RunArtifactSegmentMetadata>,
    pub sha256: String,
    pub redacted: bool,
    pub process_truncated: bool,
    pub quota_limited: bool,
    #[ts(type = "number")]
    pub created_at_unix_ms: i64,
    #[ts(type = "number")]
    pub updated_at_unix_ms: i64,
    #[ts(type = "number")]
    pub expires_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RunArtifactsResponse {
    pub run_id: Id,
    pub artifacts: Vec<RunArtifactSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RunArtifactSegmentResponse {
    pub run_id: Id,
    pub artifact_id: Id,
    pub stream: RunArtifactStream,
    pub segment: RunArtifactSegmentMetadata,
    pub artifact_sha256: String,
    pub content_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RunArtifactsDeletedResponse {
    pub run_id: Id,
    pub deleted_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct MutationPreview {
    pub kind: ChangesetMutationKind,
    pub changeset_id: Id,
    pub checkpoint_id: Id,
    #[ts(type = "number")]
    pub expected_version: u64,
    pub expected_head_sha: String,
    pub manifest_sha256: String,
    pub confirmation_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MutationResult {
    Commit {
        confirmation_digest: String,
        checkpoint_id: Id,
        manifest_sha256: String,
        changeset: Changeset,
        worktree_state: WorktreeState,
        resulting_head_sha: String,
        app_ref: String,
    },
    Discard {
        confirmation_digest: String,
        checkpoint_id: Id,
        manifest_sha256: String,
        changeset: Changeset,
        worktree_state: WorktreeState,
    },
}
