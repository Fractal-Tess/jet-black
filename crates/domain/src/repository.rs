use crate::Id;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Repository {
    pub id: Id,
    pub canonical_path: PathBuf,
    pub filesystem_identity: String,
    pub identity: String,
    pub primary_remote: Option<String>,
    pub default_branch: String,
    pub base_sha: String,
    #[serde(default)]
    #[ts(type = "number")]
    pub version: u64,
}
