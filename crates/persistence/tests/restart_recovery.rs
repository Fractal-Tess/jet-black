use domain::{Changeset, Finding, RelativePath, Repository, Run, RunState, limits};
use execution::{
    ExecutableIdentity, ProcessGroupIdentity, ProcessStartIdentity, SupervisionMetadata,
    SupervisionState, TerminationReason,
};
use persistence::{MutationLease, PersistenceError, SqliteStore};
use protocol::{RecoveryAction, SemanticEventKind};
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
    drop(connection);

    let store = SqliteStore::open(&path).unwrap();
    assert_eq!(
        store.changeset(changeset.id).unwrap(),
        Some(changeset.clone())
    );
    assert_eq!(store.run(run.id).unwrap(), Some(run.clone()));

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
    assert_eq!(user_version, 4);
    assert_eq!(changeset_version, changeset.version());
    assert_eq!(run_version, run.version());
}

#[test]
fn version_four_quarantines_legacy_artifact_rows() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("legacy-artifacts.sqlite3");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE artifacts (
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
    assert_eq!(user_version, 4);
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
            runtime: 4
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
