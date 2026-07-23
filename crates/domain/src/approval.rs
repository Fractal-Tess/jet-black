use crate::Id;
use crate::digest::update_len_prefixed;
use crate::{DomainError, RelativePath};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    WriteFile,
}

impl ActionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WriteFile => "write_file",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ActionProposal {
    pub action: ActionKind,
    pub target_path: RelativePath,
    pub content_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ApprovalScope {
    pub repository_id: Id,
    pub changeset_id: Id,
    pub base_sha: String,
    pub head_sha: String,
    pub proposal: ActionProposal,
    #[ts(type = "number")]
    pub expires_at_unix_ms: i64,
}

impl ApprovalScope {
    pub fn digest(&self) -> String {
        let mut hash = Sha256::new();
        hash.update(self.repository_id.as_bytes());
        hash.update(self.changeset_id.as_bytes());
        update_len_prefixed(&mut hash, self.base_sha.as_bytes());
        update_len_prefixed(&mut hash, self.head_sha.as_bytes());
        update_len_prefixed(&mut hash, self.proposal.action.as_str().as_bytes());
        update_len_prefixed(&mut hash, self.proposal.target_path.as_str().as_bytes());
        update_len_prefixed(&mut hash, self.proposal.content_sha256.as_bytes());
        hash.update(self.expires_at_unix_ms.to_be_bytes());
        format!("{:x}", hash.finalize())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Approval {
    pub id: Id,
    run_id: Id,
    digest: String,
    scope: ApprovalScope,
    consumed_at_unix_ms: Option<i64>,
}

impl Approval {
    pub fn new(run_id: Id, scope: ApprovalScope) -> Self {
        let digest = scope.digest();
        Self {
            id: Id::new_v4(),
            run_id,
            digest,
            scope,
            consumed_at_unix_ms: None,
        }
    }

    pub fn consume_for_run(
        &mut self,
        run_id: Id,
        presented: &ApprovalScope,
        now_unix_ms: i64,
    ) -> Result<ApprovedAction, DomainError> {
        self.validate_for_run(run_id, presented, now_unix_ms)?;
        self.consumed_at_unix_ms = Some(now_unix_ms);
        Ok(ApprovedAction {
            repository_id: self.scope.repository_id,
            changeset_id: self.scope.changeset_id,
            action: self.scope.proposal.action,
            target_path: self.scope.proposal.target_path.clone(),
            content_sha256: self.scope.proposal.content_sha256.clone(),
            base_sha: self.scope.base_sha.clone(),
            head_sha: self.scope.head_sha.clone(),
            action_digest: self.digest.clone(),
        })
    }

    pub fn validate_for_run(
        &self,
        run_id: Id,
        presented: &ApprovalScope,
        now_unix_ms: i64,
    ) -> Result<(), DomainError> {
        if self.run_id != run_id {
            return Err(DomainError::ApprovalRunMismatch);
        }
        if self.consumed_at_unix_ms.is_some() {
            return Err(DomainError::ApprovalReused);
        }
        if now_unix_ms >= self.scope.expires_at_unix_ms {
            return Err(DomainError::ApprovalExpired);
        }
        if self.scope != *presented || self.digest != presented.digest() {
            return Err(DomainError::ApprovalMismatch);
        }
        Ok(())
    }

    pub fn run_id(&self) -> Id {
        self.run_id
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn scope(&self) -> &ApprovalScope {
        &self.scope
    }

    pub fn is_consumed(&self) -> bool {
        self.consumed_at_unix_ms.is_some()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ApprovedAction {
    repository_id: Id,
    changeset_id: Id,
    action: ActionKind,
    target_path: RelativePath,
    content_sha256: String,
    base_sha: String,
    head_sha: String,
    action_digest: String,
}

impl ApprovedAction {
    pub fn repository_id(&self) -> Id {
        self.repository_id
    }

    pub fn changeset_id(&self) -> Id {
        self.changeset_id
    }

    pub fn action(&self) -> ActionKind {
        self.action
    }

    pub fn target_path(&self) -> &RelativePath {
        &self.target_path
    }

    pub fn content_sha256(&self) -> &str {
        &self.content_sha256
    }

    pub fn base_sha(&self) -> &str {
        &self.base_sha
    }

    pub fn head_sha(&self) -> &str {
        &self.head_sha
    }

    pub fn action_digest(&self) -> &str {
        &self.action_digest
    }
}
