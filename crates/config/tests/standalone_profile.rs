use config::{CliOverrides, StandaloneConfig};
use std::{collections::HashMap, fs};
use tempfile::tempdir;

#[test]
fn cli_overrides_environment_and_toml() {
    let directory = tempdir().unwrap();
    let profile = directory.path().join("profile.toml");
    fs::write(&profile, "run_timeout_seconds = 10\n").unwrap();
    let environment = HashMap::from([(
        String::from("JET_BLACK_RUN_TIMEOUT_SECONDS"),
        String::from("20"),
    )]);
    let root = directory.path().canonicalize().unwrap();
    let cli = CliOverrides::parse_from(vec![
        "run".into(),
        "--profile".into(),
        "standalone".into(),
        "--run-timeout-seconds".into(),
        "30".into(),
        "--repository-root".into(),
        root.display().to_string(),
    ])
    .unwrap();
    let config = StandaloneConfig::load(cli, &environment, None, Some(&profile)).unwrap();
    assert_eq!(config.run_timeout.as_secs(), 30);
    assert!(config.bind.ip().is_loopback());
    config.prepare_directories().unwrap();
    assert!(config.log_dir.exists());
}

#[test]
fn invalid_values_fail_before_startup() {
    let cli = CliOverrides {
        bind: Some("0.0.0.0:8080".parse().unwrap()),
        repository_roots: Some(vec!["relative".into()]),
        run_timeout_seconds: Some(0),
        ..CliOverrides::default()
    };
    let error = StandaloneConfig::load(cli, &HashMap::new(), None, None).unwrap_err();
    assert!(error.to_string().contains("loopback"));
    assert!(error.to_string().contains("absolute"));
}
