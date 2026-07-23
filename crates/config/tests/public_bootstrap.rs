use config::{CliOverrides, Secret, StandaloneConfig};
use std::collections::HashMap;

#[test]
fn bootstrap_excludes_private_paths_and_secrets() {
    let config =
        StandaloneConfig::load(CliOverrides::default(), &HashMap::new(), None, None).unwrap();
    let json = serde_json::to_string(&config.public_bootstrap()).unwrap();
    assert!(json.contains("standalone"));
    assert!(json.contains("0.1"));
    assert!(!json.contains("sqlite"));
    assert!(!json.contains("repository_roots"));
    assert!(!json.contains("secret"));
    assert_eq!(
        format!("{:?}", Secret::new("top-secret")),
        "Secret([REDACTED])"
    );
}
