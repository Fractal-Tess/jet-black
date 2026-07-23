use domain::{
    ActionKind, ActionProposal, Approval, ApprovalScope, Changeset, ChangesetMutationKind,
    ChangesetMutationScope, Checkpoint, Finding, RelativePath, Repository, Run, RunState, Worktree,
    WorktreeState, limits,
};
use execution::{
    ExecutableIdentity, ProcessGroupIdentity, ProcessStartIdentity, SupervisionMetadata,
    SupervisionState, TerminationReason,
};
use persistence::{
    ChangesetFinalizationResult, ChangesetFinalizationState, MutationLease, PersistenceError,
    SqliteStore,
};
use protocol::{OrderedRunEvent, RecoveryAction, SemanticEventKind};
use rusqlite::{Connection, params};
use std::time::Duration;
use tempfile::tempdir;

fn finding(changeset_id: uuid::Uuid, path: &str) -> Finding {
    Finding::new(
        changeset_id,
        Some(RelativePath::parse(path).unwrap()),
        "fixture-blob".to_owned(),
        "check".to_owned(),
        "warning".to_owned(),
        "fixture finding".to_owned(),
        "fixture evidence".to_owned(),
    )
}

fn supervision_metadata(state: SupervisionState) -> SupervisionMetadata {
    SupervisionMetadata {
        supervision_id: uuid::Uuid::new_v4(),
        pid: 42,
        process_group: ProcessGroupIdentity::UnixProcessGroup { pgid: 42 },
        process_start: ProcessStartIdentity::Linux {
            boot_id: "fixture-boot".to_owned(),
            start_time_ticks: 7,
        },
        executable: ExecutableIdentity {
            path: "/bin/sh".into(),
            device: 1,
            inode: 2,
        },
        command_digest: "command".to_owned(),
        environment_digest: "environment".to_owned(),
        supervision_token: "nonsecret-token".to_owned(),
        state,
        termination_reason: None,
    }
}

fn seed_changeset(store: &SqliteStore, changeset_id: uuid::Uuid) {
    let repository = Repository {
        id: uuid::Uuid::new_v4(),
        filesystem_identity: "fixture".into(),
        git_directory_identity: "git-fixture".into(),
        canonical_path: "/fixture".into(),
        identity: "fixture".into(),
        primary_remote: None,
        default_branch: "main".into(),
        base_sha: "base".into(),
        version: 0,
    };
    store.save_repository(&repository).unwrap();
    let mut changeset = Changeset::new(repository.id, "base".into());
    changeset.id = changeset_id;
    store.save_changeset(&changeset).unwrap();
}

fn seed_pending_approval(store: &SqliteStore) -> (Run, Approval) {
    let repository = Repository {
        id: uuid::Uuid::new_v4(),
        filesystem_identity: "approval-fixture".into(),
        git_directory_identity: "approval-git-fixture".into(),
        canonical_path: "/approval-fixture".into(),
        identity: "approval-fixture".into(),
        primary_remote: None,
        default_branch: "main".into(),
        base_sha: "base".into(),
        version: 0,
    };
    store.save_repository(&repository).unwrap();
    let changeset = Changeset::new(repository.id, repository.base_sha.clone());
    store.save_changeset(&changeset).unwrap();

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

    let scope = ApprovalScope {
        repository_id: repository.id,
        changeset_id: changeset.id,
        base_sha: repository.base_sha,
        head_sha: "head".into(),
        proposal: ActionProposal {
            action: ActionKind::WriteFile,
            target_path: RelativePath::parse("approved.txt").unwrap(),
            content_sha256: "content-digest".into(),
        },
        expires_at_unix_ms: 10_000,
    };
    let approval = Approval::new(run.id, scope);
    run.await_approval(approval.digest().to_owned()).unwrap();
    store
        .request_approval(
            &run,
            &approval,
            SemanticEventKind::ActionProposal {
                proposal: approval.scope().proposal.clone(),
                digest: approval.digest().to_owned(),
            },
        )
        .unwrap();
    (run, approval)
}

struct FinalizationFixture {
    changeset: Changeset,
    worktree: Worktree,
    scope: ChangesetMutationScope,
}

fn seed_finalization_fixture(
    store: &SqliteStore,
    kind: ChangesetMutationKind,
) -> FinalizationFixture {
    let base_sha = "a".repeat(40);
    let head_sha = base_sha.clone();
    let repository = Repository {
        id: uuid::Uuid::new_v4(),
        filesystem_identity: "fixture".into(),
        git_directory_identity: "git-fixture".into(),
        canonical_path: "/fixture".into(),
        identity: "fixture".into(),
        primary_remote: None,
        default_branch: "main".into(),
        base_sha: base_sha.clone(),
        version: 0,
    };
    store.save_repository(&repository).unwrap();
    let mut changeset = Changeset::new(repository.id, base_sha.clone());
    store.save_changeset(&changeset).unwrap();
    changeset.activate().unwrap();
    store.persist_changeset_transition(&changeset).unwrap();
    changeset.mark_reviewable(head_sha.clone()).unwrap();
    store.persist_changeset_transition(&changeset).unwrap();

    let run = Run::new(changeset.id);
    store.save_run(&run).unwrap();
    let checkpoint = Checkpoint {
        id: uuid::Uuid::new_v4(),
        run_id: run.id,
        base_sha: base_sha.clone(),
        head_sha: head_sha.clone(),
        diff: "fixture diff".to_owned(),
    };
    store.save_checkpoint(&checkpoint).unwrap();
    let mut worktree = Worktree::creating(
        uuid::Uuid::new_v4(),
        changeset.id,
        "/fixture-worktree".into(),
        "worktree-fixture".to_owned(),
        base_sha.clone(),
    );
    worktree.ready().unwrap();
    store.save_worktree(&worktree).unwrap();
    let scope = ChangesetMutationScope {
        kind,
        repository_id: repository.id,
        changeset_id: changeset.id,
        checkpoint_id: checkpoint.id,
        expected_version: changeset.version(),
        base_sha,
        expected_head_sha: head_sha,
        manifest_sha256: "d".repeat(64),
    };
    FinalizationFixture {
        changeset,
        worktree,
        scope,
    }
}

