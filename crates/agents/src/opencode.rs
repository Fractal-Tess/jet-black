use crate::{AgentProvider, ProposedFileChange, ProviderError, common};
use execution::{ProcessResult, ProcessSpec};
use protocol::SemanticEventKind;
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Duration,
};

const PROVIDER_NAME: &str = "opencode";
const TARGET_PATH: &str = "jet-black-opencode-approved.txt";
const CONFIG: &str = r#"{"$schema":"https://opencode.ai/config.json","autoupdate":false,"share":"disabled","permission":{"edit":"deny","bash":"deny","webfetch":"deny","external_directory":"deny","doom_loop":"deny"}}"#;
const PROVIDER_CREDENTIALS: &[(&str, &[&str])] = &[
    ("anthropic", &["ANTHROPIC_API_KEY"]),
    (
        "google",
        &["GEMINI_API_KEY", "GOOGLE_GENERATIVE_AI_API_KEY"],
    ),
    ("opencode", &["OPENCODE_API_KEY"]),
    ("openai", &["OPENAI_API_KEY"]),
    ("openrouter", &["OPENROUTER_API_KEY"]),
];

pub struct OpenCodeProvider {
    executable: PathBuf,
    environment: HashMap<String, String>,
    confinement: common::ProviderConfinement,
    credential_name: String,
    model: Option<String>,
    timeout: Duration,
}

impl OpenCodeProvider {
    pub fn discover(
        search_path: &str,
        credentials: &HashMap<String, String>,
        state_dir: &Path,
        model: Option<String>,
        timeout: Duration,
    ) -> Result<Self, ProviderError> {
        if timeout.is_zero() {
            return Err(ProviderError::InvalidConfiguration);
        }
        let model = model.filter(|value| !value.is_empty());
        let (credential_name, credential) = select_credential(credentials, model.as_deref())?;
        let common::ProviderDiscovery {
            executable,
            mut environment,
            confinement,
        } = common::discover_provider(search_path, "opencode", state_dir)?;
        environment.insert(credential_name.clone(), credential);
        environment.insert("OPENCODE_CONFIG_CONTENT".to_owned(), CONFIG.to_owned());
        Ok(Self {
            executable,
            environment,
            confinement,
            credential_name,
            model,
            timeout,
        })
    }
}

fn select_credential(
    credentials: &HashMap<String, String>,
    model: Option<&str>,
) -> Result<(String, String), ProviderError> {
    if let Some(model) = model {
        let provider = model
            .split_once('/')
            .map(|(provider, _)| provider)
            .ok_or(ProviderError::InvalidConfiguration)?;
        let (_, names) = PROVIDER_CREDENTIALS
            .iter()
            .find(|(candidate, _)| *candidate == provider)
            .ok_or(ProviderError::InvalidConfiguration)?;
        return credential_for_names(credentials, names).ok_or(ProviderError::MissingCredential);
    }

    let available = PROVIDER_CREDENTIALS
        .iter()
        .filter_map(|(_, names)| credential_for_names(credentials, names))
        .collect::<Vec<_>>();
    match available.as_slice() {
        [] => Err(ProviderError::MissingCredential),
        [credential] => Ok(credential.clone()),
        _ => Err(ProviderError::InvalidConfiguration),
    }
}

fn credential_for_names(
    credentials: &HashMap<String, String>,
    names: &[&str],
) -> Option<(String, String)> {
    names.iter().find_map(|name| {
        credentials
            .get(*name)
            .filter(|value| !value.is_empty())
            .map(|value| ((*name).to_owned(), value.clone()))
    })
}

