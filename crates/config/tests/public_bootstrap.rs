use config::{CliOverrides, ProviderKind, Secret, StandaloneConfig};
use std::{collections::HashMap, fs};
use tempfile::tempdir;

#[test]
fn bootstrap_excludes_private_paths_and_secrets() {
    let config =
        StandaloneConfig::load(CliOverrides::default(), &HashMap::new(), None, None).unwrap();
    let json = serde_json::to_string(&config.public_bootstrap(vec![ProviderKind::Mock])).unwrap();
    assert!(json.contains("standalone"));
    assert!(json.contains("0.1"));
    assert!(!json.contains("sqlite"));
    assert!(!json.contains("repository_roots"));
    assert!(!json.contains("review_timeout"));
    assert!(!json.contains("review_output_limit"));
    assert!(!json.contains("allow_unsandboxed"));
    assert!(!json.contains("secret"));
    assert_eq!(
        format!("{:?}", Secret::new("top-secret")),
        "Secret([REDACTED])"
    );
}

#[test]
fn provider_defaults_to_mock_and_supports_environment_and_cli_overrides() {
    let default =
        StandaloneConfig::load(CliOverrides::default(), &HashMap::new(), None, None).unwrap();
    assert_eq!(default.provider, ProviderKind::Mock);
    assert_eq!(default.provider_model, None);

    let environment = HashMap::from([
        ("JET_BLACK_PROVIDER".to_owned(), "opencode".to_owned()),
        (
            "JET_BLACK_PROVIDER_MODEL".to_owned(),
            "openai/gpt-5.2-codex".to_owned(),
        ),
    ]);
    let environment_config =
        StandaloneConfig::load(CliOverrides::default(), &environment, None, None).unwrap();
    assert_eq!(environment_config.provider, ProviderKind::OpenCode);
    assert_eq!(
        environment_config.provider_model.as_deref(),
        Some("openai/gpt-5.2-codex")
    );

    let cli = CliOverrides::parse_from([
        "run".to_owned(),
        "--provider".to_owned(),
        "codex".to_owned(),
        "--provider-model".to_owned(),
        "openai/gpt-5.3-codex".to_owned(),
    ])
    .unwrap();
    let cli_config = StandaloneConfig::load(cli, &environment, None, None).unwrap();
    assert_eq!(cli_config.provider, ProviderKind::Codex);
    assert_eq!(
        cli_config.provider_model.as_deref(),
        Some("openai/gpt-5.3-codex")
    );
}

#[test]
fn provider_and_model_follow_file_environment_and_cli_precedence() {
    let directory = tempdir().unwrap();
    let profile = directory.path().join("profile.toml");
    let local = directory.path().join("local.toml");
    fs::write(
        &profile,
        "provider = \"claude-code\"\nprovider_model = \"profile-model\"\n",
    )
    .unwrap();
    fs::write(
        &local,
        "provider = \"codex\"\nprovider_model = \"local-model\"\n",
    )
    .unwrap();
    let environment = HashMap::from([
        ("JET_BLACK_PROVIDER".to_owned(), "opencode".to_owned()),
        (
            "JET_BLACK_PROVIDER_MODEL".to_owned(),
            "environment-model".to_owned(),
        ),
    ]);
    let cli = CliOverrides::parse_from([
        "run".to_owned(),
        "--provider".to_owned(),
        "mock".to_owned(),
        "--provider-model".to_owned(),
        "cli-model".to_owned(),
    ])
    .unwrap();

    let config = StandaloneConfig::load(cli, &environment, Some(&local), Some(&profile)).unwrap();

    assert_eq!(config.provider, ProviderKind::Mock);
    assert_eq!(config.provider_model.as_deref(), Some("cli-model"));
}

#[test]
fn blank_provider_model_is_rejected() {
    let environment = HashMap::from([("JET_BLACK_PROVIDER_MODEL".to_owned(), "   ".to_owned())]);
    let error =
        StandaloneConfig::load(CliOverrides::default(), &environment, None, None).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("provider model must not be blank")
    );
}

#[test]
fn unsupported_provider_is_rejected() {
    let error = CliOverrides::parse_from([
        "run".to_owned(),
        "--provider".to_owned(),
        "other".to_owned(),
    ])
    .unwrap_err();
    assert!(error.to_string().contains("unsupported local provider"));
}
