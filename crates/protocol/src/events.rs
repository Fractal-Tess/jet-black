use domain::{ActionProposal, Id, RelativePath, RunState};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum SemanticEventKind {
    Text {
        text: String,
    },
    ActionProposal {
        proposal: ActionProposal,
        digest: String,
    },
    ActionResult {
        digest: String,
        success: bool,
    },
    FileChange {
        path: RelativePath,
    },
    Approval {
        digest: String,
        approved: bool,
    },
    Lifecycle {
        state: RunState,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OrderedRunEvent {
    pub run_id: Id,
    #[ts(type = "number")]
    pub sequence: u64,
    pub event: SemanticEventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct EventCursor {
    pub run_id: Id,
    #[ts(type = "number")]
    pub after_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct EventPage {
    pub events: Vec<OrderedRunEvent>,
    pub next_cursor: EventCursor,
}
