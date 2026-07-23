use config::{CliOverrides, StandaloneConfig};
use domain::limits;
use std::{collections::HashMap, fs};
use tempfile::tempdir;

#[test]
fn cli_overrides_environment_and_toml() {
    let directory = tempdir().unwrap();
    let profile = directory.path().join("profile.toml");
    fs::write(
        &profile,
        "run_timeout_seconds = 10\nreview_timeout_seconds = 100\nreview_output_limit_bytes = 1024\n",
    )
    .unwrap();
    let environment = HashMap::from([
        (
            String::from("JET_BLACK_RUN_TIMEOUT_SECONDS"),
            String::from("20"),
        ),
        (
            String::from("JET_BLACK_REVIEW_TIMEOUT_SECONDS"),
            String::from("200"),
        ),
        (
            String::from("JET_BLACK_REVIEW_OUTPUT_LIMIT_BYTES"),
            String::from("2048"),
        ),
    ]);
    let root = directory.path().canonicalize().unwrap();
    let cli = CliOverrides::parse_from(vec![
        "run".into(),
        "--profile".into(),
        "standalone".into(),
        "--run-timeout-seconds".into(),
        "30".into(),
        "--review-timeout-seconds".into(),
        "300".into(),
        "--review-output-limit-bytes".into(),
        "4096".into(),
        "--allow-unsandboxed-review-checks".into(),
        "--repository-root".into(),
        root.display().to_string(),
        "--static-assets-dir".into(),
        root.display().to_string(),
    ])
    .unwrap();
    let config = StandaloneConfig::load(cli, &environment, None, Some(&profile)).unwrap();
    assert_eq!(config.run_timeout.as_secs(), 30);
    assert_eq!(config.review_timeout.as_secs(), 300);
    assert_eq!(config.review_output_limit, 4096);
    assert!(config.allow_unsandboxed_review_checks);
    assert_eq!(config.static_assets_dir, root);
    assert!(config.bind.ip().is_loopback());
    config.prepare_directories().unwrap();
    assert!(config.log_dir.exists());
}

#[test]
fn review_checks_are_untrusted_by_default() {
    let config =
        StandaloneConfig::load(CliOverrides::default(), &HashMap::new(), None, None).unwrap();

    assert_eq!(
        config.review_timeout.as_secs(),
        limits::MAX_REVIEW_TIMEOUT_SECONDS
    );
    assert_eq!(config.review_output_limit, limits::MAX_SEMANTIC_TEXT_BYTES);
    assert!(!config.allow_unsandboxed_review_checks);
}

#[test]
fn invalid_review_environment_values_fail_before_startup() {
    for (name, value) in [
        ("JET_BLACK_REVIEW_TIMEOUT_SECONDS", "invalid"),
        ("JET_BLACK_REVIEW_OUTPUT_LIMIT_BYTES", "invalid"),
        ("JET_BLACK_ALLOW_UNSANDBOXED_REVIEW_CHECKS", "invalid"),
    ] {
        let environment = HashMap::from([(name.to_owned(), value.to_owned())]);
        let error =
            StandaloneConfig::load(CliOverrides::default(), &environment, None, None).unwrap_err();
        assert!(error.to_string().contains(name));
    }
}

#[test]
fn invalid_values_fail_before_startup() {
    let cli = CliOverrides {
        bind: Some("0.0.0.0:8080".parse().unwrap()),
        repository_roots: Some(vec!["relative".into()]),
        run_timeout_seconds: Some(0),
        review_timeout_seconds: Some(limits::MAX_REVIEW_TIMEOUT_SECONDS + 1),
        review_output_limit_bytes: Some(limits::MAX_SEMANTIC_TEXT_BYTES + 1),
        ..CliOverrides::default()
    };
    let error = StandaloneConfig::load(cli, &HashMap::new(), None, None).unwrap_err();
    assert!(error.to_string().contains("loopback"));
    assert!(error.to_string().contains("absolute"));
    assert!(error.to_string().contains("review timeout"));
    assert!(error.to_string().contains("review output limit"));
}
