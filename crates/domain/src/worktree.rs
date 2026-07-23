use crate::{DomainError, Id};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeState {
    Creating,
    Ready,
    Removing,
    Removed,
    Failed,
    Quarantined,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Worktree {
    pub id: Id,
    changeset_id: Id,
    pub path: PathBuf,
    filesystem_identity: String,
    pub base_sha: String,
    state: WorktreeState,
    #[serde(default)]
    #[ts(type = "number")]
    version: u64,
}

impl Worktree {
    pub fn creating(
        id: Id,
        changeset_id: Id,
        path: PathBuf,
        filesystem_identity: String,
        base_sha: String,
    ) -> Self {
        Self {
            id,
            changeset_id,
            path,
            filesystem_identity,
            base_sha,
            state: WorktreeState::Creating,
            version: 0,
        }
    }

    pub fn ready(&mut self) -> Result<(), DomainError> {
        self.transition(&[WorktreeState::Creating], WorktreeState::Ready)
    }

    pub fn begin_removal(&mut self) -> Result<(), DomainError> {
        self.transition(
            &[WorktreeState::Ready, WorktreeState::Failed],
            WorktreeState::Removing,
        )
    }

    pub fn removed(&mut self) -> Result<(), DomainError> {
        if self.state == WorktreeState::Removed {
            return Ok(());
        }
        self.transition(&[WorktreeState::Removing], WorktreeState::Removed)
    }

    pub fn removal_failed(&mut self) -> Result<(), DomainError> {
        self.transition(&[WorktreeState::Removing], WorktreeState::Ready)
    }

    pub fn fail(&mut self) -> Result<(), DomainError> {
        self.transition(
            &[
                WorktreeState::Creating,
                WorktreeState::Ready,
                WorktreeState::Removing,
            ],
            WorktreeState::Failed,
        )
    }

    pub fn quarantine(&mut self) -> Result<(), DomainError> {
        self.transition(
            &[WorktreeState::Ready, WorktreeState::Failed],
            WorktreeState::Quarantined,
        )
    }

    pub fn state(&self) -> WorktreeState {
        self.state
    }

    pub fn changeset_id(&self) -> Id {
        self.changeset_id
    }

    pub fn filesystem_identity(&self) -> &str {
        &self.filesystem_identity
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    fn transition(
        &mut self,
        allowed: &[WorktreeState],
        next: WorktreeState,
    ) -> Result<(), DomainError> {
        if !allowed.contains(&self.state) {
            return Err(DomainError::InvalidTransition {
                entity: "worktree",
                from: format!("{:?}", self.state),
                to: format!("{next:?}"),
            });
        }
        self.state = next;
        self.version = self.version.saturating_add(1);
        Ok(())
    }
}
