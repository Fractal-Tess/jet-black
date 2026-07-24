use crate::{
    ClaudeCodeProviderFactory, CodexProviderFactory, LocalProvider, MockProvider,
    OpenCodeProviderFactory, ProviderError,
};
use domain::{ProviderKind, ProviderSelection};
use std::sync::Arc;

pub struct ResolvedProvider {
    pub selection: ProviderSelection,
    pub provider: Arc<dyn crate::AgentProvider>,
}

pub trait ProviderResolver: Send + Sync {
    fn default_selection(&self) -> &ProviderSelection;
    fn available_kinds(&self) -> Vec<ProviderKind>;
    fn resolve(&self, selection: &ProviderSelection) -> Result<ResolvedProvider, ProviderError>;
}

pub struct FixedProviderResolver<P> {
    selection: ProviderSelection,
    provider: Arc<P>,
}

impl<P> FixedProviderResolver<P> {
    pub fn new(selection: ProviderSelection, provider: P) -> Self {
        Self {
            selection,
            provider: Arc::new(provider),
        }
    }

    pub fn mock(provider: P) -> Self {
        Self::new(
            ProviderSelection::new(ProviderKind::Mock, None)
                .expect("the fixed mock provider selection must be valid"),
            provider,
        )
    }
}

impl<P> ProviderResolver for FixedProviderResolver<P>
where
    P: crate::AgentProvider + 'static,
{
    fn default_selection(&self) -> &ProviderSelection {
        &self.selection
    }

    fn available_kinds(&self) -> Vec<ProviderKind> {
        vec![self.selection.kind()]
    }

    fn resolve(&self, selection: &ProviderSelection) -> Result<ResolvedProvider, ProviderError> {
        if selection != &self.selection {
            return Err(ProviderError::ProviderUnavailable(selection.kind()));
        }
        let provider: Arc<dyn crate::AgentProvider> = self.provider.clone();
        Ok(ResolvedProvider {
            selection: selection.clone(),
            provider,
        })
    }
}

pub struct LocalProviderRegistry {
    default_selection: ProviderSelection,
    mock: Option<MockProvider>,
    claude_code: Option<ClaudeCodeProviderFactory>,
    codex: Option<CodexProviderFactory>,
    opencode: Option<OpenCodeProviderFactory>,
}

impl LocalProviderRegistry {
    pub fn new(
        default_selection: ProviderSelection,
        mock: Option<MockProvider>,
        claude_code: Option<ClaudeCodeProviderFactory>,
        codex: Option<CodexProviderFactory>,
        opencode: Option<OpenCodeProviderFactory>,
    ) -> Result<Self, ProviderError> {
        let registry = Self {
            default_selection,
            mock,
            claude_code,
            codex,
            opencode,
        };
        registry.validate_selection(registry.default_selection())?;
        Ok(registry)
    }

    fn validate_selection(&self, selection: &ProviderSelection) -> Result<(), ProviderError> {
        match selection.kind() {
            ProviderKind::Mock => {
                if selection.model().is_some() {
                    return Err(ProviderError::InvalidConfiguration);
                }
                self.mock
                    .as_ref()
                    .ok_or(ProviderError::ProviderUnavailable(ProviderKind::Mock))?;
            }
            ProviderKind::ClaudeCode => {
                self.claude_code
                    .as_ref()
                    .ok_or(ProviderError::ProviderUnavailable(ProviderKind::ClaudeCode))?;
            }
            ProviderKind::Codex => {
                self.codex
                    .as_ref()
                    .ok_or(ProviderError::ProviderUnavailable(ProviderKind::Codex))?;
            }
            ProviderKind::OpenCode => {
                self.opencode
                    .as_ref()
                    .ok_or(ProviderError::ProviderUnavailable(ProviderKind::OpenCode))?
                    .validate(selection.model())?;
            }
        }
        Ok(())
    }
}

impl ProviderResolver for LocalProviderRegistry {
    fn default_selection(&self) -> &ProviderSelection {
        &self.default_selection
    }

