use domain::{ActionKind, ActionProposal, Approval, ApprovalScope, RelativePath};
use uuid::Uuid;

fn scope(expires: i64) -> ApprovalScope {
    ApprovalScope {
        repository_id: Uuid::new_v4(),
        changeset_id: Uuid::new_v4(),
        base_sha: "base".into(),
        head_sha: "head".into(),
        proposal: ActionProposal {
            action: ActionKind::WriteFile,
            target_path: RelativePath::parse("safe.txt").unwrap(),
            content_sha256: "content".into(),
        },
        expires_at_unix_ms: expires,
    }
}

#[test]
fn changed_scope_expiry_and_reuse_are_rejected() {
    let original = scope(100);
    let mut approval = Approval::new(Uuid::new_v4(), original.clone());
    let mut variants = Vec::new();
    let mut changed = original.clone();
    changed.proposal.target_path = RelativePath::parse("escape.txt").unwrap();
    variants.push(changed);
    let mut changed = original.clone();
    changed.proposal.content_sha256 = "other".into();
    variants.push(changed);
    let mut changed = original.clone();
    changed.repository_id = Uuid::new_v4();
    variants.push(changed);
    let mut changed = original.clone();
    changed.changeset_id = Uuid::new_v4();
    variants.push(changed);
    let mut changed = original.clone();
    changed.base_sha = "other-base".into();
    variants.push(changed);
    let mut changed = original.clone();
    changed.head_sha = "other-head".into();
    variants.push(changed);
    for changed in variants {
        assert!(matches!(
            approval.consume_for_run(approval.run_id(), &changed, 1),
            Err(domain::DomainError::ApprovalMismatch)
        ));
    }
    assert!(matches!(
        approval.consume_for_run(Uuid::new_v4(), &original, 1),
        Err(domain::DomainError::ApprovalRunMismatch)
    ));
    assert!(matches!(
        approval.consume_for_run(approval.run_id(), &original, 100),
        Err(domain::DomainError::ApprovalExpired)
    ));
    approval
        .consume_for_run(approval.run_id(), &original, 1)
        .expect("original approval should work");
    assert!(matches!(
        approval.consume_for_run(approval.run_id(), &original, 2),
        Err(domain::DomainError::ApprovalReused)
    ));
}

#[test]
fn relative_paths_are_validated_during_deserialization() {
    assert!(serde_json::from_str::<RelativePath>(r#""safe/file.txt""#).is_ok());
    assert!(serde_json::from_str::<RelativePath>(r#""../escape.txt""#).is_err());
    assert!(serde_json::from_str::<RelativePath>(r#""C:\\escape.txt""#).is_err());
}
