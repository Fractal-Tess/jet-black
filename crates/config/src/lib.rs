pub use domain::ProviderKind;
use domain::{ProviderSelection, limits};
use protocol::PROTOCOL_VERSION;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt, fs,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    time::Duration,
};
use thiserror::Error;

#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);
impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret([REDACTED])")
    }
}

#[derive(Debug, Clone, Default)]
pub struct CliOverrides {
    pub profile: Option<String>,
    pub provider: Option<ProviderKind>,
    pub provider_model: Option<String>,
    pub bind: Option<SocketAddr>,
    pub sqlite_path: Option<PathBuf>,
    pub data_dir: Option<PathBuf>,
    pub repository_roots: Option<Vec<PathBuf>>,
    pub log_dir: Option<PathBuf>,
    pub static_assets_dir: Option<PathBuf>,
    pub run_timeout_seconds: Option<u64>,
    pub review_timeout_seconds: Option<u64>,
    pub review_output_limit_bytes: Option<usize>,
    pub allow_unsandboxed_review_checks: Option<bool>,
}

impl CliOverrides {
    pub fn parse_from(arguments: impl IntoIterator<Item = String>) -> Result<Self, ConfigError> {
        let mut parsed = Self::default();
        let mut roots = Vec::new();
        let mut arguments = arguments.into_iter().peekable();
        while let Some(argument) = arguments.next() {
            let value = |arguments: &mut std::iter::Peekable<_>| {
                arguments
                    .next()
                    .ok_or_else(|| ConfigError::MissingArgumentValue(argument.clone()))
            };
            match argument.as_str() {
                "run" => {}
                "--profile" => {
                    let profile = value(&mut arguments)?;
                    if profile != "standalone" {
                        return Err(ConfigError::UnsupportedProfile(profile));
                    }
                    parsed.profile = Some(profile);
                }
                "--provider" => {
                    parsed.provider = Some(parse_provider(&value(&mut arguments)?)?);
                }
                "--provider-model" => {
                    parsed.provider_model = Some(value(&mut arguments)?);
                }
                "--bind" => {
                    parsed.bind = Some(
                        value(&mut arguments)?
                            .parse()
                            .map_err(|_| ConfigError::InvalidCliValue(argument.clone()))?,
                    )
                }
                "--sqlite-path" => parsed.sqlite_path = Some(PathBuf::from(value(&mut arguments)?)),
                "--data-dir" => parsed.data_dir = Some(PathBuf::from(value(&mut arguments)?)),
                "--repository-root" => roots.push(PathBuf::from(value(&mut arguments)?)),
                "--log-dir" => parsed.log_dir = Some(PathBuf::from(value(&mut arguments)?)),
                "--static-assets-dir" => {
                    parsed.static_assets_dir = Some(PathBuf::from(value(&mut arguments)?))
                }
                "--run-timeout-seconds" => {
                    parsed.run_timeout_seconds = Some(
                        value(&mut arguments)?
                            .parse()
                            .map_err(|_| ConfigError::InvalidCliValue(argument.clone()))?,
                    )
                }
                "--review-timeout-seconds" => {
                    parsed.review_timeout_seconds = Some(
                        value(&mut arguments)?
                            .parse()
                            .map_err(|_| ConfigError::InvalidCliValue(argument.clone()))?,
                    )
                }
                "--review-output-limit-bytes" => {
                    parsed.review_output_limit_bytes = Some(
                        value(&mut arguments)?
                            .parse()
                            .map_err(|_| ConfigError::InvalidCliValue(argument.clone()))?,
                    )
                }
                "--allow-unsandboxed-review-checks" => {
                    parsed.allow_unsandboxed_review_checks = Some(true)
                }
                _ => return Err(ConfigError::UnknownArgument(argument)),
            }
        }
        if !roots.is_empty() {
            parsed.repository_roots = Some(roots);
        }
        Ok(parsed)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
struct FileConfig {
    provider: Option<ProviderKind>,
    provider_model: Option<String>,
    bind: Option<SocketAddr>,
    sqlite_path: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    repository_roots: Option<Vec<PathBuf>>,
    log_dir: Option<PathBuf>,
    static_assets_dir: Option<PathBuf>,
    run_timeout_seconds: Option<u64>,
    review_timeout_seconds: Option<u64>,
    review_output_limit_bytes: Option<usize>,
    allow_unsandboxed_review_checks: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct StandaloneConfig {
    pub provider: ProviderKind,
    pub provider_model: Option<String>,
    pub bind: SocketAddr,
    pub sqlite_path: PathBuf,
    pub data_dir: PathBuf,
    pub repository_roots: Vec<PathBuf>,
    pub log_dir: PathBuf,
    pub static_assets_dir: PathBuf,
    pub run_timeout: Duration,
    pub review_timeout: Duration,
    pub review_output_limit: usize,
    pub allow_unsandboxed_review_checks: bool,
}

impl StandaloneConfig {
    pub fn load(
        cli: CliOverrides,
        env: &HashMap<String, String>,
        local_toml: Option<&Path>,
        profile_toml: Option<&Path>,
    ) -> Result<Self, ConfigError> {
        let profile = cli
            .profile
            .as_deref()
            .or_else(|| env.get("JET_BLACK_PROFILE").map(String::as_str))
            .unwrap_or("standalone");
        if profile != "standalone" {
            return Err(ConfigError::UnsupportedProfile(profile.to_owned()));
        }
        let home = default_data_dir();
        let static_assets_dir = default_static_assets_dir()?;
        let mut raw = FileConfig {
            provider: Some(ProviderKind::Mock),
            provider_model: None,
            bind: Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4317)),
            sqlite_path: Some(home.join("jet-black.sqlite3")),
            data_dir: Some(home.clone()),
            repository_roots: Some(Vec::new()),
            log_dir: Some(home.join("logs")),
            static_assets_dir: Some(static_assets_dir),
            run_timeout_seconds: Some(900),
            review_timeout_seconds: Some(limits::MAX_REVIEW_TIMEOUT_SECONDS),
            review_output_limit_bytes: Some(limits::MAX_SEMANTIC_TEXT_BYTES),
            allow_unsandboxed_review_checks: Some(false),
        };
        merge_file(&mut raw, profile_toml)?;
        merge_file(&mut raw, local_toml)?;
        merge_env(&mut raw, env)?;
        merge_cli(&mut raw, cli);
        let config = Self {
            provider: raw.provider.expect("safe default"),
            provider_model: raw.provider_model,
            bind: raw.bind.expect("safe default"),
            sqlite_path: raw.sqlite_path.expect("safe default"),
            data_dir: raw.data_dir.expect("safe default"),
            repository_roots: raw.repository_roots.expect("safe default"),
            log_dir: raw.log_dir.expect("safe default"),
            static_assets_dir: raw.static_assets_dir.expect("safe default"),
            run_timeout: Duration::from_secs(raw.run_timeout_seconds.expect("safe default")),
            review_timeout: Duration::from_secs(raw.review_timeout_seconds.expect("safe default")),
            review_output_limit: raw.review_output_limit_bytes.expect("safe default"),
            allow_unsandboxed_review_checks: raw
                .allow_unsandboxed_review_checks
                .expect("safe default"),
        };
        config.validate()?;
        Ok(config)
    }

    pub fn prepare_directories(&self) -> Result<(), ConfigError> {
        fs::create_dir_all(&self.data_dir)?;
        fs::create_dir_all(&self.log_dir)?;
        if let Some(parent) = self.sqlite_path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), ConfigError> {
        let mut errors = Vec::new();
        if !self.bind.ip().is_loopback() {
            errors.push("standalone bind address must be loopback".to_owned());
        }
        if self.run_timeout.is_zero() {
            errors.push("run timeout must be greater than zero".to_owned());
        }
        if self.review_timeout.is_zero()
            || self.review_timeout > Duration::from_secs(limits::MAX_REVIEW_TIMEOUT_SECONDS)
        {
            errors.push(format!(
                "review timeout must be between 1 and {} seconds",
                limits::MAX_REVIEW_TIMEOUT_SECONDS
            ));
        }
        if self.review_output_limit == 0
            || self.review_output_limit > limits::MAX_SEMANTIC_TEXT_BYTES
        {
            errors.push(format!(
                "review output limit must be between 1 and {} bytes",
                limits::MAX_SEMANTIC_TEXT_BYTES
            ));
        }
        if let Err(error) = ProviderSelection::new(self.provider, self.provider_model.clone()) {
            errors.push(error.to_string());
        }
        for root in &self.repository_roots {
            if !root.is_absolute() {
                errors.push(format!(
                    "repository root must be absolute: {}",
                    root.display()
                ));
            }
        }
        if !self.static_assets_dir.is_absolute() {
            errors.push(format!(
                "static assets directory must be absolute: {}",
                self.static_assets_dir.display()
            ));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(ConfigError::Validation(errors))
        }
    }

    pub fn public_bootstrap(&self, provider_availability: Vec<ProviderKind>) -> PublicBootstrap {
        PublicBootstrap {
            profile: "standalone".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            protocol_version: PROTOCOL_VERSION.to_owned(),
            enabled_features: vec!["local_execution".to_owned()],
            provider_availability,
            default_provider: ProviderSelection::new(self.provider, self.provider_model.clone())
                .expect("validated standalone provider selection must remain valid"),
        }
    }
}

fn default_static_assets_dir() -> Result<PathBuf, ConfigError> {
    Ok(std::env::current_dir()?.join("apps/web/build-standalone"))
}

fn default_data_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(path).join("jet-black");
    }
    if let Some(path) = std::env::var_os("HOME") {
        return PathBuf::from(path)
            .join(".local")
            .join("share")
            .join("jet-black");
    }
    std::env::temp_dir().join("jet-black")
}

