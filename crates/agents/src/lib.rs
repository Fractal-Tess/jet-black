mod claude;
mod codex;
mod common;
mod opencode;

pub use claude::ClaudeCodeProvider;
pub use codex::CodexProvider;
pub use opencode::OpenCodeProvider;

use domain::{ActionKind, ActionProposal, ApprovalScope, Id, RelativePath};
use execution::{ProcessResult, ProcessSpec};
use protocol::SemanticEventKind;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedFileChange {
    pub content: Vec<u8>,
    pub proposal: ActionProposal,
}

impl ProposedFileChange {
    pub(crate) fn write_file(
        target_path: RelativePath,
        content: Vec<u8>,
    ) -> Result<Self, ProviderError> {
        if content.len() > domain::limits::MAX_APPROVED_FILE_BYTES {
            return Err(ProviderError::InvalidResponse);
        }
        Ok(Self {
            proposal: ActionProposal {
                action: ActionKind::WriteFile,
                target_path,
                content_sha256: git::content_digest(&content),
            },
            content,
        })
    }
}

pub trait AgentProvider {
    fn name(&self) -> &'static str;
    fn process_spec(&self, _worktree_path: &Path) -> Option<ProcessSpec> {
        None
    }
    fn propose(
        &self,
        process_result: Option<&ProcessResult>,
    ) -> Result<ProposedFileChange, ProviderError>;
    fn normalized_events(
        &self,
        change: &ProposedFileChange,
        digest: &str,
    ) -> Vec<SemanticEventKind>;
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider configuration is invalid")]
    InvalidConfiguration,
    #[error("provider executable was not found")]
    ExecutableNotFound,
    #[error("provider credential is unavailable")]
    MissingCredential,
    #[error("provider PATH must contain only absolute directories")]
    UnsafeSearchPath,
    #[error("provider executable discovery failed")]
    DiscoveryIo(#[source] std::io::Error),
    #[error("provider process result was unavailable")]
    MissingProcessResult,
    #[error("provider returned an invalid response")]
    InvalidResponse,
}

#[derive(Debug, Clone)]
pub struct MockProvider {
    target_path: RelativePath,
    content: Vec<u8>,
}

impl MockProvider {
    pub fn deterministic() -> Self {
        Self {
            target_path: RelativePath::parse("jet-black-approved.txt")
                .expect("static mock path must be valid"),
            content: b"approved local change\n".to_vec(),
        }
    }
    pub fn approval_scope(
        &self,
        repository_id: Id,
        changeset_id: Id,
        base_sha: String,
        head_sha: String,
        expires_at_unix_ms: i64,
    ) -> ApprovalScope {
        ApprovalScope {
            repository_id,
            changeset_id,
            base_sha,
            head_sha,
            proposal: self
                .propose(None)
                .expect("deterministic mock proposal must be valid")
                .proposal,
            expires_at_unix_ms,
        }
    }
}

impl AgentProvider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }
    fn propose(
        &self,
        _process_result: Option<&ProcessResult>,
    ) -> Result<ProposedFileChange, ProviderError> {
        ProposedFileChange::write_file(self.target_path.clone(), self.content.clone())
    }
    fn normalized_events(
        &self,
        change: &ProposedFileChange,
        digest: &str,
    ) -> Vec<SemanticEventKind> {
        common::proposal_events(
            "mock provider prepared a deterministic file change",
            change,
            digest,
        )
    }
}

pub enum LocalProvider {
    Mock(MockProvider),
    ClaudeCode(ClaudeCodeProvider),
    Codex(CodexProvider),
    OpenCode(OpenCodeProvider),
}

impl AgentProvider for LocalProvider {
    fn name(&self) -> &'static str {
        match self {
            Self::Mock(provider) => provider.name(),
            Self::ClaudeCode(provider) => provider.name(),
            Self::Codex(provider) => provider.name(),
            Self::OpenCode(provider) => provider.name(),
        }
    }

    fn process_spec(&self, worktree_path: &Path) -> Option<ProcessSpec> {
        match self {
            Self::Mock(provider) => provider.process_spec(worktree_path),
            Self::ClaudeCode(provider) => provider.process_spec(worktree_path),
            Self::Codex(provider) => provider.process_spec(worktree_path),
            Self::OpenCode(provider) => provider.process_spec(worktree_path),
        }
    }

    fn propose(
        &self,
        process_result: Option<&ProcessResult>,
    ) -> Result<ProposedFileChange, ProviderError> {
        match self {
            Self::Mock(provider) => provider.propose(process_result),
            Self::ClaudeCode(provider) => provider.propose(process_result),
            Self::Codex(provider) => provider.propose(process_result),
            Self::OpenCode(provider) => provider.propose(process_result),
        }
    }

    fn normalized_events(
        &self,
        change: &ProposedFileChange,
        digest: &str,
    ) -> Vec<SemanticEventKind> {
        match self {
            Self::Mock(provider) => provider.normalized_events(change, digest),
            Self::ClaudeCode(provider) => provider.normalized_events(change, digest),
            Self::Codex(provider) => provider.normalized_events(change, digest),
            Self::OpenCode(provider) => provider.normalized_events(change, digest),
        }
    }
}
