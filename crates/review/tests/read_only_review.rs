use domain::{Changeset, RelativePath, limits};
use git::GitService;
use persistence::SqliteStore;
use review::{ReviewCheckKind, ReviewCheckStatus, ReviewError, ReviewOptions, ReviewService};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use tempfile::TempDir;

struct Fixture {
    _directory: TempDir,
    store: SqliteStore,
    repository: domain::Repository,
    changeset: Changeset,
    worktree: domain::Worktree,
    bin: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let repository_path = directory.path().join("repository");
        fs::create_dir(&repository_path).unwrap();
        git(&repository_path, &["init", "-q"]);
        git(
            &repository_path,
            &["config", "user.email", "test@example.com"],
        );
        git(&repository_path, &["config", "user.name", "Test"]);
        fs::write(repository_path.join("README.md"), "fixture\n").unwrap();
        git(&repository_path, &["add", "."]);
        git(&repository_path, &["commit", "-qm", "fixture"]);

        let git_service = GitService::new(
            vec![directory.path().to_path_buf()],
            directory.path().join("worktrees"),
        )
        .unwrap();
        let repository = git_service.register(&repository_path).unwrap();
        let mut changeset = Changeset::new(repository.id, repository.base_sha.clone());
        changeset.activate().unwrap();
        let worktree = git_service
            .create_worktree(&repository, changeset.id)
            .unwrap();
        fs::write(worktree.path.join("review.txt"), "review\n").unwrap();
        changeset
            .mark_reviewable(repository.base_sha.clone())
            .unwrap();

        let store = SqliteStore::open(directory.path().join("state.sqlite3")).unwrap();
        store.save_repository(&repository).unwrap();
        store.save_changeset(&changeset).unwrap();
        store.save_worktree(&worktree).unwrap();
        let bin = directory.path().join("bin");
        fs::create_dir(&bin).unwrap();

        Self {
            _directory: directory,
            store,
            repository,
            changeset,
            worktree,
            bin,
        }
    }

    fn service(&self, options: ReviewOptions) -> ReviewService {
        let git_service = GitService::new(
            vec![self._directory.path().to_path_buf()],
            self._directory.path().join("worktrees"),
        )
        .unwrap();
        ReviewService::with_search_path(
            self.store.clone(),
            git_service,
            options,
            self.bin.to_str().unwrap(),
        )
        .unwrap()
    }

    fn write_program(&self, name: &str, body: &str) {
        let path = self.bin.join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(path, permissions).unwrap();
    }
}

fn git(repository: &Path, arguments: &[&str]) {
    assert!(
        Command::new("git")
            .current_dir(repository)
            .args(arguments)
            .status()
            .unwrap()
            .success()
    );
}

fn trusted_options(timeout: Duration, output_limit: usize) -> ReviewOptions {
    ReviewOptions {
        timeout,
        output_limit,
        allow_unsandboxed_checks: true,
    }
}

#[test]
#[serial_test::serial]
fn fixed_check_passes_without_changing_the_worktree() {
    let fixture = Fixture::new();
    fixture.write_program("cargo", "printf 'checked'");
    let report = fixture
        .service(trusted_options(Duration::from_secs(1), 64))
        .review(
            &fixture.repository,
            &fixture.changeset,
            &fixture.worktree,
            &[ReviewCheckKind::Format],
        )
        .unwrap();

    assert_eq!(
        report.changed_paths,
        vec![RelativePath::parse("review.txt").unwrap()]
    );
    assert!(report.unified_diff.contains("review.txt"));
    assert_eq!(report.checks[0].status, ReviewCheckStatus::Passed);
    assert_eq!(report.checks[0].evidence, "checked");
    assert!(report.findings.is_empty());
}

#[test]
#[serial_test::serial]
fn unavailable_check_is_explicit_and_persisted() {
    let fixture = Fixture::new();
    let report = fixture
        .service(trusted_options(Duration::from_secs(1), 64))
        .review(
            &fixture.repository,
            &fixture.changeset,
            &fixture.worktree,
            &[ReviewCheckKind::SecretScan],
        )
        .unwrap();

    assert_eq!(report.checks[0].status, ReviewCheckStatus::Unavailable);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(
        fixture
            .store
            .findings_for_changeset(fixture.changeset.id)
            .unwrap(),
        report.findings
    );
}

#[test]
#[serial_test::serial]
fn mutation_is_detected_and_stops_remaining_checks() {
    let fixture = Fixture::new();
    fixture.write_program("cargo", "printf mutation > review.txt");
    let report = fixture
        .service(trusted_options(Duration::from_secs(1), 64))
        .review(
            &fixture.repository,
            &fixture.changeset,
            &fixture.worktree,
            &[ReviewCheckKind::Format, ReviewCheckKind::Typecheck],
        )
        .unwrap();

    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].status, ReviewCheckStatus::Mutated);
    assert!(
        report.checks[0]
            .evidence
            .contains("changed the worktree or HEAD")
    );
    assert_eq!(report.findings.len(), 1);
}

