use domain::{ActionKind, ActionProposal, ApprovalScope, Id, RelativePath};
use execution::ProcessSpec;
use protocol::SemanticEventKind;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedFileChange {
    pub content: Vec<u8>,
    pub proposal: ActionProposal,
}

pub trait AgentProvider {
    fn name(&self) -> &'static str;
    fn process_spec(&self, _worktree_path: &Path) -> Option<ProcessSpec> {
        None
    }
    fn propose(&self) -> ProposedFileChange;
    fn normalized_events(
        &self,
        change: &ProposedFileChange,
        digest: &str,
    ) -> Vec<SemanticEventKind>;
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
            proposal: self.propose().proposal,
            expires_at_unix_ms,
        }
    }
}

impl AgentProvider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }
    fn propose(&self) -> ProposedFileChange {
        ProposedFileChange {
            content: self.content.clone(),
            proposal: ActionProposal {
                action: ActionKind::WriteFile,
                target_path: self.target_path.clone(),
                content_sha256: git::content_digest(&self.content),
            },
        }
    }
    fn normalized_events(
        &self,
        change: &ProposedFileChange,
        digest: &str,
    ) -> Vec<SemanticEventKind> {
        vec![
            SemanticEventKind::Text {
                text: "mock provider prepared a deterministic file change".to_owned(),
            },
            SemanticEventKind::ActionProposal {
                proposal: change.proposal.clone(),
                digest: digest.to_owned(),
            },
        ]
    }
}
