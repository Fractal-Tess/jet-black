use domain::{ActionKind, ActionProposal, Approval, ApprovalScope, RelativePath, limits};
use git::{GitError, GitService, content_digest};
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
#[serial_test::serial]
fn worktree_and_file_operations_are_bounded() {
    let directory = tempdir().unwrap();
    let repo = directory.path().join("repo");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.email", "test@example.com"]);
    git(&repo, &["config", "user.name", "Test"]);
    fs::write(repo.join("README.md"), "fixture\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "fixture"]);
    let service = GitService::new(
        vec![directory.path().to_path_buf()],
        directory.path().join("worktrees"),
    )
    .unwrap();
    let registered = service.register(&repo).unwrap();
    assert_eq!(registered.base_sha.len(), 40);
    let changeset_id = uuid::Uuid::new_v4();
    let mut worktree = service.create_worktree(&registered, changeset_id).unwrap();
    let scope = ApprovalScope {
        repository_id: registered.id,
        changeset_id,
        base_sha: registered.base_sha.clone(),
        head_sha: registered.base_sha.clone(),
        proposal: ActionProposal {
            action: ActionKind::WriteFile,
            target_path: RelativePath::parse("change.txt").unwrap(),
            content_sha256: content_digest(b"changed\n"),
        },
        expires_at_unix_ms: 10,
    };
    let mut approval = Approval::new(uuid::Uuid::new_v4(), scope.clone());
    let approved = approval
        .consume_for_run(approval.run_id(), &scope, 1)
        .unwrap();
    service
        .write_approved_file(&registered, &worktree, approved, b"changed\n")
        .unwrap();
    assert!(
        service
            .diff(&worktree, std::slice::from_ref(&scope.proposal.target_path))
            .unwrap()
            .contains("change.txt")
    );
    assert!(RelativePath::parse("../escape.txt").is_err());
    let outside = directory.path().join("outside.txt");
    assert!(!outside.exists());
    let worktree_path = worktree.path.clone();
    service
        .cleanup_worktree(&registered, &mut worktree)
        .unwrap();
    service
        .cleanup_worktree(&registered, &mut worktree)
        .unwrap();
    assert!(!worktree_path.exists());
}

#[test]
#[serial_test::serial]
fn rejects_invalid_forms_and_symlink_escape() {
    let directory = tempfile::tempdir().unwrap();
    let service = GitService::new(
        vec![directory.path().to_path_buf()],
        directory.path().join("worktrees"),
    )
    .unwrap();
    assert!(service.register(&directory.path().join("missing")).is_err());
    let bare = directory.path().join("bare");
    assert!(
        Command::new("git")
            .args(["init", "--bare", "-q", bare.to_str().unwrap()])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(service.register(&bare).is_err());
    let repo = directory.path().join("repo");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.email", "test@example.com"]);
    git(&repo, &["config", "user.name", "Test"]);
    fs::write(repo.join("README.md"), "fixture\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "fixture"]);
    git(&repo, &["config", "filter.bad.process", "sh -c nope"]);
    assert!(service.register(&repo).is_err());
    git(&repo, &["config", "--unset-all", "filter.bad.process"]);
    git(
        &repo,
        &[
            "remote",
            "add",
            "origin",
            "https://token@example.com/repo.git",
        ],
    );
    let registered = service.register(&repo).unwrap();
    assert!(
        !registered
            .primary_remote
            .as_deref()
            .unwrap()
            .contains("token@")
    );
    let mut worktree = service
        .create_worktree(&registered, uuid::Uuid::new_v4())
        .unwrap();
    let outside = directory.path().join("outside");
    fs::create_dir(&outside).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, worktree.path.join("linked")).unwrap();
    #[cfg(unix)]
    {
        let scope = ApprovalScope {
            repository_id: registered.id,
            changeset_id: worktree.changeset_id(),
            base_sha: registered.base_sha.clone(),
            head_sha: registered.base_sha.clone(),
            proposal: ActionProposal {
                action: ActionKind::WriteFile,
                target_path: RelativePath::parse("linked/escape.txt").unwrap(),
                content_sha256: content_digest(b"bad"),
            },
            expires_at_unix_ms: 10,
        };
        let mut approval = Approval::new(uuid::Uuid::new_v4(), scope.clone());
        let approved = approval
            .consume_for_run(approval.run_id(), &scope, 1)
            .unwrap();
        assert!(
            service
                .write_approved_file(&registered, &worktree, approved, b"bad")
                .is_err()
        );
        assert!(!outside.join("escape.txt").exists());
    }
    service
        .cleanup_worktree(&registered, &mut worktree)
        .unwrap();
}

#[test]
#[serial_test::serial]
fn git_repository_context_is_not_redirected_by_parent_environment() {
    let directory = tempdir().unwrap();
    let repo = directory.path().join("repo");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.email", "test@example.com"]);
    git(&repo, &["config", "user.name", "Test"]);
    fs::write(repo.join("README.md"), "fixture\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "fixture"]);
    let service = GitService::new(
        vec![directory.path().to_path_buf()],
        directory.path().join("worktrees"),
    )
    .unwrap();
    let previous = std::env::var_os("GIT_DIR");
    unsafe {
        std::env::set_var("GIT_DIR", directory.path().join("not-a-repository"));
    }
    let result = service.register(&repo);
    match previous {
        Some(value) => unsafe { std::env::set_var("GIT_DIR", value) },
        None => unsafe { std::env::remove_var("GIT_DIR") },
    }
    assert!(result.is_ok());
}

#[test]
#[serial_test::serial]
fn oversized_diff_is_rejected() {
    let directory = tempdir().unwrap();
    let repo = directory.path().join("repo");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.email", "test@example.com"]);
    git(&repo, &["config", "user.name", "Test"]);
    fs::write(repo.join("README.md"), "fixture\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "fixture"]);

    let service = GitService::new(
        vec![directory.path().to_path_buf()],
        directory.path().join("worktrees"),
    )
    .unwrap();
    let registered = service.register(&repo).unwrap();
    let mut worktree = service
        .create_worktree(&registered, uuid::Uuid::new_v4())
        .unwrap();
    let path = RelativePath::parse("oversized.txt").unwrap();
    fs::write(
        worktree.path.join(path.as_path()),
        vec![b'x'; limits::MAX_UNIFIED_DIFF_BYTES + 1],
    )
    .unwrap();

    assert!(matches!(
        service.diff(&worktree, std::slice::from_ref(&path)),
        Err(GitError::ResourceLimit("unified diff"))
    ));

    service
        .cleanup_worktree(&registered, &mut worktree)
        .unwrap();
}