#[test]
#[serial_test::serial]
fn timeout_and_output_limits_become_findings() {
    let timeout_fixture = Fixture::new();
    timeout_fixture.write_program("cargo", "while :; do :; done");
    let timeout_report = timeout_fixture
        .service(trusted_options(Duration::from_millis(25), 64))
        .review(
            &timeout_fixture.repository,
            &timeout_fixture.changeset,
            &timeout_fixture.worktree,
            &[ReviewCheckKind::Format],
        )
        .unwrap();
    assert_eq!(timeout_report.checks[0].status, ReviewCheckStatus::TimedOut);

    let output_fixture = Fixture::new();
    output_fixture.write_program(
        "cargo",
        "i=0; while [ \"$i\" -lt 200 ]; do printf x; i=$((i + 1)); done",
    );
    let output_report = output_fixture
        .service(trusted_options(Duration::from_secs(1), 32))
        .review(
            &output_fixture.repository,
            &output_fixture.changeset,
            &output_fixture.worktree,
            &[ReviewCheckKind::Format],
        )
        .unwrap();
    assert_eq!(output_report.checks[0].status, ReviewCheckStatus::Failed);
    assert!(output_report.checks[0].evidence.len() <= 32);
    assert!(
        output_report.checks[0]
            .evidence
            .starts_with("process output exceeded")
    );
}

#[test]
#[serial_test::serial]
fn repository_code_execution_checks_require_explicit_trust_and_are_deduplicated() {
    let fixture = Fixture::new();
    let checks = [ReviewCheckKind::Typecheck, ReviewCheckKind::Test];
    for _ in 0..2 {
        let report = fixture
            .service(ReviewOptions::default())
            .review(
                &fixture.repository,
                &fixture.changeset,
                &fixture.worktree,
                &checks,
            )
            .unwrap();
        assert!(
            report
                .checks
                .iter()
                .all(|check| check.status == ReviewCheckStatus::Unavailable)
        );
        assert!(
            report
                .checks
                .iter()
                .all(|check| check.evidence.contains("explicit trust"))
        );
        assert_eq!(
            report.findings,
            fixture
                .store
                .findings_for_changeset(fixture.changeset.id)
                .unwrap()
        );
    }
}

#[test]
#[serial_test::serial]
fn oversized_and_nonregular_changed_files_are_rejected_before_execution() {
    let oversized = Fixture::new();
    fs::write(
        oversized.worktree.path.join("review.txt"),
        vec![b'x'; limits::MAX_APPROVED_FILE_BYTES + 1],
    )
    .unwrap();
    assert!(matches!(
        oversized.service(ReviewOptions::default()).review(
            &oversized.repository,
            &oversized.changeset,
            &oversized.worktree,
            &[]
        ),
        Err(ReviewError::ChangedFileTooLarge(_))
    ));

    let symlink = Fixture::new();
    fs::remove_file(symlink.worktree.path.join("review.txt")).unwrap();
    std::os::unix::fs::symlink("README.md", symlink.worktree.path.join("review.txt")).unwrap();
    assert!(matches!(
        symlink.service(ReviewOptions::default()).review(
            &symlink.repository,
            &symlink.changeset,
            &symlink.worktree,
            &[]
        ),
        Err(ReviewError::UnsupportedChangedPath(_))
    ));
}

#[test]
#[serial_test::serial]
fn duplicate_checks_and_unsafe_options_are_rejected() {
    let fixture = Fixture::new();
    assert!(matches!(
        fixture.service(ReviewOptions::default()).review(
            &fixture.repository,
            &fixture.changeset,
            &fixture.worktree,
            &[ReviewCheckKind::Format, ReviewCheckKind::Format]
        ),
        Err(ReviewError::DuplicateCheck)
    ));
    assert!(matches!(
        ReviewService::with_search_path(
            fixture.store.clone(),
            GitService::new(
                vec![fixture._directory.path().to_path_buf()],
                fixture._directory.path().join("worktrees")
            )
            .unwrap(),
            trusted_options(Duration::ZERO, 1),
            fixture.bin.to_str().unwrap()
        ),
        Err(ReviewError::InvalidOptions)
    ));
    assert!(matches!(
        ReviewService::with_search_path(
            fixture.store.clone(),
            GitService::new(
                vec![fixture._directory.path().to_path_buf()],
                fixture._directory.path().join("worktrees")
            )
            .unwrap(),
            ReviewOptions::default(),
            "relative/bin"
        ),
        Err(ReviewError::UnsafeSearchPath)
    ));
}
