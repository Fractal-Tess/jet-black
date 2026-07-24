use crate::{AgentProvider, ProposedFileChange, ProviderError, common};
use domain::ProviderKind;
use execution::{ProcessResult, ProcessSpec};
use protocol::SemanticEventKind;
use serde::Deserialize;
use std::{path::Path, sync::Arc, time::Duration};

const TARGET_PATH: &str = "jet-black-codex-approved.txt";

pub struct CodexProvider {
    discovery: Arc<common::ProviderDiscovery>,
    model: Option<String>,
    timeout: Duration,
}

pub struct CodexProviderFactory {
    discovery: Arc<common::ProviderDiscovery>,
    timeout: Duration,
}

impl CodexProviderFactory {
    pub fn discover(
        search_path: &str,
        api_key: Option<String>,
        state_dir: &Path,
        timeout: Duration,
    ) -> Result<Self, ProviderError> {
        if timeout.is_zero() {
            return Err(ProviderError::InvalidConfiguration);
        }
        let api_key = api_key
            .filter(|value| !value.is_empty())
            .ok_or(ProviderError::MissingCredential)?;
        let mut discovery = common::discover_provider(search_path, "codex", state_dir)?;
        discovery
            .environment
            .insert("OPENAI_API_KEY".to_owned(), api_key);
        discovery.environment.insert(
            "CODEX_HOME".to_owned(),
            state_dir.to_string_lossy().into_owned(),
        );
        Ok(Self {
            discovery: Arc::new(discovery),
            timeout,
        })
    }

    pub(crate) fn resolve(&self, model: Option<&str>) -> CodexProvider {
        CodexProvider {
            discovery: Arc::clone(&self.discovery),
            model: model.map(str::to_owned),
            timeout: self.timeout,
        }
    }
}

impl CodexProvider {
    pub fn discover(
        search_path: &str,
        api_key: Option<String>,
        state_dir: &Path,
        model: Option<String>,
        timeout: Duration,
    ) -> Result<Self, ProviderError> {
        let model = model.filter(|value| !value.is_empty());
        Ok(
            CodexProviderFactory::discover(search_path, api_key, state_dir, timeout)?
                .resolve(model.as_deref()),
        )
    }
}

