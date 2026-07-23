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
    let snapshot_before = service.status_snapshot(&registered, &worktree).unwrap();
    assert_eq!(snapshot_before.head_sha, registered.base_sha);
    assert_eq!(
        snapshot_before.changed_paths(),
        vec![RelativePath::parse("change.txt").unwrap()]
    );
    assert_eq!(snapshot_before.entries[0].index_status, '?');
    assert_eq!(snapshot_before.entries[0].worktree_status, '?');
    let status_before = Command::new("git")
        .current_dir(&worktree.path)
        .args(["status", "--porcelain"])
        .output()
        .unwrap()
        .stdout;
    assert!(
        service
            .diff(&worktree, std::slice::from_ref(&scope.proposal.target_path))
            .unwrap()
            .contains("change.txt")
    );
    let status_after = Command::new("git")
        .current_dir(&worktree.path)
        .args(["status", "--porcelain"])
        .output()
        .unwrap()
        .stdout;
    assert_eq!(status_after, status_before);
    assert_eq!(
        service.status_snapshot(&registered, &worktree).unwrap(),
        snapshot_before
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
    for key in [
        "filter.bad.process",
        "filter.bad.clean",
        "filter.bad.smudge",
        "diff.bad.textconv",
        "core.fsmonitor",
        "fsmonitor.bad",
    ] {
        git(&repo, &["config", key, "sh -c nope"]);
        assert!(matches!(
            service.register(&repo),
            Err(GitError::UnsupportedRepository("executable Git filters"))
        ));
        git(&repo, &["config", "--unset-all", key]);
    }
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
fn cleanup_rejects_a_replaced_worktree_directory() {
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
    let original = directory.path().join("original-worktree");
    fs::rename(&worktree.path, &original).unwrap();
    fs::create_dir(&worktree.path).unwrap();

    assert!(matches!(
        service.cleanup_worktree(&registered, &mut worktree),
        Err(GitError::RepositoryIdentityChanged)
    ));
    assert!(worktree.path.exists());
    assert_eq!(worktree.state(), domain::WorktreeState::Ready);

    fs::remove_dir(&worktree.path).unwrap();
    fs::rename(original, &worktree.path).unwrap();
    service
        .cleanup_worktree(&registered, &mut worktree)
        .unwrap();
}

#[test]
#[serial_test::serial]
fn rejects_redirected_git_directories_and_object_alternates() {
    let directory = tempdir().unwrap();
    let service = GitService::new(
        vec![directory.path().to_path_buf()],
        directory.path().join("worktrees"),
    )
    .unwrap();

    let redirected = directory.path().join("redirected");
    let redirected_git = directory.path().join("redirected.git");
    fs::create_dir(&redirected).unwrap();
    assert!(
        Command::new("git")
            .args([
                "init",
                "--separate-git-dir",
                redirected_git.to_str().unwrap(),
                redirected.to_str().unwrap(),
            ])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(matches!(
        service.register(&redirected),
        Err(GitError::UnsupportedRepository("redirected Git directory"))
    ));

    let repo = directory.path().join("repo");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.email", "test@example.com"]);
    git(&repo, &["config", "user.name", "Test"]);
    fs::write(repo.join("README.md"), "fixture\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "fixture"]);
    fs::create_dir_all(repo.join(".git/objects/info")).unwrap();
    fs::write(
        repo.join(".git/objects/info/alternates"),
        redirected_git.join("objects").display().to_string(),
    )
    .unwrap();
    assert!(matches!(
        service.register(&repo),
        Err(GitError::UnsupportedRepository("object alternates"))
    ));
}

#[test]
#[serial_test::serial]
fn registered_git_directory_replacement_is_rejected() {
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
    let original_git = directory.path().join("original.git");
    fs::rename(repo.join(".git"), &original_git).unwrap();
    fs::create_dir(repo.join(".git")).unwrap();

    assert!(matches!(
        service.create_worktree(&registered, uuid::Uuid::new_v4()),
        Err(GitError::RepositoryIdentityChanged)
    ));
}

#[test]
#[serial_test::serial]
fn approved_write_and_changed_file_count_limits_are_enforced() {
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
    let changeset_id = uuid::Uuid::new_v4();
    let mut worktree = service.create_worktree(&registered, changeset_id).unwrap();
    let content = vec![b'x'; limits::MAX_APPROVED_FILE_BYTES + 1];
    let target_path = RelativePath::parse("oversized-write.txt").unwrap();
    let scope = ApprovalScope {
        repository_id: registered.id,
        changeset_id,
        base_sha: registered.base_sha.clone(),
        head_sha: registered.base_sha.clone(),
        proposal: ActionProposal {
            action: ActionKind::WriteFile,
            target_path: target_path.clone(),
            content_sha256: content_digest(&content),
        },
        expires_at_unix_ms: 10,
    };
    let mut approval = Approval::new(uuid::Uuid::new_v4(), scope.clone());
    let approved = approval
        .consume_for_run(approval.run_id(), &scope, 1)
        .unwrap();

    assert!(matches!(
        service.write_approved_file(&registered, &worktree, approved, &content),
        Err(GitError::ResourceLimit("approved file content"))
    ));
    assert!(!worktree.path.join(target_path.as_path()).exists());

    let paths = vec![target_path; limits::MAX_CHANGED_FILES + 1];
    assert!(matches!(
        service.diff(&worktree, &paths),
        Err(GitError::ResourceLimit("changed file count"))
    ));

    service
        .cleanup_worktree(&registered, &mut worktree)
        .unwrap();
}

#[test]
#[serial_test::serial]
fn status_snapshot_rejects_too_many_changed_files() {
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
    for index in 0..=limits::MAX_CHANGED_FILES {
        fs::write(
            worktree.path.join(format!("change-{index}.txt")),
            "change\n",
        )
        .unwrap();
    }

    assert!(matches!(
        service.status_snapshot(&registered, &worktree),
        Err(GitError::ResourceLimit("changed file count"))
    ));
    service
        .cleanup_worktree(&registered, &mut worktree)
        .unwrap();
}

#[test]
#[serial_test::serial]
fn exact_commit_and_discard_reject_stale_manifests() {
    let directory = tempdir().unwrap();
    let repo = directory.path().join("repo");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.email", "test@example.com"]);
    git(&repo, &["config", "user.name", "Test"]);
    fs::write(repo.join("README.md"), "fixture\n").unwrap();
    fs::write(repo.join(".gitignore"), "ignored.txt\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "fixture"]);

    let service = GitService::new(
        vec![directory.path().to_path_buf()],
        directory.path().join("worktrees"),
    )
    .unwrap();
    let registered = service.register(&repo).unwrap();
    let changeset_id = uuid::Uuid::new_v4();
    let mut worktree = service.create_worktree(&registered, changeset_id).unwrap();
    fs::write(worktree.path.join("change.txt"), "first\n").unwrap();
    let stale = service.mutation_manifest(&registered, &worktree).unwrap();
    fs::write(worktree.path.join("change.txt"), "second\n").unwrap();
    fs::write(worktree.path.join("binary.bin"), [0, 0xff, 1, 0xfe]).unwrap();
    assert!(matches!(
        service.commit_exact(&registered, &worktree, &stale.head_sha, &stale.sha256),
        Err(GitError::MutationPreviewMismatch)
    ));

    let commit_manifest = service.mutation_manifest(&registered, &worktree).unwrap();
    let committed = service
        .commit_exact(
            &registered,
            &worktree,
            &commit_manifest.head_sha,
            &commit_manifest.sha256,
        )
        .unwrap();
    let parent = Command::new("git")
        .current_dir(&repo)
        .args(["rev-parse", &format!("{}^", committed.resulting_head_sha)])
        .output()
        .unwrap();
    assert!(parent.status.success());
    assert_eq!(
        String::from_utf8(parent.stdout).unwrap().trim(),
        registered.base_sha
    );
    let app_ref = Command::new("git")
        .current_dir(&repo)
        .args(["rev-parse", "--verify", &committed.app_ref])
        .output()
        .unwrap();
    assert!(app_ref.status.success());
    assert_eq!(
        String::from_utf8(app_ref.stdout).unwrap().trim(),
        committed.resulting_head_sha
    );
    assert_eq!(
        service
            .commit_exact(
                &registered,
                &worktree,
                &commit_manifest.head_sha,
                &commit_manifest.sha256,
            )
            .unwrap(),
        committed
    );
    assert_eq!(
        service
            .recover_exact_commit_result(
                &registered,
                changeset_id,
                &commit_manifest.head_sha,
                &commit_manifest.sha256,
            )
            .unwrap(),
        Some(committed.clone())
    );
    assert!(matches!(
        service.recover_exact_commit_result(
            &registered,
            changeset_id,
            &commit_manifest.head_sha,
            &"0".repeat(64),
        ),
        Err(GitError::MutationPreviewMismatch)
    ));
    assert!(matches!(
        service.recover_exact_commit_result(
            &registered,
            changeset_id,
            &"0".repeat(40),
            &commit_manifest.sha256,
        ),
        Err(GitError::CommitVerificationFailed)
    ));
    assert!(
        service
            .recover_exact_commit_result(
                &registered,
                uuid::Uuid::new_v4(),
                &commit_manifest.head_sha,
                &commit_manifest.sha256,
            )
            .unwrap()
            .is_none()
    );
    fs::write(worktree.path.join("ignored.txt"), "ignored\n").unwrap();
    assert!(matches!(
        service.mutation_manifest(&registered, &worktree),
        Err(GitError::IgnoredContent)
    ));
    service
        .cleanup_worktree(&registered, &mut worktree)
        .unwrap();

    let discard_id = uuid::Uuid::new_v4();
    let mut discard_worktree = service.create_worktree(&registered, discard_id).unwrap();
    fs::write(discard_worktree.path.join("discard.txt"), "discard\n").unwrap();
    let discard_manifest = service
        .mutation_manifest(&registered, &discard_worktree)
        .unwrap();
    fs::write(discard_worktree.path.join("later.txt"), "later\n").unwrap();
    assert!(matches!(
        service.discard_exact(
            &registered,
            &mut discard_worktree,
            &discard_manifest.head_sha,
            &discard_manifest.sha256,
        ),
        Err(GitError::MutationPreviewMismatch)
    ));
    assert!(discard_worktree.path.exists());
    fs::remove_file(discard_worktree.path.join("later.txt")).unwrap();
    service
        .discard_exact(
            &registered,
            &mut discard_worktree,
            &discard_manifest.head_sha,
            &discard_manifest.sha256,
        )
        .unwrap();
    assert!(!discard_worktree.path.exists());
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