#[test]
fn worktree_lookup_by_id_returns_existing_and_missing_rows() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("worktree-lookup.sqlite3")).unwrap();
    let changeset_id = uuid::Uuid::new_v4();
    seed_changeset(&store, changeset_id);
    let worktree = Worktree::creating(
        uuid::Uuid::new_v4(),
        changeset_id,
        "/worktree".into(),
        "fixture".to_owned(),
        "base".to_owned(),
    );
    store.save_worktree(&worktree).unwrap();

    assert_eq!(store.worktree(worktree.id).unwrap(), Some(worktree));
    assert_eq!(store.worktree(uuid::Uuid::new_v4()).unwrap(), None);
}

#[test]
fn restart_maps_active_states_deterministically() {
    let directory = tempdir().expect("temp directory");
    let path = directory.path().join("state.sqlite3");
    let store = SqliteStore::open(&path).expect("store");
    let mut run = Run::new(uuid::Uuid::new_v4());
    seed_changeset(&store, run.changeset_id());
    store.save_run(&run).expect("queued");
    store
        .persist_run_transition(
            &{
                run.start().unwrap();
                run.clone()
            },
            SemanticEventKind::Lifecycle {
                state: RunState::Starting,
            },
        )
        .expect("starting");
    let report = SqliteStore::open(&path).unwrap().recover().unwrap();
    assert_eq!(report.failed, vec![run.id]);
    let recovered = SqliteStore::open(&path)
        .unwrap()
        .run(run.id)
        .unwrap()
        .unwrap();
    assert_eq!(recovered.state(), domain::RunState::Failed);
    let events = SqliteStore::open(&path).unwrap().events(run.id).unwrap();
    assert_eq!(
        events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
}

#[test]
fn every_run_state_has_one_documented_recovery_result() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("matrix.sqlite3");
    let store = SqliteStore::open(&path).unwrap();
    let queued = Run::new(uuid::Uuid::new_v4());
    seed_changeset(&store, queued.changeset_id());
    store.save_run(&queued).unwrap();
    let mut starting = Run::new(uuid::Uuid::new_v4());
    seed_changeset(&store, starting.changeset_id());
    store.save_run(&starting).unwrap();
    starting.start().unwrap();
    store
        .persist_run_transition(
            &starting,
            SemanticEventKind::Lifecycle {
                state: RunState::Starting,
            },
        )
        .unwrap();
    let mut running = Run::new(uuid::Uuid::new_v4());
    seed_changeset(&store, running.changeset_id());
    store.save_run(&running).unwrap();
    running.start().unwrap();
    store
        .persist_run_transition(
            &running,
            SemanticEventKind::Lifecycle {
                state: RunState::Starting,
            },
        )
        .unwrap();
    running.running().unwrap();
    store
        .persist_run_transition(
            &running,
            SemanticEventKind::Lifecycle {
                state: RunState::Running,
            },
        )
        .unwrap();
    let mut awaiting = Run::new(uuid::Uuid::new_v4());
    seed_changeset(&store, awaiting.changeset_id());
    store.save_run(&awaiting).unwrap();
    awaiting.start().unwrap();
    store
        .persist_run_transition(
            &awaiting,
            SemanticEventKind::Lifecycle {
                state: RunState::Starting,
            },
        )
        .unwrap();
    awaiting.running().unwrap();
    store
        .persist_run_transition(
            &awaiting,
            SemanticEventKind::Lifecycle {
                state: RunState::Running,
            },
        )
        .unwrap();
    awaiting.await_approval("digest".into()).unwrap();
    store
        .persist_run_transition(
            &awaiting,
            SemanticEventKind::Lifecycle {
                state: RunState::AwaitingApproval,
            },
        )
        .unwrap();
    let mut reviewable = Run::new(uuid::Uuid::new_v4());
    seed_changeset(&store, reviewable.changeset_id());
    store.save_run(&reviewable).unwrap();
    reviewable.start().unwrap();
    store
        .persist_run_transition(
            &reviewable,
            SemanticEventKind::Lifecycle {
                state: RunState::Starting,
            },
        )
        .unwrap();
    reviewable.running().unwrap();
    store
        .persist_run_transition(
            &reviewable,
            SemanticEventKind::Lifecycle {
                state: RunState::Running,
            },
        )
        .unwrap();
    reviewable.mark_reviewable().unwrap();
    store
        .persist_run_transition(
            &reviewable,
            SemanticEventKind::Lifecycle {
                state: RunState::Reviewable,
            },
        )
        .unwrap();
    let report = SqliteStore::open(&path).unwrap().recover().unwrap();
    assert!(report.recoverable.contains(&queued.id));
    assert!(report.recoverable.contains(&reviewable.id));
    assert!(report.failed.contains(&starting.id));
    assert!(report.interrupted.contains(&running.id));
    assert!(report.interrupted.contains(&awaiting.id));
}

#[test]
fn one_changeset_cannot_have_two_active_runs() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("exclusive.sqlite3")).unwrap();
    let changeset_id = uuid::Uuid::new_v4();
    seed_changeset(&store, changeset_id);
    store.save_run(&Run::new(changeset_id)).unwrap();
    assert!(store.save_run(&Run::new(changeset_id)).is_err());
}

