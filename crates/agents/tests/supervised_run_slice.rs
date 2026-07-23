use agents::{AgentProvider, MockProvider};
use domain::{Changeset, RelativePath, Run, RunState};
use git::GitService;
use persistence::SqliteStore;
use protocol::SemanticEventKind;
use std::{fs, process::Command};
use tempfile::tempdir;

fn git(repo: &std::path::Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .current_dir(repo)
            .args(args)
            .output()
            .unwrap()
            .status
            .success()
    );
}

#[test]
fn supervised_local_run_survives_restart() {
    let directory = tempdir().unwrap();
    let repo_path = directory.path().join("repo");
    fs::create_dir(&repo_path).unwrap();
    git(&repo_path, &["init", "-q"]);
    git(&repo_path, &["config", "user.email", "test@example.com"]);
    git(&repo_path, &["config", "user.name", "Test"]);
    fs::write(repo_path.join("README.md"), "fixture\n").unwrap();
    git(&repo_path, &["add", "."]);
    git(&repo_path, &["commit", "-qm", "fixture"]);
    let store = SqliteStore::open(directory.path().join("state.sqlite3")).unwrap();
    let git_service = GitService::new(
        vec![directory.path().to_path_buf()],
        directory.path().join("worktrees"),
    )
    .unwrap();
    let repository = git_service.register(&repo_path).unwrap();
    store.save_repository(&repository).unwrap();
    let mut changeset = Changeset::new(repository.id, repository.base_sha.clone());
    store.save_changeset(&changeset).unwrap();
    changeset.activate().unwrap();
    store.persist_changeset_transition(&changeset).unwrap();
    let mut worktree = git_service
        .create_worktree(&repository, changeset.id)
        .unwrap();
    let mut run = Run::new(changeset.id);
    store.save_run(&run).unwrap();
    run.start().unwrap();
    store
        .persist_run_transition(
            &run,
            SemanticEventKind::Lifecycle {
                state: RunState::Starting,
            },
        )
        .unwrap();
    run.running().unwrap();
    store
        .persist_run_transition(
            &run,
            SemanticEventKind::Lifecycle {
                state: RunState::Running,
            },
        )
        .unwrap();
    let process_result = execution::supervise(
        &execution::ProcessSpec {
            program: "/bin/sh".into(),
            arguments: vec!["-c".into(), "exit 0".into()],
            environment: std::collections::HashMap::new(),
            sensitive_environment_keys: Vec::new(),
            current_dir: Some(worktree.path.clone()),
            timeout: std::time::Duration::from_secs(2),
            output_limit: 1024,
            confinement: execution::ProcessConfinement::Unconfined,
        },
        &execution::CancellationToken::default(),
    )
    .unwrap();
    assert_eq!(
        process_result.outcome,
        execution::TerminalOutcome::Completed(0)
    );
    let provider = MockProvider::deterministic();
    let change = provider.propose(Some(&process_result)).unwrap();
    let expires = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        + 60_000;
    let mut scope = provider.approval_scope(
        repository.id,
        changeset.id,
        repository.base_sha.clone(),
        repository.base_sha.clone(),
        expires,
    );
    let approval = domain::Approval::new(run.id, scope.clone());
    let normalized = provider.normalized_events(&change, approval.digest());
    store
        .persist_run_transition(&run, normalized[0].clone())
        .unwrap();
    run.await_approval(approval.digest().to_owned()).unwrap();
    store
        .persist_run_transition(&run, normalized[1].clone())
        .unwrap();
    store.save_approval(&approval).unwrap();
    scope.proposal.target_path = RelativePath::parse("tampered.txt").unwrap();
    assert!(
        store
            .authorize_and_resume(run.id, approval.id, &scope, expires - 1)
            .is_err()
    );
    let valid_scope = provider.approval_scope(
        repository.id,
        changeset.id,
        repository.base_sha.clone(),
        repository.base_sha.clone(),
        expires,
    );
    let policy_approval = approval.clone();
    policy::authorize_exact_action(&policy_approval, run.id, &valid_scope, expires - 1).unwrap();
    let authorized = store
        .authorize_and_resume(run.id, approval.id, &valid_scope, expires - 1)
        .unwrap();
    run = authorized.run;
    let action_digest = authorized.approved_action.action_digest().to_owned();
    git_service
        .write_approved_file(
            &repository,
            &worktree,
            authorized.approved_action,
            &change.content,
        )
        .unwrap();
    store
        .persist_run_transition(
            &run,
            SemanticEventKind::ActionResult {
                digest: action_digest,
                success: true,
            },
        )
        .unwrap();
    store
        .persist_run_transition(
            &run,
            SemanticEventKind::FileChange {
                path: change.proposal.target_path.clone(),
            },
        )
        .unwrap();
    let diff = git_service
        .diff(
            &worktree,
            std::slice::from_ref(&change.proposal.target_path),
        )
        .unwrap();
    assert!(diff.contains("approved local change"));
    let head = git_service.head_sha(&worktree).unwrap();
    changeset.mark_reviewable(head.clone()).unwrap();
    store.persist_changeset_transition(&changeset).unwrap();
    store
        .save_checkpoint(&domain::Checkpoint {
            id: uuid::Uuid::new_v4(),
            run_id: run.id,
            base_sha: repository.base_sha.clone(),
            head_sha: head,
            diff,
        })
        .unwrap();
    run.mark_reviewable().unwrap();
    store
        .persist_run_transition(
            &run,
            SemanticEventKind::Lifecycle {
                state: RunState::Reviewable,
            },
        )
        .unwrap();
    run.complete().unwrap();
    store
        .persist_run_transition(
            &run,
            SemanticEventKind::Lifecycle {
                state: RunState::Completed,
            },
        )
        .unwrap();
    let restarted = SqliteStore::open(directory.path().join("state.sqlite3")).unwrap();
    let report = restarted.recover().unwrap();
    assert_eq!(report.terminal, vec![run.id]);
    assert_eq!(
        restarted
            .events(run.id)
            .unwrap()
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        (1..=9).collect::<Vec<_>>()
    );
    assert!(
        restarted
            .authorize_and_resume(run.id, approval.id, &valid_scope, expires - 1)
            .is_err()
    );
    let worktree_path = worktree.path.clone();
    git_service
        .cleanup_worktree(&repository, &mut worktree)
        .unwrap();
    assert!(!worktree_path.exists());
    git_service
        .cleanup_worktree(&repository, &mut worktree)
        .unwrap();
    assert_eq!(run.state(), RunState::Completed);
}