impl AgentProvider for CodexProvider {
    fn name(&self) -> &'static str {
        ProviderKind::Codex.name()
    }

    fn requires_process_confinement(&self) -> bool {
        true
    }

    fn process_spec(&self, worktree_path: &Path) -> Option<ProcessSpec> {
        let mut arguments = vec![
            "--sandbox".to_owned(),
            "read-only".to_owned(),
            "--ask-for-approval".to_owned(),
            "never".to_owned(),
            "--strict-config".to_owned(),
        ];
        if let Some(model) = &self.model {
            arguments.extend(["--model".to_owned(), model.clone()]);
        }
        arguments.extend([
            "exec".to_owned(),
            "--ephemeral".to_owned(),
            "--ignore-user-config".to_owned(),
            "--ignore-rules".to_owned(),
            "--color".to_owned(),
            "never".to_owned(),
            "--json".to_owned(),
            "--cd".to_owned(),
            worktree_path.to_string_lossy().into_owned(),
            common::PROMPT.to_owned(),
        ]);
        Some(ProcessSpec {
            program: self.discovery.executable.to_string_lossy().into_owned(),
            arguments,
            environment: self.discovery.environment.clone(),
            sensitive_environment_keys: vec!["OPENAI_API_KEY".to_owned()],
            current_dir: Some(worktree_path.to_path_buf()),
            timeout: self.timeout,
            output_limit: common::PROVIDER_OUTPUT_LIMIT_BYTES,
            confinement: self.discovery.confinement.for_worktree(worktree_path),
        })
    }

    fn propose(
        &self,
        process_result: Option<&ProcessResult>,
    ) -> Result<ProposedFileChange, ProviderError> {
        let process_result = common::require_process_result(process_result)?;
        let mut final_message = None;
        let mut completed = false;
        for line in process_result.stdout.split(|byte| *byte == b'\n') {
            if line.is_empty() {
                continue;
            }
            let event: CodexEvent =
                serde_json::from_slice(line).map_err(|_| ProviderError::InvalidResponse)?;
            match event {
                CodexEvent::ItemCompleted { item } if item.kind == "agent_message" => {
                    final_message = item.text;
                }
                CodexEvent::TurnCompleted => completed = true,
                CodexEvent::TurnFailed => return Err(ProviderError::InvalidResponse),
                _ => {}
            }
        }
        if !completed {
            return Err(ProviderError::InvalidResponse);
        }
        common::parse_content_json(
            final_message
                .as_deref()
                .ok_or(ProviderError::InvalidResponse)?,
            TARGET_PATH,
        )
    }

    fn normalized_events(
        &self,
        change: &ProposedFileChange,
        digest: &str,
    ) -> Vec<SemanticEventKind> {
        common::proposal_events("Codex prepared a read-only file proposal", change, digest)
    }
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum CodexEvent {
    #[serde(rename = "item.completed")]
    ItemCompleted { item: CodexItem },
    #[serde(rename = "turn.completed")]
    TurnCompleted,
    #[serde(rename = "turn.failed")]
    TurnFailed,
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct CodexItem {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use execution::{CancellationToken, TerminalOutcome};
    use std::fs;
    use tempfile::tempdir;

    #[cfg(unix)]
    fn write_executable(path: &Path, content: &str) {
        use std::os::unix::fs::PermissionsExt;

        fs::write(path, content).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(path, permissions).unwrap();
    }

    fn process_result(stdout: Vec<u8>) -> ProcessResult {
        ProcessResult {
            outcome: TerminalOutcome::Completed(0),
            stdout,
            stderr: Vec::new(),
            stdout_truncated: false,
            stderr_truncated: false,
        }
    }

    #[cfg(unix)]
    fn provider_fixture(directory: &Path, model: Option<String>) -> CodexProvider {
        write_executable(&directory.join("codex"), "fixture");
        CodexProvider::discover(
            &directory.to_string_lossy(),
            Some("key".to_owned()),
            &directory.join("state"),
            model,
            Duration::from_secs(2),
        )
        .unwrap()
    }

    #[cfg(unix)]
    #[test]
    fn process_spec_is_ephemeral_read_only_and_noninteractive() {
        let directory = tempdir().unwrap();
        let provider = provider_fixture(directory.path(), Some("gpt-5.3-codex".to_owned()));
        let spec = provider.process_spec(directory.path()).unwrap();
        assert_eq!(
            spec.arguments,
            vec![
                "--sandbox",
                "read-only",
                "--ask-for-approval",
                "never",
                "--strict-config",
                "--model",
                "gpt-5.3-codex",
                "exec",
                "--ephemeral",
                "--ignore-user-config",
                "--ignore-rules",
                "--color",
                "never",
                "--json",
                "--cd",
                directory.path().to_str().unwrap(),
                common::PROMPT,
            ]
        );
        assert_eq!(
            spec.environment.get("OPENAI_API_KEY").map(String::as_str),
            Some("key")
        );
        assert_eq!(spec.current_dir.as_deref(), Some(directory.path()));
    }

    #[cfg(unix)]
    #[test]
    fn jsonl_agent_message_becomes_a_bounded_proposal() {
        let directory = tempdir().unwrap();
        let provider = provider_fixture(directory.path(), None);
        let stdout = concat!(
            "{\"type\":\"thread.started\",\"thread_id\":\"thread\"}\n",
            "{\"type\":\"item.completed\",\"item\":{\"type\":\"reasoning\",\"text\":\"ignore\"}}\n",
            "{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"{\\\"content\\\":\\\"generated read-only\\\\n\\\"}\"}}\n",
            "{\"type\":\"turn.completed\"}\n"
        );
        let change = provider
            .propose(Some(&process_result(stdout.as_bytes().to_vec())))
            .unwrap();
        assert_eq!(change.proposal.target_path.as_str(), TARGET_PATH);
        assert_eq!(change.content, b"generated read-only\n");
        assert!(matches!(
            provider.propose(Some(&process_result(
                b"{\"type\":\"thread.started\"}\n".to_vec()
            ))),
            Err(ProviderError::InvalidResponse)
        ));
        assert!(matches!(
            provider.propose(Some(&process_result(b"not-json\n".to_vec()))),
            Err(ProviderError::InvalidResponse)
        ));
        for message in [
            serde_json::json!({ "content": "   " }).to_string(),
            serde_json::json!({ "content": "valid", "extra": true }).to_string(),
        ] {
            let event = serde_json::json!({
                "type": "item.completed",
                "item": { "type": "agent_message", "text": message }
            });
            assert!(matches!(
                provider.propose(Some(&process_result(
                    format!("{event}\n{{\"type\":\"turn.completed\"}}\n").into_bytes(),
                ))),
                Err(ProviderError::InvalidResponse)
            ));
        }
        let content = "x".repeat(common::PROVIDER_CONTENT_LIMIT_BYTES + 1);
        let message = serde_json::json!({ "content": content }).to_string();
        let event = serde_json::json!({
            "type": "item.completed",
            "item": { "type": "agent_message", "text": message }
        });
        assert!(matches!(
            provider.propose(Some(&process_result(
                format!("{event}\n{{\"type\":\"turn.completed\"}}\n").into_bytes(),
            ))),
            Err(ProviderError::InvalidResponse)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn fake_codex_process_completes_under_supervision() {
        let directory = tempdir().unwrap();
        let binary_directory = directory.path().join("bin");
        let worktree = directory.path().join("worktree");
        fs::create_dir_all(&binary_directory).unwrap();
        fs::create_dir_all(&worktree).unwrap();
        write_executable(
            &binary_directory.join("codex"),
            "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"{\\\"content\\\":\\\"generated read-only\\\\n\\\"}\"}}' '{\"type\":\"turn.completed\"}'\n",
        );
        let provider = CodexProvider::discover(
            &binary_directory.to_string_lossy(),
            Some("key".to_owned()),
            &directory.path().join("state"),
            None,
            Duration::from_secs(2),
        )
        .unwrap();
        let result = execution::supervise(
            &provider.process_spec(&worktree).unwrap(),
            &CancellationToken::default(),
        )
        .unwrap();
        assert_eq!(
            result.outcome,
            TerminalOutcome::Completed(0),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            provider.propose(Some(&result)).unwrap().content,
            b"generated read-only\n"
        );
    }
}