#[test]
fn semantic_text_and_event_pages_are_bounded() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("bounded.sqlite3")).unwrap();
    let run = Run::new(uuid::Uuid::new_v4());
    seed_changeset(&store, run.changeset_id());
    store.save_run(&run).unwrap();

    let oversized = "x".repeat(limits::MAX_SEMANTIC_TEXT_BYTES + 1);
    assert!(matches!(
        store.persist_run_transition(&run, SemanticEventKind::Text { text: oversized }),
        Err(PersistenceError::ResourceLimit("semantic event text"))
    ));
    assert!(store.events(run.id).unwrap().is_empty());

    for text in ["first", "second"] {
        store
            .persist_run_transition(
                &run,
                SemanticEventKind::Text {
                    text: text.to_owned(),
                },
            )
            .unwrap();
    }

    assert_eq!(store.events_after(run.id, 0, 1).unwrap().len(), 1);
    assert_eq!(store.events_after(run.id, 1, 1).unwrap()[0].sequence, 2);

    for _ in 0..20 {
        store
            .persist_run_transition(
                &run,
                SemanticEventKind::Text {
                    text: "x".repeat(limits::MAX_SEMANTIC_TEXT_BYTES),
                },
            )
            .unwrap();
    }
    let byte_bounded = store
        .events_after(run.id, 0, limits::MAX_EVENT_PAGE_SIZE)
        .unwrap();
    assert!(byte_bounded.len() < 22);
    assert!(
        byte_bounded
            .iter()
            .map(|event| serde_json::to_string(event).unwrap().len())
            .sum::<usize>()
            <= limits::MAX_EVENT_PAGE_BYTES
    );

    assert!(matches!(
        store.events_after(run.id, 0, 0),
        Err(PersistenceError::ResourceLimit("event page size"))
    ));
    assert!(matches!(
        store.events_after(run.id, 0, limits::MAX_EVENT_PAGE_SIZE + 1),
        Err(PersistenceError::ResourceLimit("event page size"))
    ));
}

#[test]
fn snapshot_events_return_a_bounded_initial_page() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("snapshot-events.sqlite3");
    let store = SqliteStore::open(&path).unwrap();
    let run = Run::new(uuid::Uuid::new_v4());
    seed_changeset(&store, run.changeset_id());
    store.save_run(&run).unwrap();

    let mut connection = Connection::open(&path).unwrap();
    let transaction = connection.transaction().unwrap();
    for sequence in 1..=(limits::MAX_EVENT_PAGE_SIZE + 1) {
        let event = OrderedRunEvent {
            run_id: run.id,
            sequence: sequence as u64,
            event: SemanticEventKind::Text {
                text: "event".to_owned(),
            },
        };
        transaction
            .execute(
                "INSERT INTO semantic_events (run_id, sequence, body) VALUES (?1, ?2, ?3)",
                params![
                    run.id.to_string(),
                    sequence as u64,
                    serde_json::to_string(&event).unwrap()
                ],
            )
            .unwrap();
    }
    transaction.commit().unwrap();

    let snapshot = store.run_snapshot(run.id).unwrap().unwrap();
    assert_eq!(snapshot.events.len(), limits::MAX_SNAPSHOT_EVENT_PAGE_SIZE);
    assert_eq!(snapshot.events.first().unwrap().sequence, 1);
    assert_eq!(
        snapshot.events.last().unwrap().sequence,
        limits::MAX_SNAPSHOT_EVENT_PAGE_SIZE as u64
    );
}

#[test]
fn run_snapshot_exposes_only_the_exact_pending_approval() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("snapshot-approval.sqlite3")).unwrap();
    let (run, approval) = seed_pending_approval(&store);

    let snapshot = store.run_snapshot(run.id).unwrap().unwrap();
    assert_eq!(snapshot.pending_approval, Some(approval.clone()));

    store.reject_and_interrupt(run.id, approval.id).unwrap();
    let interrupted = store.run_snapshot(run.id).unwrap().unwrap();
    assert!(interrupted.pending_approval.is_none());
}

#[test]
fn run_snapshot_rejects_corrupt_pending_approval_identity() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("snapshot-approval-identity.sqlite3");
    let store = SqliteStore::open(&path).unwrap();
    let (run, approval) = seed_pending_approval(&store);
    let connection = Connection::open(&path).unwrap();

    let mut corrupt_identity = serde_json::to_value(&approval).unwrap();
    corrupt_identity["run_id"] = serde_json::Value::String(uuid::Uuid::new_v4().to_string());
    connection
        .execute(
            "UPDATE approvals SET body = ?2 WHERE id = ?1",
            params![approval.id.to_string(), corrupt_identity.to_string()],
        )
        .unwrap();
    assert!(matches!(
        store.run_snapshot(run.id),
        Err(PersistenceError::CorruptOwnership("approval"))
    ));
}

#[test]
fn run_snapshot_rejects_corrupt_pending_approval_digest() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("snapshot-approval-digest.sqlite3");
    let store = SqliteStore::open(&path).unwrap();
    let (run, approval) = seed_pending_approval(&store);
    let connection = Connection::open(&path).unwrap();

    let mut corrupt_digest = serde_json::to_value(&approval).unwrap();
    corrupt_digest["digest"] = serde_json::Value::String("tampered".into());
    connection
        .execute(
            "UPDATE approvals SET body = ?2 WHERE id = ?1",
            params![approval.id.to_string(), corrupt_digest.to_string()],
        )
        .unwrap();
    assert!(matches!(
        store.run_snapshot(run.id),
        Err(PersistenceError::ApprovalDoesNotMatchRun)
    ));
}

#[test]
fn historical_run_snapshot_does_not_attach_a_later_worktree() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("snapshot-worktree.sqlite3")).unwrap();
    let changeset_id = uuid::Uuid::new_v4();
    seed_changeset(&store, changeset_id);

    let mut first = Run::new(changeset_id);
    store.save_run(&first).unwrap();
    let first_worktree = domain::Worktree::creating(
        uuid::Uuid::new_v4(),
        changeset_id,
        "/first".into(),
        "first".to_owned(),
        "base".to_owned(),
    );
    store.save_worktree(&first_worktree).unwrap();
    first.interrupt().unwrap();
    store
        .persist_run_transition(
            &first,
            SemanticEventKind::Lifecycle {
                state: first.state(),
            },
        )
        .unwrap();

    let second = Run::new(changeset_id);
    store.save_run(&second).unwrap();
    let second_worktree = domain::Worktree::creating(
        uuid::Uuid::new_v4(),
        changeset_id,
        "/second".into(),
        "second".to_owned(),
        "base".to_owned(),
    );
    store.save_worktree(&second_worktree).unwrap();

    assert!(
        store
            .run_snapshot(first.id)
            .unwrap()
            .unwrap()
            .worktree
            .is_none()
    );
    assert_eq!(
        store
            .run_snapshot(second.id)
            .unwrap()
            .unwrap()
            .worktree
            .unwrap()
            .id,
        second_worktree.id
    );
}

