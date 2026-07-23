use domain::{
    ActionKind, ActionProposal, Approval, ApprovalScope, Changeset, ChangesetState, Finding,
    FindingState, RelativePath, Run, RunState, Worktree, WorktreeState,
};
use std::path::PathBuf;

#[test]
fn lifecycle_methods_are_the_only_valid_transition_path() {
    let mut changeset = Changeset::new(uuid::Uuid::new_v4(), "base".into());
    assert!(changeset.commit().is_err());
    changeset.activate().unwrap();
    assert_eq!(changeset.state(), ChangesetState::Active);
    let mut failed_changeset = changeset.clone();
    failed_changeset.fail().unwrap();
    assert_eq!(failed_changeset.state(), ChangesetState::Failed);
    let mut reviewable_changeset = changeset.clone();
    reviewable_changeset.mark_reviewable("head".into()).unwrap();
    reviewable_changeset.fail().unwrap();
    assert_eq!(reviewable_changeset.state(), ChangesetState::Failed);
    let mut run = Run::new(changeset.id);
    assert!(run.running().is_err());
    run.start().unwrap();
    run.running().unwrap();
    assert!(run.await_approval(String::new()).is_err());
    run.await_approval("digest".into()).unwrap();
    assert_eq!(run.state(), RunState::AwaitingApproval);
    let scope = ApprovalScope {
        repository_id: uuid::Uuid::new_v4(),
        changeset_id: changeset.id,
        base_sha: "base".into(),
        head_sha: "head".into(),
        proposal: ActionProposal {
            action: ActionKind::WriteFile,
            target_path: RelativePath::parse("file").unwrap(),
            content_sha256: "sha".into(),
        },
        expires_at_unix_ms: 100,
    };
    let mut approval = Approval::new(run.id, scope.clone());
    assert!(
        approval
            .consume_for_run(uuid::Uuid::new_v4(), &scope, 1)
            .is_err()
    );
    let mut finding = Finding::new(
        changeset.id,
        Some(RelativePath::parse("file").unwrap()),
        "blob".into(),
        "quality".into(),
        "low".into(),
        "message".into(),
        "evidence".into(),
    );
    finding.dismiss().unwrap();
    assert_eq!(finding.state(), FindingState::Dismissed);
    let mut worktree = Worktree::creating(
        uuid::Uuid::new_v4(),
        changeset.id,
        PathBuf::from("worktree"),
        "filesystem".into(),
        "base".into(),
    );
    worktree.ready().unwrap();
    worktree.begin_removal().unwrap();
    worktree.removed().unwrap();
    assert_eq!(worktree.state(), WorktreeState::Removed);
    let worktree_version = worktree.version();
    worktree.removed().unwrap();
    assert_eq!(worktree.version(), worktree_version);
    let failed_version = failed_changeset.version();
    failed_changeset.fail().unwrap();
    assert_eq!(failed_changeset.version(), failed_version);
    let mut interrupted = Run::new(changeset.id);
    interrupted.interrupt().unwrap();
    let interrupted_version = interrupted.version();
    interrupted.interrupt().unwrap();
    assert_eq!(interrupted.version(), interrupted_version);
}
