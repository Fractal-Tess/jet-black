use crate::{AgentProvider, ProposedFileChange, ProviderError, common};
use execution::{ProcessResult, ProcessSpec};
use protocol::SemanticEventKind;
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Duration,
};

const PROVIDER_NAME: &str = "claude-code";
const TARGET_PATH: &str = "jet-black-claude-approved.txt";

pub struct ClaudeCodeProvider {
    executable: PathBuf,
    environment: HashMap<String, String>,
    confinement: common::ProviderConfinement,
    model: Option<String>,
    timeout: Duration,
}

impl ClaudeCodeProvider {
    pub fn discover(
        search_path: &str,
        api_key: Option<String>,
        state_dir: &Path,
        model: Option<String>,
        timeout: Duration,
    ) -> Result<Self, ProviderError> {
        if timeout.is_zero() {
            return Err(ProviderError::InvalidConfiguration);
        }
        let api_key = api_key
            .filter(|value| !value.is_empty())
            .ok_or(ProviderError::MissingCredential)?;
        let common::ProviderDiscovery {
            executable,
            mut environment,
            confinement,
        } = common::discover_provider(search_path, "claude", state_dir)?;
        environment.insert("ANTHROPIC_API_KEY".to_owned(), api_key);
        environment.insert(
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_owned(),
            "1".to_owned(),
        );
        Ok(Self {
            executable,
            environment,
            confinement,
            model: model.filter(|value| !value.is_empty()),
            timeout,
        })
    }
}