#[test]
fn changeset_run_history_is_newest_first_and_bounded() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("history.sqlite3")).unwrap();
    let changeset_id = uuid::Uuid::new_v4();
    seed_changeset(&store, changeset_id);

    let mut first = Run::new(changeset_id);
    store.save_run(&first).unwrap();
    first.interrupt().unwrap();
    store
        .persist_run_transition(
            &first,
            SemanticEventKind::Lifecycle {
                state: first.state(),
            },
        )
        .unwrap();

    let second = Run::new(changeset_id);
    store.save_run(&second).unwrap();

    let history = store.runs_for_changeset(changeset_id, 2).unwrap();
    assert_eq!(history, vec![second.clone(), first]);
    assert_eq!(
        store.runs_for_changeset(changeset_id, 1).unwrap(),
        vec![second]
    );
    assert!(matches!(
        store.runs_for_changeset(changeset_id, 0),
        Err(PersistenceError::ResourceLimit("run history page size"))
    ));
    assert!(matches!(
        store.runs_for_changeset(changeset_id, limits::MAX_RUN_HISTORY_PAGE_SIZE + 1),
        Err(PersistenceError::ResourceLimit("run history page size"))
    ));
}

#[test]
fn adopts_user_version_zero_schema_and_preserves_existing_versions() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("legacy.sqlite3");
    let repository = Repository {
        id: uuid::Uuid::new_v4(),
        filesystem_identity: "legacy".into(),
        git_directory_identity: "git-legacy".into(),
        canonical_path: "/legacy".into(),
        identity: "legacy".into(),
        primary_remote: None,
        default_branch: "main".into(),
        base_sha: "base".into(),
        version: 0,
    };
    let mut changeset = Changeset::new(repository.id, "base".into());
    changeset.activate().unwrap();
    let mut run = Run::new(changeset.id);
    run.start().unwrap();
    let mut worktree = Worktree::creating(
        uuid::Uuid::new_v4(),
        changeset.id,
        "/legacy-worktree".into(),
        "legacy-worktree".to_owned(),
        "base".to_owned(),
    );
    worktree.ready().unwrap();

    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE repositories (id TEXT PRIMARY KEY, body TEXT NOT NULL);
             CREATE TABLE changesets (id TEXT PRIMARY KEY, repository_id TEXT NOT NULL REFERENCES repositories(id), state TEXT NOT NULL, body TEXT NOT NULL);
             CREATE TABLE worktrees (id TEXT PRIMARY KEY, changeset_id TEXT NOT NULL REFERENCES changesets(id), body TEXT NOT NULL);
             CREATE INDEX worktrees_by_changeset ON worktrees(changeset_id);
             CREATE TABLE runs (id TEXT PRIMARY KEY, changeset_id TEXT NOT NULL REFERENCES changesets(id), state TEXT NOT NULL, body TEXT NOT NULL);
             CREATE INDEX runs_by_changeset ON runs(changeset_id);
             CREATE UNIQUE INDEX one_active_run_per_changeset ON runs(changeset_id) WHERE state NOT IN ('\"completed\"', '\"interrupted\"', '\"failed\"');
             CREATE TABLE semantic_events (run_id TEXT NOT NULL REFERENCES runs(id), sequence INTEGER NOT NULL, body TEXT NOT NULL, PRIMARY KEY (run_id, sequence));
             CREATE TABLE aggregate_events (aggregate_kind TEXT NOT NULL, aggregate_id TEXT NOT NULL, sequence INTEGER NOT NULL, body TEXT NOT NULL, PRIMARY KEY (aggregate_kind, aggregate_id, sequence));
             CREATE TABLE proposed_changes (owner TEXT PRIMARY KEY REFERENCES runs(id), body TEXT NOT NULL);
             CREATE TABLE approvals (id TEXT PRIMARY KEY, owner TEXT NOT NULL REFERENCES runs(id), body TEXT NOT NULL);
             CREATE INDEX approvals_by_owner ON approvals(owner);
             CREATE TABLE checkpoints (id TEXT PRIMARY KEY, owner TEXT NOT NULL REFERENCES runs(id), body TEXT NOT NULL);
             CREATE INDEX checkpoints_by_owner ON checkpoints(owner);
             CREATE TABLE findings (id TEXT PRIMARY KEY, owner TEXT NOT NULL REFERENCES changesets(id), body TEXT NOT NULL);",
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO repositories (id, body) VALUES (?1, ?2)",
            params![
                repository.id.to_string(),
                serde_json::to_string(&repository).unwrap()
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO changesets (id, repository_id, state, body) VALUES (?1, ?2, ?3, ?4)",
            params![
                changeset.id.to_string(),
                repository.id.to_string(),
                serde_json::to_string(&changeset.state()).unwrap(),
                serde_json::to_string(&changeset).unwrap()
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO runs (id, changeset_id, state, body) VALUES (?1, ?2, ?3, ?4)",
            params![
                run.id.to_string(),
                changeset.id.to_string(),
                serde_json::to_string(&run.state()).unwrap(),
                serde_json::to_string(&run).unwrap()
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO worktrees (id, changeset_id, body) VALUES (?1, ?2, ?3)",
            params![
                worktree.id.to_string(),
                changeset.id.to_string(),
                serde_json::to_string(&worktree).unwrap()
            ],
        )
        .unwrap();
    drop(connection);

    let store = SqliteStore::open(&path).unwrap();
    assert_eq!(
        store.changeset(changeset.id).unwrap(),
        Some(changeset.clone())
    );
    assert_eq!(store.run(run.id).unwrap(), Some(run.clone()));
    assert_eq!(
        store.worktree_for_changeset(changeset.id).unwrap(),
        Some(worktree.clone())
    );

    let connection = Connection::open(&path).unwrap();
    let user_version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    let changeset_version: u64 = connection
        .query_row(
            "SELECT version FROM changesets WHERE id = ?1",
            [changeset.id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    let run_version: u64 = connection
        .query_row(
            "SELECT version FROM runs WHERE id = ?1",
            [run.id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    let (worktree_state, worktree_version): (String, u64) = connection
        .query_row(
            "SELECT state, version FROM worktrees WHERE id = ?1",
            [worktree.id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(user_version, 5);
    assert_eq!(changeset_version, changeset.version());
    assert_eq!(run_version, run.version());
    assert_eq!(
        worktree_state,
        serde_json::to_string(&worktree.state()).unwrap()
    );
    assert_eq!(worktree_version, worktree.version());
}

#[test]
fn version_four_quarantines_legacy_artifact_rows() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("legacy-artifacts.sqlite3");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE worktrees (
                id TEXT PRIMARY KEY,
                changeset_id TEXT NOT NULL,
                body TEXT NOT NULL
            );
            CREATE TABLE artifacts (
                id TEXT PRIMARY KEY,
                changeset_id TEXT NOT NULL,
                run_id TEXT,
                kind TEXT NOT NULL,
                created_at_unix_ms INTEGER NOT NULL,
                body BLOB NOT NULL
            );
            INSERT INTO artifacts (id, changeset_id, run_id, kind, created_at_unix_ms, body)
            VALUES ('legacy-artifact', 'legacy-changeset', 'legacy-run', 'provider_stdout', 42, '{}');
            PRAGMA user_version = 3;",
        )
        .unwrap();
    drop(connection);

    SqliteStore::open(&path).unwrap();
    let connection = Connection::open(&path).unwrap();
    let (status, stored_bytes, updated_at, expires_at): (String, i64, i64, Option<i64>) = connection
        .query_row(
            "SELECT status, stored_bytes, updated_at_unix_ms, expires_at_unix_ms FROM artifacts WHERE id = 'legacy-artifact'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    let user_version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();

    assert_eq!(status, "deleting");
    assert_eq!(stored_bytes, 0);
    assert_eq!(updated_at, 42);
    assert_eq!(expires_at, None);
    assert_eq!(user_version, 5);
}

#[test]
fn rejects_schema_versions_newer_than_the_runtime() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("future.sqlite3");
    let connection = Connection::open(&path).unwrap();
    connection.pragma_update(None, "user_version", 999).unwrap();
    drop(connection);

    assert!(matches!(
        SqliteStore::open(&path),
        Err(PersistenceError::UnsupportedSchemaVersion {
            database: 999,
            runtime: 5
        })
    ));
}

#[test]
fn stale_run_writer_gets_version_conflict() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("stale-run.sqlite3")).unwrap();
    let run = Run::new(uuid::Uuid::new_v4());
    seed_changeset(&store, run.changeset_id());
    store.save_run(&run).unwrap();
    let mut first = run.clone();
    let mut stale = run;
    first.start().unwrap();
    stale.interrupt().unwrap();

    store
        .persist_run_transition(
            &first,
            SemanticEventKind::Lifecycle {
                state: RunState::Starting,
            },
        )
        .unwrap();
    assert!(matches!(
        store.persist_run_transition(
            &stale,
            SemanticEventKind::Lifecycle {
                state: RunState::Interrupted,
            }
        ),
        Err(PersistenceError::VersionConflict {
            aggregate: "run",
            expected: 0,
            actual: 1,
            ..
        })
    ));
}

#[test]
fn stale_changeset_writer_gets_version_conflict() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("stale-changeset.sqlite3")).unwrap();
    let repository = Repository {
        id: uuid::Uuid::new_v4(),
        filesystem_identity: "fixture".into(),
        git_directory_identity: "git-fixture".into(),
        canonical_path: "/fixture".into(),
        identity: "fixture".into(),
        primary_remote: None,
        default_branch: "main".into(),
        base_sha: "base".into(),
        version: 0,
    };
    store.save_repository(&repository).unwrap();
    let changeset = Changeset::new(repository.id, "base".into());
    store.save_changeset(&changeset).unwrap();
    let mut first = changeset.clone();
    let mut stale = changeset;
    first.activate().unwrap();
    stale.activate().unwrap();

    store.persist_changeset_transition(&first).unwrap();
    assert!(matches!(
        store.persist_changeset_transition(&stale),
        Err(PersistenceError::VersionConflict {
            aggregate: "changeset",
            expected: 0,
            actual: 1,
            ..
        })
    ));
}

#[test]
fn stale_worktree_writer_gets_version_conflict() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("stale-worktree.sqlite3")).unwrap();
    let changeset_id = uuid::Uuid::new_v4();
    seed_changeset(&store, changeset_id);
    let worktree = Worktree::creating(
        uuid::Uuid::new_v4(),
        changeset_id,
        "/worktree".into(),
        "fixture".to_owned(),
        "base".to_owned(),
    );
    store.save_worktree(&worktree).unwrap();
    let mut first = worktree.clone();
    let mut stale = worktree;
    first.ready().unwrap();
    stale.fail().unwrap();

    store.update_worktree(&first).unwrap();
    assert!(matches!(
        store.update_worktree(&stale),
        Err(PersistenceError::VersionConflict {
            aggregate: "worktree",
            expected: 0,
            actual: 1,
            ..
        })
    ));
}