    fn available_kinds(&self) -> Vec<ProviderKind> {
        [
            self.mock.as_ref().map(|_| ProviderKind::Mock),
            self.claude_code.as_ref().map(|_| ProviderKind::ClaudeCode),
            self.codex.as_ref().map(|_| ProviderKind::Codex),
            self.opencode.as_ref().map(|_| ProviderKind::OpenCode),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn resolve(&self, selection: &ProviderSelection) -> Result<ResolvedProvider, ProviderError> {
        let provider = match selection.kind() {
            ProviderKind::Mock => {
                if selection.model().is_some() {
                    return Err(ProviderError::InvalidConfiguration);
                }
                LocalProvider::Mock(
                    self.mock
                        .clone()
                        .ok_or(ProviderError::ProviderUnavailable(ProviderKind::Mock))?,
                )
            }
            ProviderKind::ClaudeCode => LocalProvider::ClaudeCode(
                self.claude_code
                    .as_ref()
                    .ok_or(ProviderError::ProviderUnavailable(ProviderKind::ClaudeCode))?
                    .resolve(selection.model()),
            ),
            ProviderKind::Codex => LocalProvider::Codex(
                self.codex
                    .as_ref()
                    .ok_or(ProviderError::ProviderUnavailable(ProviderKind::Codex))?
                    .resolve(selection.model()),
            ),
            ProviderKind::OpenCode => LocalProvider::OpenCode(
                self.opencode
                    .as_ref()
                    .ok_or(ProviderError::ProviderUnavailable(ProviderKind::OpenCode))?
                    .resolve(selection.model())?,
            ),
        };
        Ok(ResolvedProvider {
            selection: selection.clone(),
            provider: Arc::new(provider),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClaudeCodeProviderFactory, CodexProviderFactory};
    use std::{fs, path::Path, time::Duration};
    use tempfile::tempdir;

    fn selection(kind: ProviderKind, model: Option<&str>) -> ProviderSelection {
        ProviderSelection::new(kind, model.map(str::to_owned)).unwrap()
    }

    #[test]
    fn unavailable_selection_never_falls_back_to_mock() {
        let registry = LocalProviderRegistry::new(
            selection(ProviderKind::Mock, None),
            Some(MockProvider::deterministic()),
            None,
            None,
            None,
        )
        .unwrap();

        assert!(matches!(
            registry.resolve(&selection(ProviderKind::Codex, None)),
            Err(ProviderError::ProviderUnavailable(ProviderKind::Codex))
        ));
        assert_eq!(registry.available_kinds(), vec![ProviderKind::Mock]);
        assert!(matches!(
            registry.resolve(&selection(ProviderKind::Mock, Some("ignored-model"))),
            Err(ProviderError::InvalidConfiguration)
        ));
    }

    #[test]
    fn unavailable_default_rejects_registry_creation() {
        assert!(matches!(
            LocalProviderRegistry::new(
                selection(ProviderKind::ClaudeCode, None),
                Some(MockProvider::deterministic()),
                None,
                None,
                None,
            ),
            Err(ProviderError::ProviderUnavailable(ProviderKind::ClaudeCode))
        ));
    }

    #[cfg(unix)]
    fn write_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;

        fs::write(path, "fixture").unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn registry_resolves_exact_models_in_deterministic_order() {
        let directory = tempdir().unwrap();
        write_executable(&directory.path().join("claude"));
        write_executable(&directory.path().join("codex"));
        let search_path = directory.path().to_string_lossy();
        let claude = ClaudeCodeProviderFactory::discover(
            &search_path,
            Some("anthropic-key".to_owned()),
            &directory.path().join("claude-state"),
            Duration::from_secs(1),
        )
        .unwrap();
        let codex = CodexProviderFactory::discover(
            &search_path,
            Some("openai-key".to_owned()),
            &directory.path().join("codex-state"),
            Duration::from_secs(1),
        )
        .unwrap();
        let registry = LocalProviderRegistry::new(
            selection(ProviderKind::Mock, None),
            Some(MockProvider::deterministic()),
            Some(claude),
            Some(codex),
            None,
        )
        .unwrap();

        assert_eq!(
            registry.available_kinds(),
            vec![
                ProviderKind::Mock,
                ProviderKind::ClaudeCode,
                ProviderKind::Codex,
            ]
        );
        let first = registry
            .resolve(&selection(
                ProviderKind::ClaudeCode,
                Some("claude-sonnet-4-6"),
            ))
            .unwrap();
        let second = registry
            .resolve(&selection(
                ProviderKind::ClaudeCode,
                Some("claude-opus-4-6"),
            ))
            .unwrap();
        assert_eq!(first.selection.model(), Some("claude-sonnet-4-6"));
        assert_eq!(second.selection.model(), Some("claude-opus-4-6"));
        assert!(
            first
                .provider
                .process_spec(directory.path())
                .unwrap()
                .arguments
                .windows(2)
                .any(|arguments| arguments == ["--model", "claude-sonnet-4-6"])
        );
        assert!(
            second
                .provider
                .process_spec(directory.path())
                .unwrap()
                .arguments
                .windows(2)
                .any(|arguments| arguments == ["--model", "claude-opus-4-6"])
        );
    }
}
