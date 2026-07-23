use crate::{Id, digest::update_len_prefixed};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ts_rs::TS;

const MUTATION_DIGEST_DOMAIN: &[u8] = b"jet-black:changeset-mutation:v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ChangesetMutationKind {
    Commit,
    Discard,
}

impl ChangesetMutationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Commit => "commit",
            Self::Discard => "discard",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ChangesetMutationScope {
    pub kind: ChangesetMutationKind,
    pub repository_id: Id,
    pub changeset_id: Id,
    pub checkpoint_id: Id,
    #[ts(type = "number")]
    pub expected_version: u64,
    pub base_sha: String,
    pub expected_head_sha: String,
    pub manifest_sha256: String,
}

impl ChangesetMutationScope {
    pub fn digest(&self) -> String {
        let mut hash = Sha256::new();
        update_len_prefixed(&mut hash, MUTATION_DIGEST_DOMAIN);
        update_len_prefixed(&mut hash, self.kind.as_str().as_bytes());
        hash.update(self.repository_id.as_bytes());
        hash.update(self.changeset_id.as_bytes());
        hash.update(self.checkpoint_id.as_bytes());
        hash.update(self.expected_version.to_be_bytes());
        update_len_prefixed(&mut hash, self.base_sha.as_bytes());
        update_len_prefixed(&mut hash, self.expected_head_sha.as_bytes());
        update_len_prefixed(&mut hash, self.manifest_sha256.as_bytes());
        format!("{:x}", hash.finalize())
    }
}