#[test]
fn worktree_replay_is_a_noop_and_removal_records_both_transitions() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("worktree-events.sqlite3");
    let store = SqliteStore::open(&path).unwrap();
    let changeset_id = uuid::Uuid::new_v4();
    seed_changeset(&store, changeset_id);
    let mut worktree = Worktree::creating(
        uuid::Uuid::new_v4(),
        changeset_id,
        "/worktree".into(),
        "fixture".to_owned(),
        "base".to_owned(),
    );
    worktree.ready().unwrap();
    store.save_worktree(&worktree).unwrap();
    store.update_worktree(&worktree).unwrap();

    let mut altered = worktree.clone();
    altered.path = "/different-worktree".into();
    assert!(matches!(
        store.update_worktree(&altered),
        Err(PersistenceError::InvalidPersistedTransition("worktree"))
    ));

    worktree.begin_removal().unwrap();
    worktree.removed().unwrap();
    store.update_worktree(&worktree).unwrap();
    let connection = Connection::open(path).unwrap();
    let mut statement = connection
        .prepare(
            "SELECT body FROM aggregate_events
             WHERE aggregate_kind = 'worktree' AND aggregate_id = ?1
             ORDER BY sequence",
        )
        .unwrap();
    let events = statement
        .query_map([worktree.id.to_string()], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        events,
        vec![
            "{\"state\":\"removing\"}".to_owned(),
            "{\"state\":\"removed\"}".to_owned(),
        ]
    );
}

