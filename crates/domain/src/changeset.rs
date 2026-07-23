use crate::{DomainError, Id, TicketRef};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ChangesetState {
    Created,
    Active,
    Recoverable,
    Reviewable,
    Committed,
    Discarded,
    Failed,
    Divergent,
}

impl ChangesetState {
    pub fn allows(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Created, Self::Active | Self::Discarded | Self::Failed)
                | (
                    Self::Active,
                    Self::Recoverable | Self::Reviewable | Self::Failed | Self::Divergent
                )
                | (
                    Self::Recoverable,
                    Self::Active
                        | Self::Reviewable
                        | Self::Discarded
                        | Self::Failed
                        | Self::Divergent
                )
                | (
                    Self::Reviewable,
                    Self::Committed | Self::Discarded | Self::Failed | Self::Divergent
                )
                | (Self::Divergent, Self::Discarded | Self::Failed)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Changeset {
    pub id: Id,
    repository_id: Id,
    base_sha: String,
    head_sha: String,
    state: ChangesetState,
    #[serde(default)]
    ticket: Option<TicketRef>,
    #[serde(default)]
    #[ts(type = "number")]
    version: u64,
}

impl Changeset {
    pub fn new(repository_id: Id, base_sha: String) -> Self {
        Self {
            id: Id::new_v4(),
            repository_id,
            head_sha: base_sha.clone(),
            base_sha,
            state: ChangesetState::Created,
            ticket: None,
            version: 0,
        }
    }

    pub fn with_ticket(mut self, ticket: TicketRef) -> Self {
        self.ticket = Some(ticket);
        self
    }

    pub fn activate(&mut self) -> Result<(), DomainError> {
        self.transition(ChangesetState::Active)
    }

    pub fn mark_recoverable(&mut self, head_sha: String) -> Result<(), DomainError> {
        self.transition(ChangesetState::Recoverable)?;
        self.head_sha = head_sha;
        Ok(())
    }

    pub fn mark_reviewable(&mut self, head_sha: String) -> Result<(), DomainError> {
        self.transition(ChangesetState::Reviewable)?;
        self.head_sha = head_sha;
        Ok(())
    }

    pub fn commit(&mut self) -> Result<(), DomainError> {
        if self.state == ChangesetState::Committed {
            return Ok(());
        }
        self.transition(ChangesetState::Committed)
    }

    pub fn discard(&mut self) -> Result<(), DomainError> {
        if self.state == ChangesetState::Discarded {
            return Ok(());
        }
        self.transition(ChangesetState::Discarded)
    }

    pub fn fail(&mut self) -> Result<(), DomainError> {
        if self.state == ChangesetState::Failed {
            return Ok(());
        }
        self.transition(ChangesetState::Failed)
    }

    pub fn mark_divergent(&mut self, head_sha: String) -> Result<(), DomainError> {
        self.transition(ChangesetState::Divergent)?;
        self.head_sha = head_sha;
        Ok(())
    }

    pub fn state(&self) -> ChangesetState {
        self.state
    }

    pub fn repository_id(&self) -> Id {
        self.repository_id
    }

    pub fn base_sha(&self) -> &str {
        &self.base_sha
    }

    pub fn head_sha(&self) -> &str {
        &self.head_sha
    }

    pub fn ticket(&self) -> Option<&TicketRef> {
        self.ticket.as_ref()
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    fn transition(&mut self, next: ChangesetState) -> Result<(), DomainError> {
        if !self.state.allows(next) {
            return Err(DomainError::InvalidTransition {
                entity: "changeset",
                from: format!("{:?}", self.state),
                to: format!("{next:?}"),
            });
        }
        self.state = next;
        self.version = self.version.saturating_add(1);
        Ok(())
    }
}