fn merge_file(raw: &mut FileConfig, path: Option<&Path>) -> Result<(), ConfigError> {
    let Some(path) = path else {
        return Ok(());
    };
    let parsed: FileConfig = toml::from_str(&fs::read_to_string(path)?)?;
    merge(raw, parsed);
    Ok(())
}
fn parse_provider(value: &str) -> Result<ProviderKind, ConfigError> {
    value
        .parse()
        .map_err(|_| ConfigError::UnsupportedProvider(value.to_owned()))
}

fn merge_env(raw: &mut FileConfig, env: &HashMap<String, String>) -> Result<(), ConfigError> {
    let parsed = FileConfig {
        provider: env
            .get("JET_BLACK_PROVIDER")
            .map(|value| parse_provider(value))
            .transpose()?,
        provider_model: env.get("JET_BLACK_PROVIDER_MODEL").cloned(),
        bind: env
            .get("JET_BLACK_BIND")
            .map(|v| v.parse())
            .transpose()
            .map_err(|_| ConfigError::InvalidEnvironment("JET_BLACK_BIND"))?,
        sqlite_path: env.get("JET_BLACK_SQLITE_PATH").map(PathBuf::from),
        data_dir: env.get("JET_BLACK_DATA_DIR").map(PathBuf::from),
        repository_roots: env
            .get("JET_BLACK_REPOSITORY_ROOTS")
            .map(|v| v.split(':').map(PathBuf::from).collect()),
        log_dir: env.get("JET_BLACK_LOG_DIR").map(PathBuf::from),
        static_assets_dir: env.get("JET_BLACK_STATIC_ASSETS_DIR").map(PathBuf::from),
        run_timeout_seconds: env
            .get("JET_BLACK_RUN_TIMEOUT_SECONDS")
            .map(|v| v.parse())
            .transpose()
            .map_err(|_| ConfigError::InvalidEnvironment("JET_BLACK_RUN_TIMEOUT_SECONDS"))?,
        review_timeout_seconds: env
            .get("JET_BLACK_REVIEW_TIMEOUT_SECONDS")
            .map(|v| v.parse())
            .transpose()
            .map_err(|_| ConfigError::InvalidEnvironment("JET_BLACK_REVIEW_TIMEOUT_SECONDS"))?,
        review_output_limit_bytes: env
            .get("JET_BLACK_REVIEW_OUTPUT_LIMIT_BYTES")
            .map(|v| v.parse())
            .transpose()
            .map_err(|_| ConfigError::InvalidEnvironment("JET_BLACK_REVIEW_OUTPUT_LIMIT_BYTES"))?,
        allow_unsandboxed_review_checks: env
            .get("JET_BLACK_ALLOW_UNSANDBOXED_REVIEW_CHECKS")
            .map(|v| v.parse())
            .transpose()
            .map_err(|_| {
                ConfigError::InvalidEnvironment("JET_BLACK_ALLOW_UNSANDBOXED_REVIEW_CHECKS")
            })?,
    };
    merge(raw, parsed);
    Ok(())
}
fn merge_cli(raw: &mut FileConfig, cli: CliOverrides) {
    merge(
        raw,
        FileConfig {
            provider: cli.provider,
            provider_model: cli.provider_model,
            bind: cli.bind,
            sqlite_path: cli.sqlite_path,
            data_dir: cli.data_dir,
            repository_roots: cli.repository_roots,
            log_dir: cli.log_dir,
            static_assets_dir: cli.static_assets_dir,
            run_timeout_seconds: cli.run_timeout_seconds,
            review_timeout_seconds: cli.review_timeout_seconds,
            review_output_limit_bytes: cli.review_output_limit_bytes,
            allow_unsandboxed_review_checks: cli.allow_unsandboxed_review_checks,
        },
    );
}
fn merge(target: &mut FileConfig, source: FileConfig) {
    if source.provider.is_some() {
        target.provider = source.provider;
    }
    if source.provider_model.is_some() {
        target.provider_model = source.provider_model;
    }
    if source.bind.is_some() {
        target.bind = source.bind;
    }
    if source.sqlite_path.is_some() {
        target.sqlite_path = source.sqlite_path;
    }
    if source.data_dir.is_some() {
        target.data_dir = source.data_dir;
    }
    if source.repository_roots.is_some() {
        target.repository_roots = source.repository_roots;
    }
    if source.log_dir.is_some() {
        target.log_dir = source.log_dir;
    }
    if source.static_assets_dir.is_some() {
        target.static_assets_dir = source.static_assets_dir;
    }
    if source.run_timeout_seconds.is_some() {
        target.run_timeout_seconds = source.run_timeout_seconds;
    }
    if source.review_timeout_seconds.is_some() {
        target.review_timeout_seconds = source.review_timeout_seconds;
    }
    if source.review_output_limit_bytes.is_some() {
        target.review_output_limit_bytes = source.review_output_limit_bytes;
    }
    if source.allow_unsandboxed_review_checks.is_some() {
        target.allow_unsandboxed_review_checks = source.allow_unsandboxed_review_checks;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PublicBootstrap {
    pub profile: String,
    pub version: String,
    pub protocol_version: String,
    pub enabled_features: Vec<String>,
    pub provider_availability: Vec<ProviderKind>,
    pub default_provider: ProviderSelection,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("configuration validation failed: {0:?}")]
    Validation(Vec<String>),
    #[error("invalid environment value for {0}")]
    InvalidEnvironment(&'static str),
    #[error("configuration I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid TOML configuration: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("unknown command-line argument: {0}")]
    UnknownArgument(String),
    #[error("missing value for command-line argument: {0}")]
    MissingArgumentValue(String),
    #[error("invalid command-line value for {0}")]
    InvalidCliValue(String),
    #[error("unsupported runtime profile: {0}")]
    UnsupportedProfile(String),
    #[error("unsupported local provider: {0}")]
    UnsupportedProvider(String),
}