impl AgentProvider for OpenCodeProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn requires_process_confinement(&self) -> bool {
        true
    }

    fn process_spec(&self, worktree_path: &Path) -> Option<ProcessSpec> {
        let mut arguments = vec![
            "run".to_owned(),
            "--pure".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            "--dir".to_owned(),
            worktree_path.to_string_lossy().into_owned(),
        ];
        if let Some(model) = &self.model {
            arguments.extend(["--model".to_owned(), model.clone()]);
        }
        arguments.push(common::PROMPT.to_owned());
        Some(ProcessSpec {
            program: self.executable.to_string_lossy().into_owned(),
            arguments,
            environment: self.environment.clone(),
            sensitive_environment_keys: vec![self.credential_name.clone()],
            current_dir: Some(worktree_path.to_path_buf()),
            timeout: self.timeout,
            output_limit: common::PROVIDER_OUTPUT_LIMIT_BYTES,
            confinement: self.confinement.for_worktree(worktree_path),
        })
    }

    fn propose(
        &self,
        process_result: Option<&ProcessResult>,
    ) -> Result<ProposedFileChange, ProviderError> {
        let process_result = common::require_process_result(process_result)?;
        let mut text = String::new();
        let mut completed = false;
        for line in process_result.stdout.split(|byte| *byte == b'\n') {
            if line.is_empty() {
                continue;
            }
            let event: OpenCodeEvent =
                serde_json::from_slice(line).map_err(|_| ProviderError::InvalidResponse)?;
            match event.kind.as_str() {
                "error" => return Err(ProviderError::InvalidResponse),
                "step_finish" => completed = true,
                "text" => {
                    if let Some(text_part) = event.part.and_then(|part| part.text) {
                        if text.len().saturating_add(text_part.len())
                            > common::STRUCTURED_RESPONSE_LIMIT_BYTES
                        {
                            return Err(ProviderError::InvalidResponse);
                        }
                        text.push_str(&text_part);
                    }
                }
                _ => {}
            }
        }
        if !completed || text.is_empty() {
            return Err(ProviderError::InvalidResponse);
        }
        common::parse_content_json(&text, TARGET_PATH)
    }

    fn normalized_events(
        &self,
        change: &ProposedFileChange,
        digest: &str,
    ) -> Vec<SemanticEventKind> {
        common::proposal_events(
            "OpenCode prepared a read-only file proposal",
            change,
            digest,
        )
    }
}

#[derive(Deserialize)]
struct OpenCodeEvent {
    #[serde(rename = "type")]
    kind: String,
    part: Option<OpenCodePart>,
}

#[derive(Deserialize)]
struct OpenCodePart {
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
    fn provider_fixture(directory: &Path, model: Option<String>) -> OpenCodeProvider {
        write_executable(&directory.join("opencode"), "fixture");
        OpenCodeProvider::discover(
            &directory.to_string_lossy(),
            &HashMap::from([("OPENAI_API_KEY".to_owned(), "key".to_owned())]),
            &directory.join("state"),
            model,
            Duration::from_secs(2),
        )
        .unwrap()
    }

    #[cfg(unix)]
    #[test]
    fn process_spec_is_pure_permission_restricted_and_model_aware() {
        let directory = tempdir().unwrap();
        let provider = provider_fixture(directory.path(), Some("openai/gpt-5.2-codex".to_owned()));
        let spec = provider.process_spec(directory.path()).unwrap();
        assert_eq!(
            spec.arguments,
            vec![
                "run",
                "--pure",
                "--format",
                "json",
                "--dir",
                directory.path().to_str().unwrap(),
                "--model",
                "openai/gpt-5.2-codex",
                common::PROMPT,
            ]
        );
        assert_eq!(
            spec.environment
                .get("OPENCODE_CONFIG_CONTENT")
                .map(String::as_str),
            Some(CONFIG)
        );
        assert_eq!(
            spec.environment.get("OPENAI_API_KEY").map(String::as_str),
            Some("key")
        );
    }

