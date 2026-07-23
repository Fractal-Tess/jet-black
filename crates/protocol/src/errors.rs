use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct StructuredError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl StructuredError {
    pub fn unsupported_version(version: &str) -> Self {
        Self {
            code: "unsupported_protocol_version".to_owned(),
            message: format!("protocol version {version} is unsupported"),
            retryable: false,
        }
    }
}
