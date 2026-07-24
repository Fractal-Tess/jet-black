use crate::StructuredError;
use domain::Id;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const PROTOCOL_VERSION: &str = "0.9";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Envelope<T> {
    #[ts(type = "typeof PROTOCOL_VERSION")]
    pub version: String,
    pub request_id: Id,
    pub payload: T,
}

impl<T> Envelope<T> {
    pub fn new(payload: T) -> Self {
        Self {
            version: PROTOCOL_VERSION.to_owned(),
            request_id: Id::new_v4(),
            payload,
        }
    }

    pub fn validate_version(&self) -> Result<(), StructuredError> {
        if self.version == PROTOCOL_VERSION {
            return Ok(());
        }
        Err(StructuredError::unsupported_version(&self.version))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "status", content = "data", rename_all = "snake_case")]
pub enum CommandResult<T> {
    Ok(T),
    Error(StructuredError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ResponseEnvelope<T> {
    #[ts(type = "typeof PROTOCOL_VERSION")]
    pub version: String,
    pub request_id: Id,
    pub result: CommandResult<T>,
}

impl<T> ResponseEnvelope<T> {
    pub fn success(request_id: Id, value: T) -> Self {
        Self {
            version: PROTOCOL_VERSION.to_owned(),
            request_id,
            result: CommandResult::Ok(value),
        }
    }

    pub fn error(request_id: Id, error: StructuredError) -> Self {
        Self {
            version: PROTOCOL_VERSION.to_owned(),
            request_id,
            result: CommandResult::Error(error),
        }
    }
}
