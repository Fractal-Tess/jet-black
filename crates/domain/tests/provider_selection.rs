use domain::{DomainError, ProviderKind, ProviderSelection, Run, RunKind, RunState, limits};

#[test]
fn provider_kind_uses_stable_names_and_wire_values() {
    for (name, kind) in [
        ("mock", ProviderKind::Mock),
        ("claude-code", ProviderKind::ClaudeCode),
        ("codex", ProviderKind::Codex),
        ("opencode", ProviderKind::OpenCode),
    ] {
        assert_eq!(kind.name(), name);
        assert_eq!(name.parse::<ProviderKind>().unwrap(), kind);
        assert_eq!(serde_json::to_string(&kind).unwrap(), format!("\"{name}\""));
    }
    assert!(matches!(
        "unknown".parse::<ProviderKind>(),
        Err(DomainError::UnsupportedProvider(provider)) if provider == "unknown"
    ));
}

#[test]
fn provider_selection_validates_and_round_trips() {
    let selection = ProviderSelection::new(
        ProviderKind::ClaudeCode,
        Some("claude-sonnet-4-6".to_owned()),
    )
    .unwrap();
    assert_eq!(selection.kind(), ProviderKind::ClaudeCode);
    assert_eq!(selection.model(), Some("claude-sonnet-4-6"));
    assert_eq!(
        serde_json::from_str::<ProviderSelection>(&serde_json::to_string(&selection).unwrap())
            .unwrap(),
        selection
    );

    assert!(matches!(
        ProviderSelection::new(ProviderKind::Codex, Some("  ".to_owned())),
        Err(DomainError::InvalidProviderModel(_))
    ));
    assert!(matches!(
        ProviderSelection::new(
            ProviderKind::OpenCode,
            Some("x".repeat(limits::MAX_PROVIDER_MODEL_BYTES + 1)),
        ),
        Err(DomainError::InvalidProviderModel(_))
    ));
    assert!(serde_json::from_str::<ProviderSelection>(r#"{"kind":"codex","model":""}"#).is_err());
}

#[test]
fn run_keeps_provider_selection_immutable_through_transitions() {
    let selection =
        ProviderSelection::new(ProviderKind::Codex, Some("gpt-5.3-codex".to_owned())).unwrap();
    let mut run = Run::new_with_provider(uuid::Uuid::new_v4(), selection.clone());

    run.start().unwrap();
    run.running().unwrap();
    run.mark_reviewable().unwrap();
    run.complete().unwrap();

    assert_eq!(run.state(), RunState::Completed);
    assert_eq!(run.kind(), RunKind::Mutation);
    assert_eq!(run.provider_selection(), Some(&selection));
}

#[test]
fn legacy_run_without_provider_selection_remains_unattributed() {
    let run = Run::new(uuid::Uuid::new_v4());
    let mut value = serde_json::to_value(&run).unwrap();
    value.as_object_mut().unwrap().remove("provider_selection");

    let restored: Run = serde_json::from_value(value).unwrap();
    assert_eq!(restored.provider_selection(), None);
}
