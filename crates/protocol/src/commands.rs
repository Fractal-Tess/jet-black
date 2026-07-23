use crate::OrderedRunEvent;
use domain::{
    ApprovalScope, Changeset, Checkpoint, Finding, Id, Repository, Run, TicketRef, Worktree,
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
pub struct ApprovalRequest {
    pub run_id: Id,
    pub scope: ApprovalScope,
    pub digest: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct MutationPreview {
    pub changeset_id: Id,
    #[ts(type = "number")]
    pub expected_version: u64,
    pub expected_head_sha: String,
    pub confirmation_digest: String,
}
