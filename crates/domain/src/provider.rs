use crate::{DomainError, limits};
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    Mock,
    ClaudeCode,
    Codex,
    #[serde(rename = "opencode")]
    OpenCode,
}

impl ProviderKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::ClaudeCode => "claude-code",
            Self::Codex => "codex",
            Self::OpenCode => "opencode",
        }
    }
}

impl FromStr for ProviderKind {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "mock" => Ok(Self::Mock),
            "claude-code" => Ok(Self::ClaudeCode),
            "codex" => Ok(Self::Codex),
            "opencode" => Ok(Self::OpenCode),
            _ => Err(DomainError::UnsupportedProvider(value.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct ProviderSelection {
    kind: ProviderKind,
    model: Option<String>,
}

impl ProviderSelection {
    pub fn new(kind: ProviderKind, model: Option<String>) -> Result<Self, DomainError> {
        if let Some(model) = &model {
            if model.trim().is_empty() {
                return Err(DomainError::InvalidProviderModel(
                    "provider model must not be blank".to_owned(),
                ));
            }
            if model.len() > limits::MAX_PROVIDER_MODEL_BYTES {
                return Err(DomainError::InvalidProviderModel(format!(
                    "provider model exceeds {} bytes",
                    limits::MAX_PROVIDER_MODEL_BYTES
                )));
            }
        }
        Ok(Self { kind, model })
    }

    pub fn kind(&self) -> ProviderKind {
        self.kind
    }

    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }
}

#[derive(Deserialize)]
struct ProviderSelectionData {
    kind: ProviderKind,
    model: Option<String>,
}

impl<'de> Deserialize<'de> for ProviderSelection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = ProviderSelectionData::deserialize(deserializer)?;
        Self::new(data.kind, data.model).map_err(serde::de::Error::custom)
    }
}
