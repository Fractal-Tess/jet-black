use crate::Id;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Checkpoint {
    pub id: Id,
    pub run_id: Id,
    pub base_sha: String,
    pub head_sha: String,
    pub diff: String,
}