fn seed_lease_runs(store: &SqliteStore) -> (uuid::Uuid, Run, Run) {
    let changeset_id = uuid::Uuid::new_v4();
    seed_changeset(store, changeset_id);
    let mut first = Run::new(changeset_id);
    store.save_run(&first).unwrap();
    first.interrupt().unwrap();
    store
        .persist_run_transition(
            &first,
            SemanticEventKind::Lifecycle {
                state: RunState::Interrupted,
            },
        )
        .unwrap();
    let second = Run::new(changeset_id);
    store.save_run(&second).unwrap();
    (changeset_id, first, second)
}

#[test]
fn mutation_lease_rejects_an_unexpired_competing_owner() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("lease-contention.sqlite3")).unwrap();
    let (changeset_id, first, second) = seed_lease_runs(&store);
    let lease = store
        .acquire_mutation_lease(changeset_id, first.id, 1_000, Duration::from_secs(10))
        .unwrap();

    assert!(matches!(
        store.acquire_mutation_lease(
            changeset_id,
            second.id,
            1_001,
            Duration::from_secs(10)
        ),
        Err(PersistenceError::MutationLeaseHeld { run_id, .. }) if run_id == first.id
    ));
    assert_eq!(store.mutation_lease(changeset_id).unwrap(), Some(lease));
}

#[test]
fn expired_lease_takeover_fences_the_stale_owner() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("lease-takeover.sqlite3")).unwrap();
    let (changeset_id, first, second) = seed_lease_runs(&store);
    let first_lease = store
        .acquire_mutation_lease(changeset_id, first.id, 1_000, Duration::from_millis(10))
        .unwrap();
    let second_lease = store
        .acquire_mutation_lease(changeset_id, second.id, 1_010, Duration::from_secs(10))
        .unwrap();

    assert_eq!(second_lease.fencing_epoch, first_lease.fencing_epoch + 1);
    assert_eq!(
        store
            .verify_mutation_lease(changeset_id, second.id, second_lease.fencing_epoch, 1_011)
            .unwrap(),
        second_lease
    );
    assert!(
        !store
            .release_mutation_lease(changeset_id, first.id, first_lease.fencing_epoch)
            .unwrap()
    );
    assert_eq!(
        store.mutation_lease(changeset_id).unwrap(),
        Some(second_lease.clone())
    );
    assert!(
        store
            .release_mutation_lease(changeset_id, second.id, second_lease.fencing_epoch)
            .unwrap()
    );
    assert_eq!(store.mutation_lease(changeset_id).unwrap(), None);
}

#[test]
fn mutation_lease_is_serializable() {
    let lease = MutationLease {
        changeset_id: uuid::Uuid::new_v4(),
        run_id: uuid::Uuid::new_v4(),
        fencing_epoch: 7,
        expires_at_unix_ms: 42,
    };
    let encoded = serde_json::to_string(&lease).unwrap();
    assert_eq!(
        serde_json::from_str::<MutationLease>(&encoded).unwrap(),
        lease
    );
}

#[test]
fn recovery_actions_are_idempotent_and_reject_conflicting_reuse() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("recovery-actions.sqlite3")).unwrap();
    let aggregate_id = uuid::Uuid::new_v4();
    let action = RecoveryAction {
        aggregate_kind: "run".to_owned(),
        aggregate_id,
        action: "interrupted".to_owned(),
        detail: "startup interrupted the run".to_owned(),
    };

    let first = store
        .record_recovery_action("run:interrupted", None, None, action.clone(), 10)
        .unwrap();
    let repeated = store
        .record_recovery_action("run:interrupted", None, None, action, 20)
        .unwrap();
    assert_eq!(repeated, first);
    assert_eq!(store.recovery_actions().unwrap(), vec![first]);

    let conflicting = RecoveryAction {
        aggregate_kind: "run".to_owned(),
        aggregate_id,
        action: "failed".to_owned(),
        detail: "different recovery result".to_owned(),
    };
    assert!(matches!(
        store.record_recovery_action("run:interrupted", None, None, conflicting, 30),
        Err(PersistenceError::RecoveryActionConflict)
    ));
    assert!(matches!(
        store.record_recovery_action(
            "",
            None,
            None,
            RecoveryAction {
                aggregate_kind: "run".to_owned(),
                aggregate_id,
                action: "ignored".to_owned(),
                detail: "invalid key".to_owned(),
            },
            40,
        ),
        Err(PersistenceError::InvalidRecoveryAction)
    ));
}

