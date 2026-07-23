use crate::{DomainError, Id, RelativePath};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum FindingState {
    Open,
    Resolved,
    Dismissed,
    Superseded,
}

impl FindingState {
    pub fn allows(self, next: Self) -> bool {
        self == Self::Open && matches!(next, Self::Resolved | Self::Dismissed | Self::Superseded)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Finding {
    pub id: Id,
    changeset_id: Id,
    pub path: Option<RelativePath>,
    pub blob_identity: String,
    pub line_range: Option<(u32, u32)>,
    pub category: String,
    pub severity: String,
    pub message: String,
    pub evidence: String,
    state: FindingState,
    #[serde(default)]
    #[ts(type = "number")]
    version: u64,
}

impl Finding {
    pub fn new(
        changeset_id: Id,
        path: Option<RelativePath>,
        blob_identity: String,
        category: String,
        severity: String,
        message: String,
        evidence: String,
    ) -> Self {
        Self {
            id: Id::new_v4(),
            changeset_id,
            path,
            blob_identity,
            line_range: None,
            category,
            severity,
            message,
            evidence,
            state: FindingState::Open,
            version: 0,
        }
    }

    pub fn resolve(&mut self) -> Result<(), DomainError> {
        self.transition(FindingState::Resolved)
    }

    pub fn dismiss(&mut self) -> Result<(), DomainError> {
        self.transition(FindingState::Dismissed)
    }

    pub fn supersede(&mut self) -> Result<(), DomainError> {
        self.transition(FindingState::Superseded)
    }

    pub fn state(&self) -> FindingState {
        self.state
    }

    pub fn changeset_id(&self) -> Id {
        self.changeset_id
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    fn transition(&mut self, next: FindingState) -> Result<(), DomainError> {
        if !self.state.allows(next) {
            return Err(DomainError::InvalidTransition {
                entity: "finding",
                from: format!("{:?}", self.state),
                to: format!("{next:?}"),
            });
        }
        self.state = next;
        self.version = self.version.saturating_add(1);
        Ok(())
    }
}
