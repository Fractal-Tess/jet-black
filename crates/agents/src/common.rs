use crate::{ProposedFileChange, ProviderError};
use domain::RelativePath;
use execution::ProcessResult;
use protocol::SemanticEventKind;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

pub(crate) const PROVIDER_CONTENT_LIMIT_BYTES: usize = 128 * 1024;
pub(crate) const STRUCTURED_RESPONSE_LIMIT_BYTES: usize = PROVIDER_CONTENT_LIMIT_BYTES * 8 + 1024;
pub(crate) const CONTENT_SCHEMA: &str = r#"{"type":"object","additionalProperties":false,"properties":{"content":{"type":"string","minLength":1,"maxLength":131072}},"required":["content"]}"#;
pub(crate) const PROVIDER_OUTPUT_LIMIT_BYTES: usize = 5 * 1024 * 1024;
pub(crate) const PROMPT: &str = "Inspect this repository read-only. Return JSON matching the requested schema with content for exactly one new text file. The content must briefly summarize the repository and state that the proposal was generated read-only. Do not modify files, access paths outside the repository, access the network, or include secrets.";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContentOutput {
    content: String,
}

pub(crate) struct ProviderDiscovery {
    pub executable: PathBuf,
    pub environment: HashMap<String, String>,
}

pub(crate) fn discover_provider(
    search_path: &str,
    name: &str,
    state_dir: &Path,
) -> Result<ProviderDiscovery, ProviderError> {
    if !state_dir.is_absolute() {
        return Err(ProviderError::InvalidConfiguration);
    }
    let search_path = normalize_search_path(search_path)?;
    let executable =
        resolve_named_binary(&search_path, name)?.ok_or(ProviderError::ExecutableNotFound)?;
    fs::create_dir_all(state_dir).map_err(ProviderError::DiscoveryIo)?;
    secure_directory(state_dir)?;
    let state_dir = fs::canonicalize(state_dir).map_err(ProviderError::DiscoveryIo)?;
    let environment = HashMap::from([
        ("PATH".to_owned(), search_path.clone()),
        ("HOME".to_owned(), path_string(&state_dir)?),
        (
            "XDG_CONFIG_HOME".to_owned(),
            path_string(&state_dir.join("config"))?,
        ),
        (
            "XDG_DATA_HOME".to_owned(),
            path_string(&state_dir.join("data"))?,
        ),
        (
            "XDG_CACHE_HOME".to_owned(),
            path_string(&state_dir.join("cache"))?,
        ),
        (
            "XDG_STATE_HOME".to_owned(),
            path_string(&state_dir.join("state"))?,
        ),
        ("LC_ALL".to_owned(), "C".to_owned()),
        ("GIT_TERMINAL_PROMPT".to_owned(), "0".to_owned()),
        ("GIT_CONFIG_NOSYSTEM".to_owned(), "1".to_owned()),
        ("GIT_CONFIG_GLOBAL".to_owned(), "/dev/null".to_owned()),
        ("GIT_ATTR_NOSYSTEM".to_owned(), "1".to_owned()),
        ("GIT_OPTIONAL_LOCKS".to_owned(), "0".to_owned()),
    ]);
    Ok(ProviderDiscovery {
        executable,
        environment,
    })
}

pub(crate) fn parse_content_json(
    value: &str,
    target_path: &'static str,
) -> Result<ProposedFileChange, ProviderError> {
    let output: ContentOutput =
        serde_json::from_str(value).map_err(|_| ProviderError::InvalidResponse)?;
    proposal_from_content(output.content, target_path)
}

pub(crate) fn proposal_from_content(
    content: String,
    target_path: &'static str,
) -> Result<ProposedFileChange, ProviderError> {
    if content.trim().is_empty() || content.len() > PROVIDER_CONTENT_LIMIT_BYTES {
        return Err(ProviderError::InvalidResponse);
    }
    let target_path =
        RelativePath::parse(target_path).map_err(|_| ProviderError::InvalidResponse)?;
    ProposedFileChange::write_file(target_path, content.into_bytes())
}

pub(crate) fn proposal_events(
    text: &'static str,
    change: &ProposedFileChange,
    digest: &str,
) -> Vec<SemanticEventKind> {
    vec![
        SemanticEventKind::Text {
            text: text.to_owned(),
        },
        SemanticEventKind::ActionProposal {
            proposal: change.proposal.clone(),
            digest: digest.to_owned(),
        },
    ]
}

pub(crate) fn require_process_result(
    process_result: Option<&ProcessResult>,
) -> Result<&ProcessResult, ProviderError> {
    process_result.ok_or(ProviderError::MissingProcessResult)
}

fn path_string(path: &Path) -> Result<String, ProviderError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or(ProviderError::InvalidConfiguration)
}

#[cfg(unix)]
fn secure_directory(path: &Path) -> Result<(), ProviderError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(ProviderError::DiscoveryIo)
}

#[cfg(not(unix))]
fn secure_directory(_: &Path) -> Result<(), ProviderError> {
    Ok(())
}

fn normalize_search_path(search_path: &str) -> Result<String, ProviderError> {
    let directories = std::env::split_paths(search_path).collect::<Vec<_>>();
    if directories.is_empty() || directories.iter().any(|directory| !directory.is_absolute()) {
        return Err(ProviderError::UnsafeSearchPath);
    }
    std::env::join_paths(directories)
        .map_err(|_| ProviderError::UnsafeSearchPath)?
        .into_string()
        .map_err(|_| ProviderError::UnsafeSearchPath)
}

fn resolve_named_binary(search_path: &str, name: &str) -> Result<Option<PathBuf>, ProviderError> {
    for directory in std::env::split_paths(search_path) {
        let candidate = directory.join(name);
        let metadata = match fs::metadata(&candidate) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(ProviderError::DiscoveryIo(error)),
        };
        if metadata.is_file() && is_executable(&metadata) {
            return fs::canonicalize(candidate)
                .map(Some)
                .map_err(ProviderError::DiscoveryIo);
        }
    }
    Ok(None)
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_: &fs::Metadata) -> bool {
    true
}
