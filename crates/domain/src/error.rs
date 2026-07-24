use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("invalid {entity} transition from {from} to {to}")]
    InvalidTransition {
        entity: &'static str,
        from: String,
        to: String,
    },
    #[error("a protected action requires a proposal digest")]
    MissingProposalDigest,
    #[error("approval scope does not match the protected action")]
    ApprovalMismatch,
    #[error("approval has expired")]
    ApprovalExpired,
    #[error("approval was already consumed")]
    ApprovalReused,
    #[error("approval belongs to a different run")]
    ApprovalRunMismatch,
    #[error("invalid relative path: {0}")]
    InvalidRelativePath(String),
    #[error("invalid resulting commit SHA")]
    InvalidCommitSha,
    #[error("unsupported provider: {0}")]
    UnsupportedProvider(String),
    #[error("invalid provider model: {0}")]
    InvalidProviderModel(String),
    #[error("changeset is already committed with a different result")]
    CommitResultMismatch,
}