#[test]
fn process_supervision_transitions_prepared_running_terminated() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("supervision.sqlite3")).unwrap();
    let run = Run::new(uuid::Uuid::new_v4());
    seed_changeset(&store, run.changeset_id());
    store.save_run(&run).unwrap();

    let prepared = supervision_metadata(SupervisionState::Prepared);
    let inserted = store
        .insert_prepared_process_supervision(run.id, &prepared, 10)
        .unwrap();
    assert_eq!(inserted.metadata, prepared);

    let mut running = prepared.clone();
    running.state = SupervisionState::Running;
    let running_record = store
        .mark_process_supervision_running(run.id, &running, 20)
        .unwrap();
    assert_eq!(running_record.metadata.state, SupervisionState::Running);

    let mut terminated = running.clone();
    terminated.state = SupervisionState::Terminated;
    terminated.termination_reason = Some(TerminationReason::Exited { code: 0 });
    let terminated_record = store
        .mark_process_supervision_terminated(run.id, &terminated, 30)
        .unwrap();
    assert_eq!(
        store
            .process_supervision_for_run(run.id, prepared.supervision_id)
            .unwrap(),
        Some(terminated_record.clone())
    );
    assert_eq!(
        store.latest_process_supervision_for_run(run.id).unwrap(),
        Some(terminated_record)
    );
    assert!(
        store
            .nonterminated_process_supervisions()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn process_supervision_rejects_invalid_transition_and_identity() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("invalid-supervision.sqlite3")).unwrap();
    let run = Run::new(uuid::Uuid::new_v4());
    seed_changeset(&store, run.changeset_id());
    store.save_run(&run).unwrap();
    let prepared = supervision_metadata(SupervisionState::Prepared);
    store
        .insert_prepared_process_supervision(run.id, &prepared, 10)
        .unwrap();

    let mut running = prepared.clone();
    running.state = SupervisionState::Running;
    store
        .mark_process_supervision_running(run.id, &running, 20)
        .unwrap();
    assert!(matches!(
        store.mark_process_supervision_running(run.id, &running, 30),
        Err(PersistenceError::InvalidProcessSupervisionTransition)
    ));

    let mut terminated = running;
    terminated.state = SupervisionState::Terminated;
    terminated.termination_reason = Some(TerminationReason::Requested);
    terminated.pid += 1;
    assert!(matches!(
        store.mark_process_supervision_terminated(run.id, &terminated, 30),
        Err(PersistenceError::InvalidProcessSupervisionTransition)
    ));
}

#[test]
fn active_process_supervision_listing_is_deterministic() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("active-supervision.sqlite3")).unwrap();
    let first_run = Run::new(uuid::Uuid::new_v4());
    let second_run = Run::new(uuid::Uuid::new_v4());
    seed_changeset(&store, first_run.changeset_id());
    seed_changeset(&store, second_run.changeset_id());
    store.save_run(&first_run).unwrap();
    store.save_run(&second_run).unwrap();

    let first = supervision_metadata(SupervisionState::Prepared);
    store
        .insert_prepared_process_supervision(first_run.id, &first, 20)
        .unwrap();
    let second = supervision_metadata(SupervisionState::Prepared);
    store
        .insert_prepared_process_supervision(second_run.id, &second, 10)
        .unwrap();
    let mut second_running = second.clone();
    second_running.state = SupervisionState::Running;
    store
        .mark_process_supervision_running(second_run.id, &second_running, 30)
        .unwrap();

    let active = store.nonterminated_process_supervisions().unwrap();
    assert_eq!(active.len(), 2);
    assert_eq!(active[0].run_id, first_run.id);
    assert_eq!(active[1].run_id, second_run.id);
    assert_eq!(active[1].metadata.state, SupervisionState::Running);
}

#[test]
fn findings_reload_in_creation_order_with_current_versions() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("findings.sqlite3")).unwrap();
    let changeset_id = uuid::Uuid::new_v4();
    seed_changeset(&store, changeset_id);
    let first = finding(changeset_id, "first.txt");
    let mut second = finding(changeset_id, "second.txt");
    store
        .save_findings(&[first.clone(), second.clone()])
        .unwrap();
    store
        .save_findings(&[first.clone(), second.clone()])
        .unwrap();
    second.resolve().unwrap();
    store.persist_finding_transition(&second).unwrap();

    let findings = store.findings_for_changeset(changeset_id).unwrap();
    assert_eq!(findings, vec![first, second]);
}

#[test]
fn finding_batches_are_atomic_and_bounded() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("bounded-findings.sqlite3")).unwrap();
    let changeset_id = uuid::Uuid::new_v4();
    seed_changeset(&store, changeset_id);
    let findings = (0..=limits::MAX_FINDINGS_PER_CHANGESET)
        .map(|index| finding(changeset_id, &format!("finding-{index}.txt")))
        .collect::<Vec<_>>();

    assert!(matches!(
        store.save_findings(&findings),
        Err(PersistenceError::ResourceLimit("findings per changeset"))
    ));
    assert!(
        store
            .findings_for_changeset(changeset_id)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn finalization_preparation_is_exact_replay_safe_and_ordered() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("finalization-prepare.sqlite3")).unwrap();
    let first = seed_finalization_fixture(&store, ChangesetMutationKind::Commit);
    let first_digest = first.scope.digest();
    let prepared = store
        .prepare_changeset_finalization(
            &first_digest,
            &first.scope,
            first.worktree.id,
            first.worktree.version(),
            20,
        )
        .unwrap();
    assert_eq!(prepared.state, ChangesetFinalizationState::Prepared);
    assert_eq!(
        store
            .prepare_changeset_finalization(
                &first_digest,
                &first.scope,
                first.worktree.id,
                first.worktree.version(),
                99,
            )
            .unwrap(),
        prepared
    );
    assert!(matches!(
        store.prepare_changeset_finalization(
            "wrong",
            &first.scope,
            first.worktree.id,
            first.worktree.version(),
            30,
        ),
        Err(PersistenceError::FinalizationDigestMismatch)
    ));
    let mut conflicting_scope = first.scope.clone();
    conflicting_scope.manifest_sha256 = "e".repeat(64);
    assert!(matches!(
        store.prepare_changeset_finalization(
            &conflicting_scope.digest(),
            &conflicting_scope,
            first.worktree.id,
            first.worktree.version(),
            30,
        ),
        Err(PersistenceError::FinalizationConflict)
    ));

    let second = seed_finalization_fixture(&store, ChangesetMutationKind::Discard);
    store
        .prepare_changeset_finalization(
            &second.scope.digest(),
            &second.scope,
            second.worktree.id,
            second.worktree.version(),
            10,
        )
        .unwrap();
    let prepared = store.prepared_changeset_finalizations().unwrap();
    assert_eq!(prepared.len(), 2);
    assert_eq!(prepared[0].scope.changeset_id, second.changeset.id);
    assert_eq!(prepared[1].scope.changeset_id, first.changeset.id);
}

