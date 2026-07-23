use crate::{DomainError, Id};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum RunKind {
    #[default]
    Mutation,
    ReadOnlyReview,
    Precheck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Queued,
    Starting,
    Running,
    AwaitingApproval,
    Reviewable,
    Completed,
    Interrupted,
    Failed,
}

impl RunState {
    pub fn allows(self, next: Self) -> bool {
        matches!(
            (self, next),
            (
                Self::Queued,
                Self::Starting | Self::Interrupted | Self::Failed
            ) | (
                Self::Starting,
                Self::Running | Self::Interrupted | Self::Failed
            ) | (
                Self::Running,
                Self::AwaitingApproval | Self::Reviewable | Self::Interrupted | Self::Failed
            ) | (
                Self::AwaitingApproval,
                Self::Running | Self::Interrupted | Self::Failed
            ) | (
                Self::Reviewable,
                Self::Completed | Self::Interrupted | Self::Failed
            )
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Run {
    pub id: Id,
    changeset_id: Id,
    state: RunState,
    proposal_digest: Option<String>,
    #[serde(default)]
    kind: RunKind,
    #[serde(default)]
    #[ts(type = "number")]
    version: u64,
}

impl Run {
    pub fn new(changeset_id: Id) -> Self {
        Self::new_kind(changeset_id, RunKind::Mutation)
    }

    pub fn new_kind(changeset_id: Id, kind: RunKind) -> Self {
        Self {
            id: Id::new_v4(),
            changeset_id,
            state: RunState::Queued,
            proposal_digest: None,
            kind,
            version: 0,
        }
    }

    pub fn start(&mut self) -> Result<(), DomainError> {
        self.transition(RunState::Starting)
    }

    pub fn running(&mut self) -> Result<(), DomainError> {
        self.transition(RunState::Running)
    }

    pub fn await_approval(&mut self, digest: String) -> Result<(), DomainError> {
        if digest.is_empty() {
            return Err(DomainError::MissingProposalDigest);
        }
        self.transition(RunState::AwaitingApproval)?;
        self.proposal_digest = Some(digest);
        Ok(())
    }

    pub fn resume_after_approval(&mut self) -> Result<(), DomainError> {
        self.transition(RunState::Running)
    }

    pub fn mark_reviewable(&mut self) -> Result<(), DomainError> {
        self.transition(RunState::Reviewable)
    }

    pub fn complete(&mut self) -> Result<(), DomainError> {
        if self.state == RunState::Completed {
            return Ok(());
        }
        self.transition(RunState::Completed)
    }

    pub fn interrupt(&mut self) -> Result<(), DomainError> {
        if self.state == RunState::Interrupted {
            return Ok(());
        }
        self.transition(RunState::Interrupted)
    }

    pub fn fail(&mut self) -> Result<(), DomainError> {
        if self.state == RunState::Failed {
            return Ok(());
        }
        self.transition(RunState::Failed)
    }

    pub fn state(&self) -> RunState {
        self.state
    }

    pub fn changeset_id(&self) -> Id {
        self.changeset_id
    }

    pub fn proposal_digest(&self) -> Option<&str> {
        self.proposal_digest.as_deref()
    }

    pub fn kind(&self) -> RunKind {
        self.kind
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    fn transition(&mut self, next: RunState) -> Result<(), DomainError> {
        if !self.state.allows(next) {
            return Err(DomainError::InvalidTransition {
                entity: "run",
                from: format!("{:?}", self.state),
                to: format!("{next:?}"),
            });
        }
        self.state = next;
        self.version = self.version.saturating_add(1);
        Ok(())
    }
}