    #[cfg(unix)]
    #[test]
    fn credential_selection_is_model_scoped_and_cannot_override_isolation() {
        let directory = tempdir().unwrap();
        write_executable(&directory.path().join("opencode"), "fixture");
        let state_dir = directory.path().join("state");
        let credentials = HashMap::from([
            ("ANTHROPIC_API_KEY".to_owned(), "anthropic-key".to_owned()),
            ("HOME".to_owned(), "/unsafe".to_owned()),
            ("OPENAI_API_KEY".to_owned(), "openai-key".to_owned()),
        ]);
        let provider = OpenCodeProvider::discover(
            &directory.path().to_string_lossy(),
            &credentials,
            &state_dir,
            Some("openai/gpt-5.2-codex".to_owned()),
            Duration::from_secs(2),
        )
        .unwrap();
        let spec = provider.process_spec(directory.path()).unwrap();

        assert_eq!(
            spec.environment.get("OPENAI_API_KEY").map(String::as_str),
            Some("openai-key")
        );
        assert!(!spec.environment.contains_key("ANTHROPIC_API_KEY"));
        assert_eq!(
            spec.environment.get("HOME"),
            Some(
                &fs::canonicalize(state_dir)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            )
        );
        assert!(matches!(
            OpenCodeProvider::discover(
                &directory.path().to_string_lossy(),
                &credentials,
                &directory.path().join("other-state"),
                None,
                Duration::from_secs(2),
            ),
            Err(ProviderError::InvalidConfiguration)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn json_text_events_become_a_bounded_proposal() {
        let directory = tempdir().unwrap();
        let provider = provider_fixture(directory.path(), None);
        let stdout = concat!(
            "{\"type\":\"step_start\",\"part\":{\"id\":\"step\"}}\n",
            "{\"type\":\"text\",\"part\":{\"text\":\"{\\\"content\\\":\"}}\n",
            "{\"type\":\"text\",\"part\":{\"text\":\"\\\"generated read-only\\\\n\\\"}\"}}\n",
            "{\"type\":\"step_finish\",\"part\":{\"reason\":\"stop\"}}\n"
        );
        let change = provider
            .propose(Some(&process_result(stdout.as_bytes().to_vec())))
            .unwrap();
        assert_eq!(change.proposal.target_path.as_str(), TARGET_PATH);
        assert_eq!(change.content, b"generated read-only\n");
        assert!(matches!(
            provider.propose(Some(&process_result(
                b"{\"type\":\"step_start\"}\n".to_vec()
            ))),
            Err(ProviderError::InvalidResponse)
        ));
        assert!(matches!(
            provider.propose(Some(&process_result(b"not-json\n".to_vec()))),
            Err(ProviderError::InvalidResponse)
        ));
        let content = "x".repeat(common::PROVIDER_CONTENT_LIMIT_BYTES + 1);
        let text = serde_json::json!({ "content": content }).to_string();
        let event = serde_json::json!({
            "type": "text",
            "part": { "text": text }
        });
        assert!(matches!(
            provider.propose(Some(&process_result(
                format!("{event}\n{{\"type\":\"step_finish\"}}\n").into_bytes(),
            ))),
            Err(ProviderError::InvalidResponse)
        ));
        assert!(matches!(
            provider.propose(Some(&process_result(
                b"{\"type\":\"error\",\"message\":\"failed\"}\n".to_vec()
            ))),
            Err(ProviderError::InvalidResponse)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn fake_opencode_process_completes_under_supervision() {
        let directory = tempdir().unwrap();
        let binary_directory = directory.path().join("bin");
        let worktree = directory.path().join("worktree");
        fs::create_dir_all(&binary_directory).unwrap();
        fs::create_dir_all(&worktree).unwrap();
        write_executable(
            &binary_directory.join("opencode"),
            "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"text\",\"part\":{\"text\":\"{\\\"content\\\":\\\"generated read-only\\\\n\\\"}\"}}' '{\"type\":\"step_finish\",\"part\":{\"reason\":\"stop\"}}'\n",
        );
        let provider = OpenCodeProvider::discover(
            &binary_directory.to_string_lossy(),
            &HashMap::from([("OPENAI_API_KEY".to_owned(), "key".to_owned())]),
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
