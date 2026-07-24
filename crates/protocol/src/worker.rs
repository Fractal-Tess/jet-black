use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ProductWorker {
    #[ts(type = "Id")]
    pub id: Uuid,
    #[ts(type = "Id")]
    pub workspace_id: Uuid,
    pub name: String,
    pub protocol_version: String,
    pub capabilities: Vec<String>,
    pub repository_identities: Vec<String>,
    pub status: String,
    #[ts(type = "number | null")]
    pub last_seen_at_ms: Option<i64>,
    #[ts(type = "number")]
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct EnrollWorkerRequest {
    #[ts(type = "Id")]
    pub workspace_id: Uuid,
    pub name: String,
    pub protocol_version: String,
    pub capabilities: Vec<String>,
    pub repository_identities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct EnrolledWorker {
    pub worker: ProductWorker,
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct WorkerHeartbeatRequest {
    pub protocol_version: String,
    pub capabilities: Vec<String>,
    pub repository_identities: Vec<String>,
    pub draining: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CreateRemoteAssignmentRequest {
    #[ts(type = "Id")]
    pub ticket_id: Uuid,
    #[ts(type = "Id")]
    pub worker_id: Uuid,
    pub repository_identity: String,
    pub provider: String,
    pub command_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RemoteAssignment {
    #[ts(type = "Id")]
    pub id: Uuid,
    #[ts(type = "Id")]
    pub workspace_id: Uuid,
    #[ts(type = "Id")]
    pub worker_id: Uuid,
    #[ts(type = "Id")]
    pub ticket_id: Uuid,
    pub repository_identity: String,
    pub provider: String,
    pub command_json: String,
    pub status: String,
    #[ts(type = "number")]
    pub fencing_epoch: u64,
    #[ts(type = "number | null")]
    pub lease_expires_at_ms: Option<i64>,
    #[ts(type = "number")]
    pub next_event_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct WorkerClaimResponse {
    pub assignment: Option<RemoteAssignment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct WorkerEventRequest {
    #[ts(type = "Id")]
    pub assignment_id: Uuid,
    #[ts(type = "number")]
    pub fencing_epoch: u64,
    #[ts(type = "number")]
    pub sequence: u64,
    pub event_kind: String,
    pub body: String,
    pub terminal_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct WorkerEventAck {
    #[ts(type = "Id")]
    pub assignment_id: Uuid,
    #[ts(type = "number")]
    pub sequence: u64,
    #[ts(type = "number")]
    pub fencing_epoch: u64,
}
