use crate::DomainError;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path};
use ts_rs::TS;
use uuid::Uuid;

pub type Id = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
pub struct ControlPlaneId(pub Id);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
pub struct ExecutionTargetId(pub Id);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
pub struct TicketRef {
    pub control_plane_id: Option<ControlPlaneId>,
    pub workspace_id: String,
    pub project_id: String,
    pub ticket_id: String,
    pub identifier: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, TS)]
pub struct RelativePath(String);

impl<'de> Deserialize<'de> for RelativePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

impl RelativePath {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let path = Path::new(&value);
        let portable = !value.is_empty()
            && !value.contains('\\')
            && !value.contains('\0')
            && !value.contains(':')
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)));
        if !portable {
            return Err(DomainError::InvalidRelativePath(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}