impl AgentProvider for ClaudeCodeProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn requires_process_confinement(&self) -> bool {
        true
    }

    fn process_spec(&self, worktree_path: &Path) -> Option<ProcessSpec> {
        let mut arguments = vec![
            "--bare".to_owned(),
            "--print".to_owned(),
            "--output-format".to_owned(),
            "json".to_owned(),
            "--json-schema".to_owned(),
            common::CONTENT_SCHEMA.to_owned(),
            "--permission-mode".to_owned(),
            "dontAsk".to_owned(),
            "--tools".to_owned(),
            "Read,Glob,Grep".to_owned(),
            "--disable-slash-commands".to_owned(),
            "--no-session-persistence".to_owned(),
            "--no-chrome".to_owned(),
            "--strict-mcp-config".to_owned(),
            "--mcp-config".to_owned(),
            "{}".to_owned(),
        ];
        if let Some(model) = &self.model {
            arguments.extend(["--model".to_owned(), model.clone()]);
        }
        arguments.push(common::PROMPT.to_owned());
        Some(ProcessSpec {
            program: self.executable.to_string_lossy().into_owned(),
            arguments,
            environment: self.environment.clone(),
            sensitive_environment_keys: vec!["ANTHROPIC_API_KEY".to_owned()],
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
        let response: ClaudeResponse = serde_json::from_slice(&process_result.stdout)
            .map_err(|_| ProviderError::InvalidResponse)?;
        if response.is_error {
            return Err(ProviderError::InvalidResponse);
        }
        let output = response
            .structured_output
            .ok_or(ProviderError::InvalidResponse)?;
        common::proposal_from_content(output.content, TARGET_PATH)
    }

    fn normalized_events(
        &self,
        change: &ProposedFileChange,
        digest: &str,
    ) -> Vec<SemanticEventKind> {
        common::proposal_events(
            "Claude Code prepared a read-only file proposal",
            change,
            digest,
        )
    }
}

#[derive(Deserialize)]
struct ClaudeResponse {
    is_error: bool,
    structured_output: Option<ClaudeOutput>,
}

#[derive(Deserialize)]
struct ClaudeOutput {
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use execution::{CancellationToken, TerminalOutcome};
    use std::{collections::HashSet, fs};
    use tempfile::tempdir;

    fn process_result(stdout: Vec<u8>) -> ProcessResult {
        ProcessResult {
            outcome: TerminalOutcome::Completed(0),
            stdout,
            stderr: Vec::new(),
            stdout_truncated: false,
            stderr_truncated: false,
        }
    }

    fn valid_response(content: &str) -> Vec<u8> {
        serde_json::json!({
            "is_error": false,
            "structured_output": { "content": content }
        })
        .to_string()
        .into_bytes()
    }

    #[cfg(unix)]
    fn write_executable(path: &Path, content: &str) {
        use std::os::unix::fs::PermissionsExt;

        fs::write(path, content).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn discovery_requires_safe_path_executable_credential_state_and_timeout() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempdir().unwrap();
        write_executable(&directory.path().join("claude"), "fixture");
        let search_path = directory.path().to_string_lossy();
        let state_dir = directory.path().join("state");

        assert!(matches!(
            ClaudeCodeProvider::discover(
                &search_path,
                Some("key".to_owned()),
                &state_dir,
                None,
                Duration::ZERO,
            ),
            Err(ProviderError::InvalidConfiguration)
        ));
        assert!(matches!(
            ClaudeCodeProvider::discover(
                "relative",
                Some("key".to_owned()),
                &state_dir,
                None,
                Duration::from_secs(1),
            ),
            Err(ProviderError::UnsafeSearchPath)
        ));
        assert!(matches!(
            ClaudeCodeProvider::discover(
                &search_path,
                None,
                &state_dir,
                None,
                Duration::from_secs(1),
            ),
            Err(ProviderError::MissingCredential)
        ));
        let executable = directory.path().join("claude");
        fs::write(&executable, "not executable").unwrap();
        let mut permissions = fs::metadata(&executable).unwrap().permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(&executable, permissions).unwrap();
        assert!(matches!(
            ClaudeCodeProvider::discover(
                &search_path,
                Some("key".to_owned()),
                &state_dir,
                None,
                Duration::from_secs(1),
            ),
            Err(ProviderError::ExecutableNotFound)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn process_spec_uses_fixed_arguments_and_isolated_environment() {
        let directory = tempdir().unwrap();
        let executable = directory.path().join("claude");
        write_executable(&executable, "fixture");
        let state_dir = directory.path().join("state");
        let provider = ClaudeCodeProvider::discover(
            &directory.path().to_string_lossy(),
            Some("secret-key".to_owned()),
            &state_dir,
            Some("claude-sonnet-4-6".to_owned()),
            Duration::from_secs(7),
        )
        .unwrap();
        let worktree = directory.path().join("worktree");
        let spec = provider.process_spec(&worktree).unwrap();

        assert_eq!(
            PathBuf::from(&spec.program),
            fs::canonicalize(executable).unwrap()
        );
        assert_eq!(spec.current_dir, Some(worktree));
        assert_eq!(spec.timeout, Duration::from_secs(7));
        assert_eq!(spec.output_limit, common::PROVIDER_OUTPUT_LIMIT_BYTES);
        assert_eq!(
            spec.arguments,
            vec![
                "--bare",
                "--print",
                "--output-format",
                "json",
                "--json-schema",
                common::CONTENT_SCHEMA,
                "--permission-mode",
                "dontAsk",
                "--tools",
                "Read,Glob,Grep",
                "--disable-slash-commands",
                "--no-session-persistence",
                "--no-chrome",
                "--strict-mcp-config",
                "--mcp-config",
                "{}",
                "--model",
                "claude-sonnet-4-6",
                common::PROMPT,
            ]
        );
        assert_eq!(
            spec.environment
                .keys()
                .map(String::as_str)
                .collect::<HashSet<_>>(),
            HashSet::from([
                "PATH",
                "HOME",
                "XDG_CONFIG_HOME",
                "XDG_DATA_HOME",
                "XDG_CACHE_HOME",
                "XDG_STATE_HOME",
                "TMPDIR",
                "TMP",
                "TEMP",
                "LC_ALL",
                "GIT_TERMINAL_PROMPT",
                "GIT_CONFIG_NOSYSTEM",
                "GIT_CONFIG_GLOBAL",
                "GIT_ATTR_NOSYSTEM",
                "GIT_OPTIONAL_LOCKS",
                "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
                "ANTHROPIC_API_KEY",
            ])
        );
        assert_eq!(
            spec.environment.get("HOME"),
            Some(&state_dir.to_string_lossy().into_owned())
        );
        assert_eq!(
            spec.environment
                .get("ANTHROPIC_API_KEY")
                .map(String::as_str),
            Some("secret-key")
        );
    }

    #[cfg(unix)]
    #[test]
    fn structured_output_is_normalized_and_invalid_responses_are_rejected() {
        let directory = tempdir().unwrap();
        let executable = directory.path().join("claude");
        write_executable(&executable, "fixture");
        let provider = ClaudeCodeProvider::discover(
            &directory.path().to_string_lossy(),
            Some("key".to_owned()),
            &directory.path().join("state"),
            None,
            Duration::from_secs(1),
        )
        .unwrap();
        let change = provider
            .propose(Some(&process_result(valid_response(
                "read-only proposal\n",
            ))))
            .unwrap();
        assert_eq!(change.proposal.target_path.as_str(), TARGET_PATH);
        assert_eq!(change.content, b"read-only proposal\n");
        assert!(matches!(
            provider.propose(Some(&process_result(b"not-json".to_vec()))),
            Err(ProviderError::InvalidResponse)
        ));
        assert!(matches!(
            provider.propose(Some(&process_result(
                br#"{"structured_output":{"content":"missing status"}}"#.to_vec()
            ))),
            Err(ProviderError::InvalidResponse)
        ));
        assert!(matches!(
            provider.propose(Some(&process_result(
                br#"{"is_error":true,"structured_output":null}"#.to_vec()
            ))),
            Err(ProviderError::InvalidResponse)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn fake_claude_process_runs_under_supervision() {
        let directory = tempdir().unwrap();
        use std::os::unix::fs::symlink;

        let binary_directory = directory.path().join("bin");
        let installation_directory = directory.path().join("package");
        let installation_bin = installation_directory.join("bin");
        let worktree = directory.path().join("worktree");
        fs::create_dir_all(&binary_directory).unwrap();
        fs::create_dir_all(&installation_bin).unwrap();
        fs::create_dir_all(&worktree).unwrap();
        fs::write(installation_directory.join("runtime.txt"), "runtime\n").unwrap();
        let executable = installation_bin.join("claude");
        write_executable(
            &executable,
            &format!(
                "#!/bin/sh\nIFS= read -r runtime < \"${{0%/*}}/../runtime.txt\" || exit 9\n[ \"$runtime\" = runtime ] || exit 10\nprintf '%s' '{}'\n",
                String::from_utf8(valid_response("generated read-only\n")).unwrap()
            ),
        );
        symlink(&executable, binary_directory.join("claude")).unwrap();
        let search_path = std::env::join_paths([
            directory.path().join("missing-bin"),
            binary_directory.clone(),
        ])
        .unwrap();
        let provider = ClaudeCodeProvider::discover(
            &search_path.to_string_lossy(),
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