#[test]
fn prepared_finalization_recovery_detects_scalar_status_corruption() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("finalization-corrupt-status.sqlite3");
    let store = SqliteStore::open(&path).unwrap();
    let fixture = seed_finalization_fixture(&store, ChangesetMutationKind::Commit);
    let digest = fixture.scope.digest();
    store
        .prepare_changeset_finalization(
            &digest,
            &fixture.scope,
            fixture.worktree.id,
            fixture.worktree.version(),
            10,
        )
        .unwrap();
    Connection::open(path)
        .unwrap()
        .execute(
            "UPDATE changeset_finalizations SET status = ?2 WHERE confirmation_digest = ?1",
            params![digest, "\"completed\""],
        )
        .unwrap();

    assert!(matches!(
        store.prepared_changeset_finalizations(),
        Err(PersistenceError::CorruptFinalization)
    ));
}

#[test]
fn commit_finalization_updates_all_records_atomically_and_replays() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("finalization-commit.sqlite3")).unwrap();
    let fixture = seed_finalization_fixture(&store, ChangesetMutationKind::Commit);
    let digest = fixture.scope.digest();
    store
        .prepare_changeset_finalization(
            &digest,
            &fixture.scope,
            fixture.worktree.id,
            fixture.worktree.version(),
            10,
        )
        .unwrap();
    assert!(matches!(
        store.complete_changeset_finalization(&digest, ChangesetFinalizationResult::Discard, 20,),
        Err(PersistenceError::FinalizationKindMismatch)
    ));

    let result = ChangesetFinalizationResult::Commit {
        resulting_head_sha: "b".repeat(40),
        app_ref: format!("refs/jet-black/changesets/{}", fixture.changeset.id),
    };
    assert!(matches!(
        store.complete_changeset_finalization(
            &digest,
            ChangesetFinalizationResult::Commit {
                resulting_head_sha: "b".repeat(40),
                app_ref: "refs/jet-black/changesets/wrong".to_owned(),
            },
            30,
        ),
        Err(PersistenceError::InvalidFinalizationInput("commit result"))
    ));
    let completed = store
        .complete_changeset_finalization(&digest, result.clone(), 30)
        .unwrap();
    assert_eq!(
        completed.finalization.state,
        ChangesetFinalizationState::Completed
    );
    assert_eq!(completed.finalization.result, Some(result.clone()));
    assert_eq!(
        completed.changeset.state(),
        domain::ChangesetState::Committed
    );
    assert_eq!(completed.changeset.head_sha(), "b".repeat(40));
    assert_eq!(completed.worktree.state(), WorktreeState::Removed);
    assert_eq!(
        store.completed_changeset_finalization(&digest).unwrap(),
        completed
    );
    assert_eq!(
        store
            .complete_changeset_finalization(&digest, result, 40)
            .unwrap(),
        completed
    );
    assert_eq!(
        store
            .prepare_changeset_finalization(
                &digest,
                &fixture.scope,
                fixture.worktree.id,
                fixture.worktree.version(),
                50,
            )
            .unwrap(),
        completed.finalization
    );
    assert_eq!(
        store.changeset(fixture.changeset.id).unwrap(),
        Some(completed.changeset)
    );
    assert_eq!(
        store.worktree_for_changeset(fixture.changeset.id).unwrap(),
        Some(completed.worktree)
    );
}

#[test]
fn discard_and_divergent_finalizations_persist_terminal_aggregate_states() {
    let directory = tempdir().unwrap();
    let store = SqliteStore::open(directory.path().join("finalization-terminal.sqlite3")).unwrap();
    let discard = seed_finalization_fixture(&store, ChangesetMutationKind::Discard);
    let discard_digest = discard.scope.digest();
    store
        .prepare_changeset_finalization(
            &discard_digest,
            &discard.scope,
            discard.worktree.id,
            discard.worktree.version(),
            10,
        )
        .unwrap();
    let discarded = store
        .complete_changeset_finalization(&discard_digest, ChangesetFinalizationResult::Discard, 20)
        .unwrap();
    assert_eq!(
        discarded.changeset.state(),
        domain::ChangesetState::Discarded
    );
    assert_eq!(discarded.worktree.state(), WorktreeState::Removed);

    let divergent = seed_finalization_fixture(&store, ChangesetMutationKind::Commit);
    let divergent_digest = divergent.scope.digest();
    store
        .prepare_changeset_finalization(
            &divergent_digest,
            &divergent.scope,
            divergent.worktree.id,
            divergent.worktree.version(),
            30,
        )
        .unwrap();
    let detail = "application ref points to a different commit".to_owned();
    assert!(matches!(
        store.mark_changeset_finalization_divergent(
            &divergent_digest,
            "not-a-sha".to_owned(),
            detail.clone(),
            40,
        ),
        Err(PersistenceError::InvalidFinalizationInput(
            "observed head sha"
        ))
    ));
    let marked = store
        .mark_changeset_finalization_divergent(
            &divergent_digest,
            "c".repeat(40),
            detail.clone(),
            40,
        )
        .unwrap();
    assert_eq!(
        marked.finalization.state,
        ChangesetFinalizationState::Divergent
    );
    assert_eq!(marked.changeset.state(), domain::ChangesetState::Divergent);
    assert_eq!(marked.changeset.head_sha(), "c".repeat(40));
    assert_eq!(marked.worktree.state(), WorktreeState::Quarantined);
    assert_eq!(
        store
            .mark_changeset_finalization_divergent(
                &divergent_digest,
                "c".repeat(40),
                detail.clone(),
                50,
            )
            .unwrap(),
        marked
    );

    let mut resolved = marked.changeset.clone();
    resolved.discard().unwrap();
    store.persist_changeset_transition(&resolved).unwrap();
    assert!(matches!(
        store.mark_changeset_finalization_divergent(&divergent_digest, "c".repeat(40), detail, 60,),
        Err(PersistenceError::FinalizationConflict)
    ));
}
