use crate::{Envelope, ResponseEnvelope, StructuredError};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ProductWorkspaceRole {
    Owner,
    Admin,
    Member,
    Guest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ProductUser {
    #[ts(type = "Id")]
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    #[ts(type = "number")]
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ProductWorkspace {
    #[ts(type = "Id")]
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub role: ProductWorkspaceRole,
    #[ts(type = "number")]
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ProductProject {
    #[ts(type = "Id")]
    pub id: Uuid,
    #[ts(type = "Id")]
    pub workspace_id: Uuid,
    pub identifier: String,
    pub name: String,
    pub description: String,
    pub repository_identity: Option<String>,
    pub repository_kind: Option<String>,
    pub repository_location: Option<String>,
    pub repository_origin: Option<String>,
    #[ts(type = "number")]
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct ProductWorkflowState {
    #[ts(type = "Id")]
    pub id: Uuid,
    #[ts(type = "Id")]
    pub project_id: Uuid,
    pub name: String,
    pub state_group: String,
    pub color: String,
    pub position: f64,
    #[ts(type = "number")]
    pub version: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ProductTicketPriority {
    None,
    Urgent,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ProductTicket {
    #[ts(type = "Id")]
    pub id: Uuid,
    #[ts(type = "Id")]
    pub project_id: Uuid,
    #[ts(type = "number")]
    pub sequence_number: u64,
    pub title: String,
    pub description: String,
    #[ts(type = "Id | null")]
    pub state_id: Option<Uuid>,
    pub priority: ProductTicketPriority,
    #[ts(type = "Id")]
    pub created_by_id: Uuid,
    #[ts(type = "number")]
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ProductSprint {
    #[ts(type = "Id")]
    pub id: Uuid,
    #[ts(type = "Id")]
    pub project_id: Uuid,
    pub name: String,
    pub description: String,
    #[ts(type = "number | null")]
    pub starts_at_ms: Option<i64>,
    #[ts(type = "number | null")]
    pub ends_at_ms: Option<i64>,
    pub status: String,
    #[ts(type = "number")]
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ProductModule {
    #[ts(type = "Id")]
    pub id: Uuid,
    #[ts(type = "Id")]
    pub project_id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    #[ts(type = "number | null")]
    pub target_at_ms: Option<i64>,
    #[ts(type = "number")]
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ProductPage {
    #[ts(type = "Id")]
    pub id: Uuid,
    #[ts(type = "Id")]
    pub project_id: Uuid,
    pub title: String,
    pub content: String,
    #[ts(type = "Id")]
    pub created_by_id: Uuid,
    #[ts(type = "number")]
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ProductIntakeItem {
    #[ts(type = "Id")]
    pub id: Uuid,
    #[ts(type = "Id")]
    pub project_id: Uuid,
    pub title: String,
    pub description: String,
    pub submitter_email: Option<String>,
    pub status: String,
    #[ts(type = "Id | null")]
    pub ticket_id: Option<Uuid>,
    #[ts(type = "number")]
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct ProductSnapshot {
    pub user: ProductUser,
    pub workspaces: Vec<ProductWorkspace>,
    pub projects: Vec<ProductProject>,
    pub workflow_states: Vec<ProductWorkflowState>,
    pub tickets: Vec<ProductTicket>,
    pub sprints: Vec<ProductSprint>,
    pub modules: Vec<ProductModule>,
    pub pages: Vec<ProductPage>,
    pub intake: Vec<ProductIntakeItem>,
    #[ts(type = "number")]
    pub event_cursor: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ProductCommand {
    CreateWorkspace {
        slug: String,
        name: String,
    },
    CreateProject {
        #[ts(type = "Id")]
        workspace_id: Uuid,
        identifier: String,
        name: String,
        description: String,
        repository_identity: Option<String>,
        repository_kind: Option<String>,
        repository_location: Option<String>,
    },
    CreateTicket {
        #[ts(type = "Id")]
        project_id: Uuid,
        title: String,
        description: String,
        priority: ProductTicketPriority,
        idempotency_key: String,
    },
    CreateSprint {
        #[ts(type = "Id")]
        project_id: Uuid,
        name: String,
        description: String,
        #[ts(type = "number | null")]
        starts_at_ms: Option<i64>,
        #[ts(type = "number | null")]
        ends_at_ms: Option<i64>,
    },
    CreateModule {
        #[ts(type = "Id")]
        project_id: Uuid,
        name: String,
        description: String,
        #[ts(type = "number | null")]
        target_at_ms: Option<i64>,
    },
    CreatePage {
        #[ts(type = "Id")]
        project_id: Uuid,
        title: String,
        content: String,
    },
    CreateIntakeItem {
        #[ts(type = "Id")]
        project_id: Uuid,
        title: String,
        description: String,
        submitter_email: Option<String>,
    },
    MoveTicket {
        #[ts(type = "Id")]
        ticket_id: Uuid,
        state_group: String,
        #[ts(type = "number")]
        expected_version: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ProductCommandResponse {
    WorkspaceCreated(ProductWorkspace),
    ProjectCreated(ProductProject),
    TicketCreated(ProductTicket),
    SprintCreated(ProductSprint),
    ModuleCreated(ProductModule),
    PageCreated(ProductPage),
    IntakeItemCreated(ProductIntakeItem),
    TicketMoved(ProductTicket),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ProductEvent {
    #[ts(type = "number")]
    pub cursor: u64,
    #[ts(type = "Id")]
    pub workspace_id: Uuid,
    pub aggregate_kind: String,
    #[ts(type = "Id")]
    pub aggregate_id: Uuid,
    #[ts(type = "number")]
    pub aggregate_version: u64,
    pub event_kind: String,
    #[ts(type = "Id | null")]
    pub actor_id: Option<Uuid>,
    pub body: String,
    #[ts(type = "number")]
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ProductEventPage {
    pub events: Vec<ProductEvent>,
    #[ts(type = "number")]
    pub next_cursor: u64,
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ProductClientMessage {
    Subscribe {
        #[ts(type = "Id")]
        workspace_id: Uuid,
        #[ts(type = "number")]
        after_cursor: u64,
    },
    Command(Envelope<ProductCommand>),
    Acknowledge {
        #[ts(type = "number")]
        cursor: u64,
    },
    Ping,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ProductServerMessage {
    Ready {
        #[ts(type = "Id")]
        session_id: Uuid,
    },
    CommandResult(ResponseEnvelope<ProductCommandResponse>),
    Events(ProductEventPage),
    Subscribed {
        #[ts(type = "Id")]
        workspace_id: Uuid,
        #[ts(type = "number")]
        cursor: u64,
    },
    Error(StructuredError),
    Pong,
}
