mod artifacts;

pub use artifacts::{
    ArtifactPolicy, ArtifactSegment, ArtifactState, ArtifactStream, LocalArtifactStore,
    RunArtifact, VerifiedArtifactSegment,
};

use domain::{
    ActionProposal, Approval, ApprovalScope, ApprovedAction, Changeset, ChangesetMutationKind,
    ChangesetMutationScope, ChangesetState, Checkpoint, Finding, Repository, Run, RunState,
    Worktree, WorktreeState,
};
use execution::{SupervisionMetadata, SupervisionState};
use fs2::FileExt;
use protocol::{OrderedRunEvent, RecoveryAction, SemanticEventKind};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    time::Duration,
};
use thiserror::Error;
use uuid::Uuid;

const CURRENT_SCHEMA_VERSION: u32 = 5;

#[derive(Debug, Clone)]
pub struct SqliteStore {
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredRunSnapshot {
    pub repository: Repository,
    pub changeset: Changeset,
    pub run: Run,
    pub worktree: Option<Worktree>,
    pub checkpoint: Option<Checkpoint>,
    pub pending_approval: Option<Approval>,
    pub findings: Vec<Finding>,
    pub events: Vec<OrderedRunEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangesetFinalizationState {
    Prepared,
    Completed,
    Divergent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChangesetFinalizationResult {
    Commit {
        resulting_head_sha: String,
        app_ref: String,
    },
    Discard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangesetFinalization {
    pub confirmation_digest: String,
    pub scope: ChangesetMutationScope,
    pub worktree_id: Uuid,
    pub expected_worktree_version: u64,
    pub state: ChangesetFinalizationState,
    pub result: Option<ChangesetFinalizationResult>,
    pub divergence_detail: Option<String>,
    pub version: u64,
    pub created_at_unix_ms: i64,
    pub updated_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedChangesetFinalization {
    pub finalization: ChangesetFinalization,
    pub changeset: Changeset,
    pub worktree: Worktree,
}

pub struct RecoveryLock {
    file: File,
}

impl Drop for RecoveryLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

impl SqliteStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PersistenceError> {
        let store = Self {
            path: path.as_ref().to_path_buf(),
        };
        store.with_connection(migrate)?;
        Ok(store)
    }

    pub fn acquire_recovery_lock(&self) -> Result<RecoveryLock, PersistenceError> {
        let mut lock_path = self.path.as_os_str().to_os_string();
        lock_path.push(".recovery.lock");
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(PathBuf::from(lock_path))?;
        file.lock_exclusive()?;
        Ok(RecoveryLock { file })
    }

    fn with_connection<T>(
        &self,
        operation: impl FnOnce(&mut Connection) -> Result<T, PersistenceError>,
    ) -> Result<T, PersistenceError> {
        let mut connection = Connection::open(&self.path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        operation(&mut connection)
    }

    pub fn save_repository(&self, repository: &Repository) -> Result<(), PersistenceError> {
        self.with_connection(|connection| {
            connection.execute(
                "INSERT INTO repositories (id, body) VALUES (?1, ?2)",
                params![repository.id.to_string(), encode(repository)?],
            )?;
            Ok(())
        })
    }

    pub fn repository(&self, repository_id: Uuid) -> Result<Option<Repository>, PersistenceError> {
        self.with_connection(|connection| {
            let body = connection
                .query_row(
                    "SELECT body FROM repositories WHERE id = ?1",
                    [repository_id.to_string()],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            body.map(|value| decode(&value)).transpose()
        })
    }

    pub fn save_changeset(&self, changeset: &Changeset) -> Result<(), PersistenceError> {
        self.with_connection(|connection| {
            connection.execute(
                "INSERT INTO changesets (id, repository_id, state, version, body) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    changeset.id.to_string(),
                    changeset.repository_id().to_string(),
                    state_json(&changeset.state())?,
                    changeset.version(),
                    encode(changeset)?
                ],
            )?;
            Ok(())
        })
    }

    pub fn persist_changeset_transition(
        &self,
        changeset: &Changeset,
    ) -> Result<(), PersistenceError> {
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let (stored, stored_version) = load_changeset(&transaction, changeset.id)?;
            validate_state_transition(
                "changeset",
                changeset.id,
                stored.state(),
                changeset.state(),
                stored_version,
                changeset.version(),
                |state, next| state.allows(next),
            )?;
            update_changeset(&transaction, changeset, stored_version)?;
            append_aggregate_event(
                &transaction,
                "changeset",
                changeset.id,
                &format!("{:?}", changeset.state()).to_lowercase(),
            )?;
            transaction.commit()?;
            Ok(())
        })
    }

    pub fn changeset(&self, changeset_id: Uuid) -> Result<Option<Changeset>, PersistenceError> {
        self.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT body, version FROM changesets WHERE id = ?1",
                    [changeset_id.to_string()],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?)),
                )
                .optional()?
                .map(|(body, scalar_version)| {
                    let changeset: Changeset = decode(&body)?;
                    ensure_version_agreement(
                        "changeset",
                        changeset_id,
                        scalar_version,
                        changeset.version(),
                    )?;
                    Ok(changeset)
                })
                .transpose()
        })
    }

    pub fn changesets(&self) -> Result<Vec<Changeset>, PersistenceError> {
        self.with_connection(|connection| {
            let mut statement =
                connection.prepare("SELECT body, version FROM changesets ORDER BY rowid")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
            })?;
            rows.map(|row| {
                let (body, scalar_version) = row?;
                let changeset: Changeset = decode(&body)?;
                ensure_version_agreement(
                    "changeset",
                    changeset.id,
                    scalar_version,
                    changeset.version(),
                )?;
                Ok(changeset)
            })
            .collect()
        })
    }

    pub fn save_worktree(&self, worktree: &Worktree) -> Result<(), PersistenceError> {
        self.with_connection(|connection| {
            connection.execute(
                "INSERT INTO worktrees (id, changeset_id, state, version, body) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    worktree.id.to_string(),
                    worktree.changeset_id().to_string(),
                    state_json(&worktree.state())?,
                    worktree.version(),
                    encode(worktree)?
                ],
            )?;
            Ok(())
        })
    }

    pub fn worktree_for_changeset(
        &self,
        changeset_id: Uuid,
    ) -> Result<Option<Worktree>, PersistenceError> {
        self.with_connection(|connection| load_latest_worktree(connection, changeset_id))
    }

    pub fn worktree(&self, worktree_id: Uuid) -> Result<Option<Worktree>, PersistenceError> {
        self.with_connection(|connection| load_optional_worktree(connection, worktree_id))
    }

    pub fn update_worktree(&self, worktree: &Worktree) -> Result<(), PersistenceError> {
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let (stored, stored_version) = load_worktree(&transaction, worktree.id)?;
            if stored == *worktree {
                return Ok(());
            }
            validate_worktree_transition(&stored, worktree)?;
            if worktree.version() == stored_version.saturating_add(2) {
                let mut removing = stored;
                removing.begin_removal()?;
                update_worktree(&transaction, &removing, stored_version)?;
                append_aggregate_event(&transaction, "worktree", removing.id, "removing")?;
                update_worktree(&transaction, worktree, removing.version())?;
            } else {
                update_worktree(&transaction, worktree, stored_version)?;
            }
            append_aggregate_event(
                &transaction,
                "worktree",
                worktree.id,
                &format!("{:?}", worktree.state()).to_lowercase(),
            )?;
            transaction.commit()?;
            Ok(())
        })
    }

    pub fn save_run(&self, run: &Run) -> Result<(), PersistenceError> {
        self.with_connection(|connection| {
            connection.execute(
                "INSERT INTO runs (id, changeset_id, state, version, body) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    run.id.to_string(),
                    run.changeset_id().to_string(),
                    state_json(&run.state())?,
                    run.version(),
                    encode(run)?
                ],
            )?;
            Ok(())
        })
    }

    pub fn persist_run_transition(
        &self,
        run: &Run,
        event: SemanticEventKind,
    ) -> Result<OrderedRunEvent, PersistenceError> {
        self.persist_run_transition_from(run, event, None)
    }

    pub fn claim_run_for_drive(&self, run: &Run) -> Result<OrderedRunEvent, PersistenceError> {
        if run.state() != RunState::Running {
            return Err(PersistenceError::InvalidPersistedTransition(
                "run drive claim",
            ));
        }
        self.persist_run_transition_from(
            run,
            SemanticEventKind::Lifecycle {
                state: RunState::Running,
            },
            Some(RunState::Starting),
        )
    }

    fn persist_run_transition_from(
        &self,
        run: &Run,
        event: SemanticEventKind,
        expected_source: Option<RunState>,
    ) -> Result<OrderedRunEvent, PersistenceError> {
        validate_semantic_event(&event)?;
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let (stored, stored_version) = load_run(&transaction, run.id)?;
            if expected_source.is_some_and(|expected| stored.state() != expected) {
                return Err(PersistenceError::RunDriveAlreadyClaimed);
            }
            validate_versioned_transition(
                "run",
                run.id,
                stored.state(),
                run.state(),
                stored_version,
                run.version(),
                |state, next| {
                    !(state == RunState::AwaitingApproval && next == RunState::Running)
                        && state.allows(next)
                },
            )?;
            let sequence: u64 = transaction.query_row(
                "SELECT COALESCE(MAX(sequence), 0) + 1 FROM semantic_events WHERE run_id = ?1",
                [run.id.to_string()],
                |row| row.get(0),
            )?;
            update_run(&transaction, run, stored_version)?;
            let ordered = OrderedRunEvent {
                run_id: run.id,
                sequence,
                event,
            };
            transaction.execute(
                "INSERT INTO semantic_events (run_id, sequence, body) VALUES (?1, ?2, ?3)",
                params![run.id.to_string(), sequence, encode(&ordered)?],
            )?;
            transaction.commit()?;
            Ok(ordered)
        })
    }

    pub fn request_approval(
        &self,
        run: &Run,
        approval: &Approval,
        event: SemanticEventKind,
    ) -> Result<OrderedRunEvent, PersistenceError> {
        validate_semantic_event(&event)?;
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let (stored, stored_version) = load_run(&transaction, run.id)?;
            validate_versioned_transition(
                "run",
                run.id,
                stored.state(),
                run.state(),
                stored_version,
                run.version(),
                |state, next| state.allows(next),
            )?;
            if stored.state() != RunState::Running {
                return Err(PersistenceError::ApprovalDoesNotMatchRun);
            }
            validate_pending_approval(run, approval)?;
            let sequence: u64 = transaction.query_row(
                "SELECT COALESCE(MAX(sequence), 0) + 1 FROM semantic_events WHERE run_id = ?1",
                [run.id.to_string()],
                |row| row.get(0),
            )?;
            let ordered = OrderedRunEvent {
                run_id: run.id,
                sequence,
                event,
            };
            update_run(&transaction, run, stored_version)?;
            transaction.execute(
                "INSERT INTO approvals (id, owner, body) VALUES (?1, ?2, ?3)",
                params![
                    approval.id.to_string(),
                    run.id.to_string(),
                    encode(approval)?
                ],
            )?;
            transaction.execute(
                "INSERT INTO semantic_events (run_id, sequence, body) VALUES (?1, ?2, ?3)",
                params![run.id.to_string(), sequence, encode(&ordered)?],
            )?;
            transaction.commit()?;
            Ok(ordered)
        })
    }

    pub fn events(&self, run_id: Uuid) -> Result<Vec<OrderedRunEvent>, PersistenceError> {
        self.with_connection(|connection| {
            let mut statement = connection
                .prepare("SELECT body FROM semantic_events WHERE run_id = ?1 ORDER BY sequence")?;
            let rows = statement.query_map([run_id.to_string()], |row| row.get::<_, String>(0))?;
            rows.map(|row| decode(&row?)).collect()
        })
    }

    pub fn events_after(
        &self,
        run_id: Uuid,
        after_sequence: u64,
        limit: usize,
    ) -> Result<Vec<OrderedRunEvent>, PersistenceError> {
        if limit == 0 || limit > domain::limits::MAX_EVENT_PAGE_SIZE {
            return Err(PersistenceError::ResourceLimit("event page size"));
        }
        self.with_connection(|connection| {
            query_bounded_events(connection, run_id, after_sequence, limit)
        })
    }

    pub fn run(&self, run_id: Uuid) -> Result<Option<Run>, PersistenceError> {
        self.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT body, version FROM runs WHERE id = ?1",
                    [run_id.to_string()],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?)),
                )
                .optional()?
                .map(|(body, scalar_version)| {
                    let run: Run = decode(&body)?;
                    ensure_version_agreement("run", run_id, scalar_version, run.version())?;
                    Ok(run)
                })
                .transpose()
        })
    }

    pub fn runs(&self) -> Result<Vec<Run>, PersistenceError> {
        self.with_connection(|connection| {
            let mut statement =
                connection.prepare("SELECT body, version FROM runs ORDER BY rowid")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
            })?;
            rows.map(|row| {
                let (body, scalar_version) = row?;
                let run: Run = decode(&body)?;
                ensure_version_agreement("run", run.id, scalar_version, run.version())?;
                Ok(run)
            })
            .collect()
        })
    }

    pub fn runs_for_changeset(
        &self,
        changeset_id: Uuid,
        limit: usize,
    ) -> Result<Vec<Run>, PersistenceError> {
        if limit == 0 || limit > domain::limits::MAX_RUN_HISTORY_PAGE_SIZE {
            return Err(PersistenceError::ResourceLimit("run history page size"));
        }
        self.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, changeset_id, body, version FROM runs WHERE changeset_id = ?1 ORDER BY rowid DESC LIMIT ?2",
            )?;
            let rows = statement.query_map(
                params![changeset_id.to_string(), limit as i64],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, u64>(3)?,
                    ))
                },
            )?;
            rows.map(|row| {
                let (scalar_run_id, scalar_changeset_id, body, scalar_version) = row?;
                let run_id = Uuid::parse_str(&scalar_run_id)
                    .map_err(|_| PersistenceError::CorruptIdentifier("run id"))?;
                let run = decode_versioned_run(run_id, &body, scalar_version)?;
                if scalar_changeset_id != changeset_id.to_string()
                    || run.changeset_id() != changeset_id
                {
                    return Err(PersistenceError::CorruptOwnership("run"));
                }
                Ok(run)
            })
            .collect()
        })
    }

    pub fn run_snapshot(
        &self,
        run_id: Uuid,
    ) -> Result<Option<StoredRunSnapshot>, PersistenceError> {
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Deferred)?;
            let Some((run_body, run_version)) = transaction
                .query_row(
                    "SELECT body, version FROM runs WHERE id = ?1",
                    [run_id.to_string()],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?)),
                )
                .optional()?
            else {
                return Ok(None);
            };
            let run = decode_versioned_run(run_id, &run_body, run_version)?;
            let (changeset, _) = load_changeset(&transaction, run.changeset_id())?;
            if run.changeset_id() != changeset.id {
                return Err(PersistenceError::CorruptOwnership("run"));
            }
            let repository = load_repository(&transaction, changeset.repository_id())?;
            let latest_run_id = transaction.query_row(
                "SELECT id FROM runs WHERE changeset_id = ?1 ORDER BY rowid DESC LIMIT 1",
                [changeset.id.to_string()],
                |row| row.get::<_, String>(0),
            )?;
            let worktree = if latest_run_id == run.id.to_string() {
                load_latest_worktree(&transaction, changeset.id)?
            } else {
                None
            };
            let checkpoint = load_latest_checkpoint(&transaction, run.id)?;
            let pending_approval = load_pending_approval(&transaction, &run)?;
            let findings = load_findings(&transaction, changeset.id)?;
            let events = query_bounded_events(
                &transaction,
                run.id,
                0,
                domain::limits::MAX_SNAPSHOT_EVENT_PAGE_SIZE,
            )?;
            transaction.commit()?;
            Ok(Some(StoredRunSnapshot {
                repository,
                changeset,
                run,
                worktree,
                checkpoint,
                pending_approval,
                findings,
                events,
            }))
        })
    }

    pub fn save_proposed_change(
        &self,
        run: &Run,
        proposal: &ActionProposal,
        content: &[u8],
    ) -> Result<(), PersistenceError> {
        if content.len() > domain::limits::MAX_APPROVED_FILE_BYTES {
            return Err(PersistenceError::ResourceLimit("approved file content"));
        }
        let proposed_change = encode(&StoredProposedChange {
            proposal: proposal.clone(),
            content: content.to_vec(),
        })?;
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let (stored, stored_version) = load_run(&transaction, run.id)?;
            if stored_version != run.version() {
                return Err(PersistenceError::VersionConflict {
                    aggregate: "run",
                    id: run.id,
                    expected: run.version(),
                    actual: stored_version,
                });
            }
            if stored != *run || run.state() != RunState::Running {
                return Err(PersistenceError::InvalidPersistedTransition(
                    "proposed change",
                ));
            }
            transaction.execute(
                "INSERT INTO proposed_changes (owner, body) VALUES (?1, ?2)",
                params![run.id.to_string(), proposed_change],
            )?;
            transaction.commit()?;
            Ok(())
        })
    }

    pub fn proposed_change_for_run(
        &self,
        run_id: Uuid,
    ) -> Result<Option<StoredProposedChange>, PersistenceError> {
        self.with_connection(|connection| {
            let body = connection
                .query_row(
                    "SELECT body FROM proposed_changes WHERE owner = ?1",
                    [run_id.to_string()],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            body.map(|value| decode(&value)).transpose()
        })
    }

    pub fn delete_proposed_change_for_run(&self, run_id: Uuid) -> Result<(), PersistenceError> {
        self.with_connection(|connection| {
            connection.execute(
                "DELETE FROM proposed_changes WHERE owner = ?1",
                [run_id.to_string()],
            )?;
            Ok(())
        })
    }

    pub fn save_approval(&self, approval: &Approval) -> Result<(), PersistenceError> {
        self.with_connection(|connection| {
            connection.execute(
                "INSERT INTO approvals (id, owner, body) VALUES (?1, ?2, ?3)",
                params![
                    approval.id.to_string(),
                    approval.run_id().to_string(),
                    encode(approval)?
                ],
            )?;
            Ok(())
        })
    }

    pub fn approval_for_run(&self, run_id: Uuid) -> Result<Option<Approval>, PersistenceError> {
        self.with_connection(|connection| load_latest_approval(connection, run_id))
    }

    pub fn reject_and_interrupt(
        &self,
        run_id: Uuid,
        approval_id: Uuid,
    ) -> Result<Run, PersistenceError> {
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let (mut run, stored_version) = load_run(&transaction, run_id)?;
            let approval = load_approval(&transaction, run_id, approval_id)?;
            validate_pending_approval(&run, &approval)?;
            run.interrupt()?;
            validate_versioned_transition(
                "run",
                run.id,
                RunState::AwaitingApproval,
                run.state(),
                stored_version,
                run.version(),
                |state, next| state.allows(next),
            )?;
            let sequence: u64 = transaction.query_row(
                "SELECT COALESCE(MAX(sequence), 0) + 1 FROM semantic_events WHERE run_id = ?1",
                [run_id.to_string()],
                |row| row.get(0),
            )?;
            let event = OrderedRunEvent {
                run_id,
                sequence,
                event: SemanticEventKind::Approval {
                    digest: approval.digest().to_owned(),
                    approved: false,
                },
            };
            update_run(&transaction, &run, stored_version)?;
            transaction.execute(
                "INSERT INTO semantic_events (run_id, sequence, body) VALUES (?1, ?2, ?3)",
                params![run_id.to_string(), sequence, encode(&event)?],
            )?;
            transaction.commit()?;
            Ok(run)
        })
    }

    pub fn authorize_and_resume(
        &self,
        run_id: Uuid,
        approval_id: Uuid,
        presented: &ApprovalScope,
        now_unix_ms: i64,
    ) -> Result<AuthorizationResult, PersistenceError> {
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let (mut run, stored_version) = load_run(&transaction, run_id)?;
            let mut approval = load_approval(&transaction, run_id, approval_id)?;
            validate_pending_approval(&run, &approval)?;
            if run.changeset_id() != presented.changeset_id {
                return Err(PersistenceError::ApprovalDoesNotMatchRun);
            }
            let approved_action = approval.consume_for_run(run_id, presented, now_unix_ms)?;
            run.resume_after_approval()?;
            validate_versioned_transition(
                "run",
                run.id,
                RunState::AwaitingApproval,
                run.state(),
                stored_version,
                run.version(),
                |state, next| state.allows(next),
            )?;
            let sequence: u64 = transaction.query_row(
                "SELECT COALESCE(MAX(sequence), 0) + 1 FROM semantic_events WHERE run_id = ?1",
                [run_id.to_string()],
                |row| row.get(0),
            )?;
            let event = OrderedRunEvent {
                run_id,
                sequence,
                event: SemanticEventKind::Approval {
                    digest: approval.digest().to_owned(),
                    approved: true,
                },
            };
            transaction.execute(
                "UPDATE approvals SET body = ?2 WHERE id = ?1",
                params![approval_id.to_string(), encode(&approval)?],
            )?;
            update_run(&transaction, &run, stored_version)?;
            transaction.execute(
                "INSERT INTO semantic_events (run_id, sequence, body) VALUES (?1, ?2, ?3)",
                params![run_id.to_string(), sequence, encode(&event)?],
            )?;
            transaction.commit()?;
            Ok(AuthorizationResult {
                run,
                approved_action,
                event,
            })
        })
    }

    pub fn save_checkpoint(&self, checkpoint: &Checkpoint) -> Result<(), PersistenceError> {
        self.with_connection(|connection| {
            connection.execute(
                "INSERT INTO checkpoints (id, owner, body) VALUES (?1, ?2, ?3)",
                params![
                    checkpoint.id.to_string(),
                    checkpoint.run_id.to_string(),
                    encode(checkpoint)?
                ],
            )?;
            Ok(())
        })
    }

    pub fn checkpoint_for_run(&self, run_id: Uuid) -> Result<Option<Checkpoint>, PersistenceError> {
        self.with_connection(|connection| {
            let body = connection
                .query_row(
                    "SELECT body FROM checkpoints WHERE owner = ?1 ORDER BY rowid DESC LIMIT 1",
                    [run_id.to_string()],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            body.map(|value| decode(&value)).transpose()
        })
    }

    pub fn checkpoint_for_changeset(
        &self,
        changeset_id: Uuid,
    ) -> Result<Option<Checkpoint>, PersistenceError> {
        self.with_connection(|connection| {
            let body = connection.query_row("SELECT checkpoints.body FROM checkpoints JOIN runs ON runs.id = checkpoints.owner WHERE runs.changeset_id = ?1 ORDER BY checkpoints.rowid DESC LIMIT 1", [changeset_id.to_string()], |row| row.get::<_, String>(0)).optional()?;
            body.map(|value| decode(&value)).transpose()
        })
    }

    pub fn prepare_changeset_finalization(
        &self,
        confirmation_digest: &str,
        scope: &ChangesetMutationScope,
        worktree_id: Uuid,
        expected_worktree_version: u64,
        now_unix_ms: i64,
    ) -> Result<ChangesetFinalization, PersistenceError> {
        if !is_sha256_digest(confirmation_digest) || confirmation_digest != scope.digest() {
            return Err(PersistenceError::FinalizationDigestMismatch);
        }
        validate_finalization_scope(scope)?;
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            if let Some(stored) =
                load_changeset_finalization_for_changeset(&transaction, scope.changeset_id)?
            {
                return if finalization_matches_request(
                    &stored,
                    confirmation_digest,
                    scope,
                    worktree_id,
                    expected_worktree_version,
                ) {
                    Ok(stored)
                } else {
                    Err(PersistenceError::FinalizationConflict)
                };
            }

            let (changeset, stored_changeset_version) =
                load_changeset(&transaction, scope.changeset_id)?;
            let (worktree, stored_worktree_version) = load_worktree(&transaction, worktree_id)?;
            let checkpoint = load_checkpoint_for_changeset(
                &transaction,
                scope.checkpoint_id,
                scope.changeset_id,
            )?;
            if stored_changeset_version != scope.expected_version {
                return Err(PersistenceError::VersionConflict {
                    aggregate: "changeset",
                    id: changeset.id,
                    expected: scope.expected_version,
                    actual: stored_changeset_version,
                });
            }
            if stored_worktree_version != expected_worktree_version {
                return Err(PersistenceError::VersionConflict {
                    aggregate: "worktree",
                    id: worktree.id,
                    expected: expected_worktree_version,
                    actual: stored_worktree_version,
                });
            }
            if changeset.repository_id() != scope.repository_id
                || changeset.state() != ChangesetState::Reviewable
                || changeset.base_sha() != scope.base_sha
                || changeset.head_sha() != scope.expected_head_sha
                || worktree.changeset_id() != scope.changeset_id
                || worktree.state() != WorktreeState::Ready
                || worktree.base_sha != scope.base_sha
                || checkpoint.base_sha != scope.base_sha
                || checkpoint.head_sha != scope.expected_head_sha
            {
                return Err(PersistenceError::InvalidPersistedTransition(
                    "changeset finalization preparation",
                ));
            }

            let finalization = prepared_finalization(
                confirmation_digest,
                scope,
                worktree_id,
                expected_worktree_version,
                now_unix_ms,
            );
            insert_changeset_finalization(&transaction, &finalization)?;
            transaction.commit()?;
            Ok(finalization)
        })
    }

    pub fn changeset_finalization(
        &self,
        confirmation_digest: &str,
    ) -> Result<Option<ChangesetFinalization>, PersistenceError> {
        self.with_connection(|connection| {
            load_changeset_finalization(connection, confirmation_digest)
        })
    }

    pub fn completed_changeset_finalization(
        &self,
        confirmation_digest: &str,
    ) -> Result<CompletedChangesetFinalization, PersistenceError> {
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Deferred)?;
            let finalization = load_changeset_finalization(&transaction, confirmation_digest)?
                .ok_or(PersistenceError::NotFound("changeset finalization"))?;
            if finalization.state != ChangesetFinalizationState::Completed {
                return Err(PersistenceError::FinalizationConflict);
            }
            let (changeset, changeset_version) =
                load_changeset(&transaction, finalization.scope.changeset_id)?;
            let (worktree, worktree_version) =
                load_worktree(&transaction, finalization.worktree_id)?;
            validate_completed_finalization_replay(
                &finalization,
                &changeset,
                changeset_version,
                &worktree,
                worktree_version,
            )?;
            transaction.commit()?;
            Ok(CompletedChangesetFinalization {
                finalization,
                changeset,
                worktree,
            })
        })
    }

    pub fn prepared_changeset_finalizations(
        &self,
    ) -> Result<Vec<ChangesetFinalization>, PersistenceError> {
        self.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT confirmation_digest, changeset_id, repository_id, checkpoint_id, worktree_id, kind, status, version, created_at_unix_ms, updated_at_unix_ms, body
                 FROM changeset_finalizations
                 WHERE status = ?1 OR json_extract(body, '$.state') = ?2
                 ORDER BY created_at_unix_ms, rowid
                 LIMIT ?3",
            )?;
            let rows = statement.query_map(
                params![
                    state_json(&ChangesetFinalizationState::Prepared)?,
                    "prepared",
                    domain::limits::MAX_RECOVERY_ACTIONS as i64,
                ],
                changeset_finalization_row,
            )?;
            rows.map(|row| validate_changeset_finalization(row?)).collect()
        })
    }

    pub fn complete_changeset_finalization(
        &self,
        confirmation_digest: &str,
        result: ChangesetFinalizationResult,
        now_unix_ms: i64,
    ) -> Result<CompletedChangesetFinalization, PersistenceError> {
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let mut finalization = load_changeset_finalization(&transaction, confirmation_digest)?
                .ok_or(PersistenceError::NotFound("changeset finalization"))?;
            if finalization.state == ChangesetFinalizationState::Completed {
                if finalization.result.as_ref() != Some(&result) {
                    return Err(PersistenceError::FinalizationConflict);
                }
                let (changeset, changeset_version) =
                    load_changeset(&transaction, finalization.scope.changeset_id)?;
                let (worktree, worktree_version) =
                    load_worktree(&transaction, finalization.worktree_id)?;
                validate_completed_finalization_replay(
                    &finalization,
                    &changeset,
                    changeset_version,
                    &worktree,
                    worktree_version,
                )?;
                return Ok(CompletedChangesetFinalization {
                    finalization,
                    changeset,
                    worktree,
                });
            }
            if finalization.state != ChangesetFinalizationState::Prepared {
                return Err(PersistenceError::FinalizationConflict);
            }
            validate_finalization_result(&finalization.scope, &result)?;

            let (mut changeset, stored_changeset_version) =
                load_changeset(&transaction, finalization.scope.changeset_id)?;
            let (mut worktree, stored_worktree_version) =
                load_worktree(&transaction, finalization.worktree_id)?;
            validate_prepared_finalization_aggregates(
                &finalization,
                &changeset,
                stored_changeset_version,
                &worktree,
                stored_worktree_version,
            )?;
            if now_unix_ms < finalization.created_at_unix_ms {
                return Err(PersistenceError::InvalidFinalizationInput("timestamp"));
            }

            match &result {
                ChangesetFinalizationResult::Commit {
                    resulting_head_sha, ..
                } => changeset.commit(resulting_head_sha.clone())?,
                ChangesetFinalizationResult::Discard => changeset.discard()?,
            }
            worktree.begin_removal()?;
            update_changeset(&transaction, &changeset, stored_changeset_version)?;
            update_worktree(&transaction, &worktree, stored_worktree_version)?;
            append_aggregate_event(
                &transaction,
                "changeset",
                changeset.id,
                &format!("{:?}", changeset.state()).to_lowercase(),
            )?;
            append_aggregate_event(&transaction, "worktree", worktree.id, "removing")?;
            let removing_version = worktree.version();
            worktree.removed()?;
            update_worktree(&transaction, &worktree, removing_version)?;
            append_aggregate_event(&transaction, "worktree", worktree.id, "removed")?;

            finalization.state = ChangesetFinalizationState::Completed;
            finalization.result = Some(result);
            finalization.version = finalization.version.saturating_add(1);
            finalization.updated_at_unix_ms = now_unix_ms;
            update_changeset_finalization(&transaction, &finalization, 0)?;
            transaction.commit()?;
            Ok(CompletedChangesetFinalization {
                finalization,
                changeset,
                worktree,
            })
        })
    }

    pub fn mark_changeset_finalization_divergent(
        &self,
        confirmation_digest: &str,
        observed_head_sha: String,
        detail: String,
        now_unix_ms: i64,
    ) -> Result<CompletedChangesetFinalization, PersistenceError> {
        if detail.is_empty() || detail.len() > domain::limits::MAX_SEMANTIC_TEXT_BYTES {
            return Err(PersistenceError::ResourceLimit(
                "finalization divergence detail",
            ));
        }
        if !is_git_object_id(&observed_head_sha) {
            return Err(PersistenceError::InvalidFinalizationInput(
                "observed head sha",
            ));
        }
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let mut finalization = load_changeset_finalization(&transaction, confirmation_digest)?
                .ok_or(PersistenceError::NotFound("changeset finalization"))?;
            if finalization.state == ChangesetFinalizationState::Divergent {
                let (changeset, changeset_version) =
                    load_changeset(&transaction, finalization.scope.changeset_id)?;
                let (worktree, worktree_version) =
                    load_worktree(&transaction, finalization.worktree_id)?;
                if finalization.divergence_detail.as_deref() != Some(&detail)
                    || changeset.head_sha() != observed_head_sha
                {
                    return Err(PersistenceError::FinalizationConflict);
                }
                validate_divergent_finalization_replay(
                    &finalization,
                    &changeset,
                    changeset_version,
                    &worktree,
                    worktree_version,
                )?;
                return Ok(CompletedChangesetFinalization {
                    finalization,
                    changeset,
                    worktree,
                });
            }
            if finalization.state != ChangesetFinalizationState::Prepared {
                return Err(PersistenceError::FinalizationConflict);
            }

            let (mut changeset, stored_changeset_version) =
                load_changeset(&transaction, finalization.scope.changeset_id)?;
            let (mut worktree, stored_worktree_version) =
                load_worktree(&transaction, finalization.worktree_id)?;
            validate_prepared_finalization_aggregates(
                &finalization,
                &changeset,
                stored_changeset_version,
                &worktree,
                stored_worktree_version,
            )?;
            if now_unix_ms < finalization.created_at_unix_ms {
                return Err(PersistenceError::InvalidFinalizationInput("timestamp"));
            }
            changeset.mark_divergent(observed_head_sha)?;
            worktree.quarantine()?;
            update_changeset(&transaction, &changeset, stored_changeset_version)?;
            update_worktree(&transaction, &worktree, stored_worktree_version)?;
            append_aggregate_event(&transaction, "changeset", changeset.id, "divergent")?;
            append_aggregate_event(&transaction, "worktree", worktree.id, "quarantined")?;

            finalization.state = ChangesetFinalizationState::Divergent;
            finalization.divergence_detail = Some(detail);
            finalization.version = finalization.version.saturating_add(1);
            finalization.updated_at_unix_ms = now_unix_ms;
            update_changeset_finalization(&transaction, &finalization, 0)?;
            transaction.commit()?;
            Ok(CompletedChangesetFinalization {
                finalization,
                changeset,
                worktree,
            })
        })
    }

    pub fn complete_with_checkpoint(
        &self,
        changeset: &Changeset,
        run: &Run,
        checkpoint: &Checkpoint,
    ) -> Result<Vec<OrderedRunEvent>, PersistenceError> {
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let (stored_changeset, stored_changeset_version) =
                load_changeset(&transaction, changeset.id)?;
            let (stored_run, stored_run_version) = load_run(&transaction, run.id)?;
            validate_state_transition(
                "changeset",
                changeset.id,
                stored_changeset.state(),
                changeset.state(),
                stored_changeset_version,
                changeset.version(),
                |state, next| state.allows(next),
            )?;
            let expected_run_version = run.version().saturating_sub(2);
            if stored_run_version != expected_run_version {
                return Err(PersistenceError::VersionConflict {
                    aggregate: "run",
                    id: run.id,
                    expected: expected_run_version,
                    actual: stored_run_version,
                });
            }
            let mut reviewable_run = stored_run.clone();
            reviewable_run.mark_reviewable()?;
            let mut completed_run = reviewable_run.clone();
            completed_run.complete()?;
            if stored_changeset.state() != ChangesetState::Active
                || changeset.state() != ChangesetState::Reviewable
                || stored_run.state() != RunState::Running
                || run.state() != RunState::Completed
                || completed_run != *run
                || checkpoint.run_id != run.id
                || run.changeset_id() != changeset.id
            {
                return Err(PersistenceError::InvalidPersistedTransition(
                    "run completion",
                ));
            }
            let first_sequence: u64 = transaction.query_row(
                "SELECT COALESCE(MAX(sequence), 0) + 1 FROM semantic_events WHERE run_id = ?1",
                [run.id.to_string()],
                |row| row.get(0),
            )?;
            let events = vec![
                OrderedRunEvent {
                    run_id: run.id,
                    sequence: first_sequence,
                    event: SemanticEventKind::Lifecycle {
                        state: RunState::Reviewable,
                    },
                },
                OrderedRunEvent {
                    run_id: run.id,
                    sequence: first_sequence + 1,
                    event: SemanticEventKind::Lifecycle {
                        state: RunState::Completed,
                    },
                },
            ];
            update_changeset(&transaction, changeset, stored_changeset_version)?;
            update_run(&transaction, &reviewable_run, stored_run_version)?;
            update_run(&transaction, &completed_run, reviewable_run.version())?;
            transaction.execute(
                "INSERT INTO checkpoints (id, owner, body) VALUES (?1, ?2, ?3)",
                params![
                    checkpoint.id.to_string(),
                    run.id.to_string(),
                    encode(checkpoint)?
                ],
            )?;
            for event in &events {
                transaction.execute(
                    "INSERT INTO semantic_events (run_id, sequence, body) VALUES (?1, ?2, ?3)",
                    params![run.id.to_string(), event.sequence, encode(event)?],
                )?;
            }
            append_aggregate_event(&transaction, "changeset", changeset.id, "reviewable")?;
            transaction.commit()?;
            Ok(events)
        })
    }

    pub fn save_finding(&self, finding: &Finding) -> Result<(), PersistenceError> {
        self.save_findings(std::slice::from_ref(finding)).map(drop)
    }

    pub fn save_findings(&self, findings: &[Finding]) -> Result<Vec<Finding>, PersistenceError> {
        if findings.is_empty() {
            return Ok(Vec::new());
        }
        let changeset_id = validate_finding_owners(findings)?;
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let canonical = save_findings(&transaction, changeset_id, findings)?;
            transaction.commit()?;
            Ok(canonical)
        })
    }

    pub fn save_findings_for_revision(
        &self,
        changeset: &Changeset,
        findings: &[Finding],
    ) -> Result<Vec<Finding>, PersistenceError> {
        if !findings.is_empty() && validate_finding_owners(findings)? != changeset.id {
            return Err(PersistenceError::MixedFindingOwners);
        }
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let (stored, stored_version) = load_changeset(&transaction, changeset.id)?;
            if stored_version != changeset.version() {
                return Err(PersistenceError::VersionConflict {
                    aggregate: "changeset",
                    id: changeset.id,
                    expected: changeset.version(),
                    actual: stored_version,
                });
            }
            if stored.state() != ChangesetState::Reviewable
                || stored.head_sha() != changeset.head_sha()
            {
                return Err(PersistenceError::InvalidPersistedTransition(
                    "review finding persistence",
                ));
            }
            let canonical = save_findings(&transaction, changeset.id, findings)?;
            transaction.commit()?;
            Ok(canonical)
        })
    }

    pub fn findings_for_changeset(
        &self,
        changeset_id: Uuid,
    ) -> Result<Vec<Finding>, PersistenceError> {
        self.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT body, version FROM findings WHERE owner = ?1 ORDER BY rowid LIMIT ?2",
            )?;
            let rows = statement.query_map(
                params![
                    changeset_id.to_string(),
                    domain::limits::MAX_FINDINGS_PER_CHANGESET
                ],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?)),
            )?;
            let mut findings = Vec::new();
            for row in rows {
                let (body, scalar_version) = row?;
                let finding: Finding = decode(&body)?;
                ensure_version_agreement("finding", finding.id, scalar_version, finding.version())?;
                findings.push(finding);
            }
            Ok(findings)
        })
    }

    pub fn persist_finding_transition(&self, finding: &Finding) -> Result<(), PersistenceError> {
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let (stored, stored_version) = load_finding(&transaction, finding.id)?;
            validate_state_transition(
                "finding",
                finding.id,
                stored.state(),
                finding.state(),
                stored_version,
                finding.version(),
                |state, next| state.allows(next),
            )?;
            update_finding(&transaction, finding, stored_version)?;
            append_aggregate_event(
                &transaction,
                "finding",
                finding.id,
                &format!("{:?}", finding.state()).to_lowercase(),
            )?;
            transaction.commit()?;
            Ok(())
        })
    }

    pub fn acquire_mutation_lease(
        &self,
        changeset_id: Uuid,
        run_id: Uuid,
        now_unix_ms: i64,
        ttl: Duration,
    ) -> Result<MutationLease, PersistenceError> {
        let ttl_millis = i64::try_from(ttl.as_millis())
            .ok()
            .filter(|millis| *millis > 0)
            .ok_or(PersistenceError::InvalidLeaseTtl)?;
        let expires_at_unix_ms = now_unix_ms
            .checked_add(ttl_millis)
            .ok_or(PersistenceError::InvalidLeaseTtl)?;

        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let existing = load_mutation_lease_from(&transaction, changeset_id)?;
            let lease = match existing {
                Some(existing)
                    if existing.expires_at_unix_ms > now_unix_ms
                        && existing.run_id != run_id =>
                {
                    return Err(PersistenceError::MutationLeaseHeld {
                        changeset_id,
                        run_id: existing.run_id,
                        expires_at_unix_ms: existing.expires_at_unix_ms,
                    });
                }
                Some(existing) if existing.expires_at_unix_ms > now_unix_ms => MutationLease {
                    expires_at_unix_ms,
                    ..existing
                },
                Some(existing) => MutationLease {
                    changeset_id,
                    run_id,
                    fencing_epoch: existing
                        .fencing_epoch
                        .checked_add(1)
                        .ok_or(PersistenceError::FencingEpochExhausted)?,
                    expires_at_unix_ms,
                },
                None => MutationLease {
                    changeset_id,
                    run_id,
                    fencing_epoch: 1,
                    expires_at_unix_ms,
                },
            };
            transaction.execute(
                "INSERT INTO mutation_leases (changeset_id, run_id, fencing_epoch, expires_at_unix_ms, body)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(changeset_id) DO UPDATE SET
                     run_id = excluded.run_id,
                     fencing_epoch = excluded.fencing_epoch,
                     expires_at_unix_ms = excluded.expires_at_unix_ms,
                     body = excluded.body",
                params![
                    lease.changeset_id.to_string(),
                    lease.run_id.to_string(),
                    lease.fencing_epoch,
                    lease.expires_at_unix_ms,
                    encode(&lease)?
                ],
            )?;
            transaction.commit()?;
            Ok(lease)
        })
    }

    pub fn mutation_lease(
        &self,
        changeset_id: Uuid,
    ) -> Result<Option<MutationLease>, PersistenceError> {
        self.with_connection(|connection| load_mutation_lease_from(connection, changeset_id))
    }

    pub fn mutation_leases(&self) -> Result<Vec<MutationLease>, PersistenceError> {
        self.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT changeset_id, run_id, fencing_epoch, expires_at_unix_ms, body
                 FROM mutation_leases
                 ORDER BY changeset_id",
            )?;
            let rows = statement.query_map([], mutation_lease_row)?;
            rows.map(|row| validate_mutation_lease(row?)).collect()
        })
    }

    pub fn verify_mutation_lease(
        &self,
        changeset_id: Uuid,
        run_id: Uuid,
        fencing_epoch: u64,
        now_unix_ms: i64,
    ) -> Result<MutationLease, PersistenceError> {
        let lease = self
            .mutation_lease(changeset_id)?
            .ok_or(PersistenceError::MutationLeaseMismatch)?;
        if lease.run_id != run_id || lease.fencing_epoch != fencing_epoch {
            return Err(PersistenceError::MutationLeaseMismatch);
        }
        if lease.expires_at_unix_ms <= now_unix_ms {
            return Err(PersistenceError::MutationLeaseExpired);
        }
        Ok(lease)
    }

    pub fn release_mutation_lease(
        &self,
        changeset_id: Uuid,
        run_id: Uuid,
        fencing_epoch: u64,
    ) -> Result<bool, PersistenceError> {
        self.with_connection(|connection| {
            let changed = connection.execute(
                "DELETE FROM mutation_leases
                 WHERE changeset_id = ?1 AND run_id = ?2 AND fencing_epoch = ?3",
                params![changeset_id.to_string(), run_id.to_string(), fencing_epoch],
            )?;
            Ok(changed == 1)
        })
    }

    pub fn insert_prepared_process_supervision(
        &self,
        run_id: Uuid,
        metadata: &SupervisionMetadata,
        updated_at_unix_ms: i64,
    ) -> Result<ProcessSupervisionRecord, PersistenceError> {
        if metadata.state != SupervisionState::Prepared || metadata.termination_reason.is_some() {
            return Err(PersistenceError::InvalidProcessSupervisionTransition);
        }
        let record = ProcessSupervisionRecord {
            run_id,
            metadata: metadata.clone(),
            updated_at_unix_ms,
        };
        self.with_connection(|connection| {
            connection.execute(
                "INSERT INTO process_supervision_records
                 (id, run_id, status, updated_at_unix_ms, body)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    metadata.supervision_id.to_string(),
                    run_id.to_string(),
                    supervision_state_json(metadata.state)?,
                    updated_at_unix_ms,
                    encode(&record)?
                ],
            )?;
            Ok(record)
        })
    }

    pub fn mark_process_supervision_running(
        &self,
        run_id: Uuid,
        metadata: &SupervisionMetadata,
        updated_at_unix_ms: i64,
    ) -> Result<ProcessSupervisionRecord, PersistenceError> {
        self.transition_process_supervision(
            run_id,
            metadata,
            SupervisionState::Prepared,
            SupervisionState::Running,
            updated_at_unix_ms,
        )
    }

    pub fn mark_process_supervision_terminated(
        &self,
        run_id: Uuid,
        metadata: &SupervisionMetadata,
        updated_at_unix_ms: i64,
    ) -> Result<ProcessSupervisionRecord, PersistenceError> {
        if metadata.state != SupervisionState::Terminated || metadata.termination_reason.is_none() {
            return Err(PersistenceError::InvalidProcessSupervisionTransition);
        }
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let stored =
                load_process_supervision_from(&transaction, run_id, metadata.supervision_id)?
                    .ok_or(PersistenceError::NotFound("process supervision record"))?;
            if !matches!(
                stored.metadata.state,
                SupervisionState::Prepared | SupervisionState::Running
            ) || stored.updated_at_unix_ms > updated_at_unix_ms
                || !same_process_supervision_identity(&stored.metadata, metadata)
            {
                return Err(PersistenceError::InvalidProcessSupervisionTransition);
            }
            let record = ProcessSupervisionRecord {
                run_id,
                metadata: metadata.clone(),
                updated_at_unix_ms,
            };
            let changed = transaction.execute(
                "UPDATE process_supervision_records
                 SET status = ?3, updated_at_unix_ms = ?4, body = ?5
                 WHERE id = ?1 AND run_id = ?2 AND status = ?6",
                params![
                    metadata.supervision_id.to_string(),
                    run_id.to_string(),
                    supervision_state_json(SupervisionState::Terminated)?,
                    updated_at_unix_ms,
                    encode(&record)?,
                    supervision_state_json(stored.metadata.state)?
                ],
            )?;
            if changed != 1 {
                return Err(PersistenceError::InvalidProcessSupervisionTransition);
            }
            transaction.commit()?;
            Ok(record)
        })
    }

    pub fn latest_process_supervision_for_run(
        &self,
        run_id: Uuid,
    ) -> Result<Option<ProcessSupervisionRecord>, PersistenceError> {
        self.with_connection(|connection| {
            let row = connection
                .query_row(
                    "SELECT id, run_id, status, updated_at_unix_ms, body
                     FROM process_supervision_records
                     WHERE run_id = ?1
                     ORDER BY updated_at_unix_ms DESC, rowid DESC
                     LIMIT 1",
                    [run_id.to_string()],
                    process_supervision_row,
                )
                .optional()?;
            row.map(validate_process_supervision_record).transpose()
        })
    }

    pub fn process_supervision_for_run(
        &self,
        run_id: Uuid,
        supervision_id: Uuid,
    ) -> Result<Option<ProcessSupervisionRecord>, PersistenceError> {
        self.with_connection(|connection| {
            let row = connection
                .query_row(
                    "SELECT id, run_id, status, updated_at_unix_ms, body
                     FROM process_supervision_records
                     WHERE id = ?1 AND run_id = ?2",
                    params![supervision_id.to_string(), run_id.to_string()],
                    process_supervision_row,
                )
                .optional()?;
            row.map(validate_process_supervision_record).transpose()
        })
    }

    pub fn nonterminated_process_supervisions(
        &self,
    ) -> Result<Vec<ProcessSupervisionRecord>, PersistenceError> {
        self.with_connection(|connection| {
            let terminated = supervision_state_json(SupervisionState::Terminated)?;
            let mut statement = connection.prepare(
                "SELECT id, run_id, status, updated_at_unix_ms, body
                 FROM process_supervision_records
                 WHERE status != ?1
                 ORDER BY updated_at_unix_ms, rowid",
            )?;
            let rows = statement.query_map([terminated], process_supervision_row)?;
            rows.map(|row| validate_process_supervision_record(row?))
                .collect()
        })
    }

    fn transition_process_supervision(
        &self,
        run_id: Uuid,
        metadata: &SupervisionMetadata,
        expected_state: SupervisionState,
        next_state: SupervisionState,
        updated_at_unix_ms: i64,
    ) -> Result<ProcessSupervisionRecord, PersistenceError> {
        if metadata.state != next_state
            || (next_state == SupervisionState::Terminated && metadata.termination_reason.is_none())
            || (next_state != SupervisionState::Terminated && metadata.termination_reason.is_some())
        {
            return Err(PersistenceError::InvalidProcessSupervisionTransition);
        }
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let stored =
                load_process_supervision_from(&transaction, run_id, metadata.supervision_id)?
                    .ok_or(PersistenceError::NotFound("process supervision record"))?;
            if stored.metadata.state != expected_state
                || stored.updated_at_unix_ms > updated_at_unix_ms
                || !same_process_supervision_identity(&stored.metadata, metadata)
            {
                return Err(PersistenceError::InvalidProcessSupervisionTransition);
            }
            let record = ProcessSupervisionRecord {
                run_id,
                metadata: metadata.clone(),
                updated_at_unix_ms,
            };
            let changed = transaction.execute(
                "UPDATE process_supervision_records
                 SET status = ?3, updated_at_unix_ms = ?4, body = ?5
                 WHERE id = ?1 AND run_id = ?2 AND status = ?6",
                params![
                    metadata.supervision_id.to_string(),
                    run_id.to_string(),
                    supervision_state_json(next_state)?,
                    updated_at_unix_ms,
                    encode(&record)?,
                    supervision_state_json(expected_state)?
                ],
            )?;
            if changed != 1 {
                return Err(PersistenceError::InvalidProcessSupervisionTransition);
            }
            transaction.commit()?;
            Ok(record)
        })
    }

    pub fn record_recovery_action(
        &self,
        idempotency_key: &str,
        run_id: Option<Uuid>,
        changeset_id: Option<Uuid>,
        action: RecoveryAction,
        created_at_unix_ms: i64,
    ) -> Result<RecoveryActionRecord, PersistenceError> {
        if idempotency_key.is_empty() {
            return Err(PersistenceError::InvalidRecoveryAction);
        }
        let record = RecoveryActionRecord {
            idempotency_key: idempotency_key.to_owned(),
            run_id,
            changeset_id,
            status: RecoveryActionStatus::Completed,
            created_at_unix_ms,
            action,
        };
        self.with_connection(|connection| {
            connection.execute(
                "INSERT OR IGNORE INTO recovery_actions
                 (id, run_id, changeset_id, status, created_at_unix_ms, body)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    record.idempotency_key,
                    record.run_id.map(|id| id.to_string()),
                    record.changeset_id.map(|id| id.to_string()),
                    state_json(&record.status)?,
                    record.created_at_unix_ms,
                    encode(&record)?
                ],
            )?;
            let stored = connection.query_row(
                "SELECT id, run_id, changeset_id, status, created_at_unix_ms, body
                 FROM recovery_actions WHERE id = ?1",
                [idempotency_key],
                recovery_action_row,
            )?;
            let stored = validate_recovery_action_record(stored)?;
            if stored.run_id != run_id
                || stored.changeset_id != changeset_id
                || stored.action != record.action
            {
                return Err(PersistenceError::RecoveryActionConflict);
            }
            Ok(stored)
        })
    }

    pub fn recovery_actions(&self) -> Result<Vec<RecoveryActionRecord>, PersistenceError> {
        self.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, run_id, changeset_id, status, created_at_unix_ms, body
                 FROM recovery_actions
                 ORDER BY created_at_unix_ms DESC, rowid DESC
                 LIMIT ?1",
            )?;
            let rows = statement.query_map(
                [domain::limits::MAX_RECOVERY_ACTIONS as i64],
                recovery_action_row,
            )?;
            let mut records = rows
                .map(|row| validate_recovery_action_record(row?))
                .collect::<Result<Vec<_>, _>>()?;
            records.reverse();
            Ok(records)
        })
    }

    pub fn recover(&self) -> Result<RecoveryReport, PersistenceError> {
        self.with_connection(|connection| {
            let mut statement = connection.prepare("SELECT body, version FROM runs")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
            })?;
            let mut runs = Vec::new();
            for row in rows {
                let (body, scalar_version) = row?;
                let run: Run = decode(&body)?;
                ensure_version_agreement("run", run.id, scalar_version, run.version())?;
                runs.push((run, scalar_version));
            }
            drop(statement);
            let mut report = RecoveryReport::default();
            for (mut run, stored_version) in runs {
                let original = run.state();
                match original {
                    RunState::Starting => {
                        run.fail().map_err(PersistenceError::Domain)?;
                        report.failed.push(run.id);
                    }
                    RunState::Running | RunState::AwaitingApproval => {
                        run.interrupt().map_err(PersistenceError::Domain)?;
                        report.interrupted.push(run.id);
                    }
                    RunState::Queued | RunState::Reviewable => {
                        report.recoverable.push(run.id);
                        continue;
                    }
                    RunState::Completed | RunState::Interrupted | RunState::Failed => {
                        report.terminal.push(run.id);
                        continue;
                    }
                }
                validate_versioned_transition(
                    "run",
                    run.id,
                    original,
                    run.state(),
                    stored_version,
                    run.version(),
                    |state, next| state.allows(next),
                )?;
                let transaction =
                    connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
                let sequence: u64 = transaction.query_row(
                    "SELECT COALESCE(MAX(sequence), 0) + 1 FROM semantic_events WHERE run_id = ?1",
                    [run.id.to_string()],
                    |row| row.get(0),
                )?;
                update_run(&transaction, &run, stored_version)?;
                let event = OrderedRunEvent {
                    run_id: run.id,
                    sequence,
                    event: SemanticEventKind::Lifecycle { state: run.state() },
                };
                transaction.execute(
                    "INSERT INTO semantic_events (run_id, sequence, body) VALUES (?1, ?2, ?3)",
                    params![run.id.to_string(), sequence, encode(&event)?],
                )?;
                transaction.commit()?;
            }
            Ok(report)
        })
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RecoveryReport {
    pub recoverable: Vec<Uuid>,
    pub interrupted: Vec<Uuid>,
    pub failed: Vec<Uuid>,
    pub terminal: Vec<Uuid>,
    pub actions: Vec<RecoveryAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryActionStatus {
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryActionRecord {
    pub idempotency_key: String,
    pub run_id: Option<Uuid>,
    pub changeset_id: Option<Uuid>,
    pub status: RecoveryActionStatus,
    pub created_at_unix_ms: i64,
    pub action: RecoveryAction,
}

#[derive(Debug)]
pub struct AuthorizationResult {
    pub run: Run,
    pub approved_action: ApprovedAction,
    pub event: OrderedRunEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredProposedChange {
    pub proposal: ActionProposal,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationLease {
    pub changeset_id: Uuid,
    pub run_id: Uuid,
    pub fencing_epoch: u64,
    pub expires_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessSupervisionRecord {
    pub run_id: Uuid,
    pub metadata: SupervisionMetadata,
    pub updated_at_unix_ms: i64,
}

fn migrate(connection: &mut Connection) -> Result<(), PersistenceError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS execution_schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at_unix_ms INTEGER NOT NULL
         ) STRICT;",
    )?;
    let recorded_version: u32 = connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM execution_schema_migrations",
        [],
        |row| row.get(0),
    )?;
    let has_execution_schema: bool = connection.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM sqlite_master
            WHERE type = 'table' AND name = 'repositories'
         )",
        [],
        |row| row.get(0),
    )?;
    let has_product_schema: bool = connection.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM sqlite_master
            WHERE type = 'table' AND name = 'product_schema_migrations'
         )",
        [],
        |row| row.get(0),
    )?;
    let legacy_version: u32 =
        connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if recorded_version == 0
        && !has_execution_schema
        && !has_product_schema
        && legacy_version > CURRENT_SCHEMA_VERSION
    {
        return Err(PersistenceError::UnsupportedSchemaVersion {
            database: legacy_version,
            runtime: CURRENT_SCHEMA_VERSION,
        });
    }
    let version = if recorded_version > 0 {
        recorded_version
    } else if has_execution_schema {
        legacy_version.min(CURRENT_SCHEMA_VERSION)
    } else {
        0
    };
    if version > CURRENT_SCHEMA_VERSION {
        return Err(PersistenceError::UnsupportedSchemaVersion {
            database: version,
            runtime: CURRENT_SCHEMA_VERSION,
        });
    }

    for target_version in (version + 1)..=CURRENT_SCHEMA_VERSION {
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        match target_version {
            1 => migrate_to_version_1(&transaction)?,
            2 => migrate_to_version_2(&transaction)?,
            3 => migrate_to_version_3(&transaction)?,
            4 => migrate_to_version_4(&transaction)?,
            5 => migrate_to_version_5(&transaction)?,
            _ => unreachable!("all schema migrations are explicitly ordered"),
        }
        transaction.execute(
            "INSERT INTO execution_schema_migrations (version, applied_at_unix_ms)
             VALUES (?1, CAST(unixepoch('subsec') * 1000 AS INTEGER))",
            [target_version],
        )?;
        transaction.pragma_update(None, "user_version", target_version)?;
        transaction.commit()?;
    }
    Ok(())
}

fn migrate_to_version_1(transaction: &Transaction<'_>) -> Result<(), PersistenceError> {
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS repositories (id TEXT PRIMARY KEY, body TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS changesets (id TEXT PRIMARY KEY, repository_id TEXT NOT NULL REFERENCES repositories(id), state TEXT NOT NULL, body TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS worktrees (id TEXT PRIMARY KEY, changeset_id TEXT NOT NULL REFERENCES changesets(id), body TEXT NOT NULL);
         CREATE INDEX IF NOT EXISTS worktrees_by_changeset ON worktrees(changeset_id);
         CREATE TABLE IF NOT EXISTS runs (id TEXT PRIMARY KEY, changeset_id TEXT NOT NULL REFERENCES changesets(id), state TEXT NOT NULL, body TEXT NOT NULL);
         CREATE INDEX IF NOT EXISTS runs_by_changeset ON runs(changeset_id);
         CREATE UNIQUE INDEX IF NOT EXISTS one_active_run_per_changeset ON runs(changeset_id) WHERE state NOT IN ('\"completed\"', '\"interrupted\"', '\"failed\"');
         CREATE TABLE IF NOT EXISTS semantic_events (run_id TEXT NOT NULL REFERENCES runs(id), sequence INTEGER NOT NULL, body TEXT NOT NULL, PRIMARY KEY (run_id, sequence));
         CREATE TABLE IF NOT EXISTS aggregate_events (aggregate_kind TEXT NOT NULL, aggregate_id TEXT NOT NULL, sequence INTEGER NOT NULL, body TEXT NOT NULL, PRIMARY KEY (aggregate_kind, aggregate_id, sequence));
         CREATE TABLE IF NOT EXISTS proposed_changes (owner TEXT PRIMARY KEY REFERENCES runs(id), body TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS approvals (id TEXT PRIMARY KEY, owner TEXT NOT NULL REFERENCES runs(id), body TEXT NOT NULL);
         CREATE INDEX IF NOT EXISTS approvals_by_owner ON approvals(owner);
         CREATE TABLE IF NOT EXISTS checkpoints (id TEXT PRIMARY KEY, owner TEXT NOT NULL REFERENCES runs(id), body TEXT NOT NULL);
         CREATE INDEX IF NOT EXISTS checkpoints_by_owner ON checkpoints(owner);
         CREATE TABLE IF NOT EXISTS findings (id TEXT PRIMARY KEY, owner TEXT NOT NULL REFERENCES changesets(id), body TEXT NOT NULL);",
    )?;
    Ok(())
}

fn migrate_to_version_3(transaction: &Transaction<'_>) -> Result<(), PersistenceError> {
    transaction.execute(
        "CREATE INDEX IF NOT EXISTS findings_by_owner ON findings(owner)",
        [],
    )?;
    Ok(())
}

fn migrate_to_version_4(transaction: &Transaction<'_>) -> Result<(), PersistenceError> {
    add_column_if_missing(
        transaction,
        "artifacts",
        "status",
        "ALTER TABLE artifacts ADD COLUMN status TEXT NOT NULL DEFAULT 'complete'",
    )?;
    add_column_if_missing(
        transaction,
        "artifacts",
        "stored_bytes",
        "ALTER TABLE artifacts ADD COLUMN stored_bytes INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        transaction,
        "artifacts",
        "updated_at_unix_ms",
        "ALTER TABLE artifacts ADD COLUMN updated_at_unix_ms INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        transaction,
        "artifacts",
        "expires_at_unix_ms",
        "ALTER TABLE artifacts ADD COLUMN expires_at_unix_ms INTEGER",
    )?;
    transaction.execute(
        "UPDATE artifacts SET status = 'deleting', updated_at_unix_ms = created_at_unix_ms WHERE expires_at_unix_ms IS NULL",
        [],
    )?;
    transaction.execute_batch(
        "CREATE INDEX IF NOT EXISTS artifacts_by_status ON artifacts(status, updated_at_unix_ms);
         CREATE INDEX IF NOT EXISTS artifacts_by_expiry ON artifacts(expires_at_unix_ms, status);",
    )?;
    Ok(())
}

fn migrate_to_version_5(transaction: &Transaction<'_>) -> Result<(), PersistenceError> {
    add_column_if_missing(
        transaction,
        "worktrees",
        "state",
        "ALTER TABLE worktrees ADD COLUMN state TEXT NOT NULL DEFAULT '\"creating\"'",
    )?;
    add_column_if_missing(
        transaction,
        "worktrees",
        "version",
        "ALTER TABLE worktrees ADD COLUMN version INTEGER NOT NULL DEFAULT 0",
    )?;
    transaction.execute(
        "UPDATE worktrees
         SET state = json_quote(json_extract(body, '$.state')),
             version = COALESCE(CAST(json_extract(body, '$.version') AS INTEGER), 0)",
        [],
    )?;
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS changeset_finalizations (
             confirmation_digest TEXT PRIMARY KEY,
             changeset_id TEXT NOT NULL UNIQUE REFERENCES changesets(id),
             repository_id TEXT NOT NULL REFERENCES repositories(id),
             checkpoint_id TEXT NOT NULL REFERENCES checkpoints(id),
             worktree_id TEXT NOT NULL REFERENCES worktrees(id),
             kind TEXT NOT NULL,
             status TEXT NOT NULL,
             version INTEGER NOT NULL,
             created_at_unix_ms INTEGER NOT NULL,
             updated_at_unix_ms INTEGER NOT NULL,
             body TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS changeset_finalizations_by_status
             ON changeset_finalizations(status, created_at_unix_ms);
         CREATE INDEX IF NOT EXISTS changeset_finalizations_by_worktree
             ON changeset_finalizations(worktree_id);",
    )?;
    Ok(())
}

fn migrate_to_version_2(transaction: &Transaction<'_>) -> Result<(), PersistenceError> {
    add_column_if_missing(
        transaction,
        "changesets",
        "version",
        "ALTER TABLE changesets ADD COLUMN version INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        transaction,
        "runs",
        "version",
        "ALTER TABLE runs ADD COLUMN version INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        transaction,
        "findings",
        "version",
        "ALTER TABLE findings ADD COLUMN version INTEGER NOT NULL DEFAULT 0",
    )?;
    transaction.execute(
        "UPDATE changesets SET version = COALESCE(CAST(json_extract(body, '$.version') AS INTEGER), 0)",
        [],
    )?;
    transaction.execute(
        "UPDATE runs SET version = COALESCE(CAST(json_extract(body, '$.version') AS INTEGER), 0)",
        [],
    )?;
    transaction.execute(
        "UPDATE findings SET version = COALESCE(CAST(json_extract(body, '$.version') AS INTEGER), 0)",
        [],
    )?;
    transaction.execute_batch(
        "CREATE INDEX IF NOT EXISTS changesets_by_repository ON changesets(repository_id);
         CREATE TABLE IF NOT EXISTS process_supervision_records (
             id TEXT PRIMARY KEY,
             run_id TEXT NOT NULL REFERENCES runs(id),
             status TEXT NOT NULL,
             updated_at_unix_ms INTEGER NOT NULL,
             body TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS process_supervision_by_run ON process_supervision_records(run_id);
         CREATE INDEX IF NOT EXISTS process_supervision_by_status ON process_supervision_records(status, updated_at_unix_ms);
         CREATE TABLE IF NOT EXISTS mutation_leases (
             changeset_id TEXT PRIMARY KEY REFERENCES changesets(id),
             run_id TEXT NOT NULL REFERENCES runs(id),
             fencing_epoch INTEGER NOT NULL,
             expires_at_unix_ms INTEGER NOT NULL,
             body TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS mutation_leases_by_run ON mutation_leases(run_id);
         CREATE INDEX IF NOT EXISTS mutation_leases_by_expiry ON mutation_leases(expires_at_unix_ms);
         CREATE TABLE IF NOT EXISTS recovery_actions (
             id TEXT PRIMARY KEY,
             run_id TEXT REFERENCES runs(id),
             changeset_id TEXT REFERENCES changesets(id),
             status TEXT NOT NULL,
             created_at_unix_ms INTEGER NOT NULL,
             body TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS recovery_actions_by_run ON recovery_actions(run_id);
         CREATE INDEX IF NOT EXISTS recovery_actions_by_changeset ON recovery_actions(changeset_id);
         CREATE INDEX IF NOT EXISTS recovery_actions_by_status ON recovery_actions(status, created_at_unix_ms);
         CREATE TABLE IF NOT EXISTS artifacts (
             id TEXT PRIMARY KEY,
             changeset_id TEXT NOT NULL REFERENCES changesets(id),
             run_id TEXT REFERENCES runs(id),
             kind TEXT NOT NULL,
             created_at_unix_ms INTEGER NOT NULL,
             body BLOB NOT NULL
         );
         CREATE INDEX IF NOT EXISTS artifacts_by_changeset ON artifacts(changeset_id);
         CREATE INDEX IF NOT EXISTS artifacts_by_run ON artifacts(run_id);
         CREATE INDEX IF NOT EXISTS artifacts_by_kind ON artifacts(kind, created_at_unix_ms);",
    )?;
    Ok(())
}

fn add_column_if_missing(
    transaction: &Transaction<'_>,
    table: &str,
    column: &str,
    alter_statement: &str,
) -> Result<(), PersistenceError> {
    let mut statement = transaction.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for existing in columns {
        if existing? == column {
            return Ok(());
        }
    }
    transaction.execute(alter_statement, [])?;
    Ok(())
}

fn query_bounded_events(
    connection: &Connection,
    run_id: Uuid,
    after_sequence: u64,
    limit: usize,
) -> Result<Vec<OrderedRunEvent>, PersistenceError> {
    let mut statement = connection.prepare(
        "SELECT run_id, sequence, body FROM semantic_events WHERE run_id = ?1 AND sequence > ?2 ORDER BY sequence LIMIT ?3",
    )?;
    let rows = statement.query_map(
        params![
            run_id.to_string(),
            after_sequence.min(i64::MAX as u64) as i64,
            limit as i64
        ],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, u64>(1)?,
                row.get::<_, String>(2)?,
            ))
        },
    )?;
    let mut events = Vec::new();
    let mut encoded_bytes = 0_usize;
    for row in rows {
        let (scalar_run_id, scalar_sequence, body) = row?;
        let next_bytes = encoded_bytes
            .checked_add(body.len())
            .ok_or(PersistenceError::ResourceLimit("event page bytes"))?;
        if next_bytes > domain::limits::MAX_EVENT_PAGE_BYTES {
            if events.is_empty() {
                return Err(PersistenceError::ResourceLimit("event page bytes"));
            }
            break;
        }
        let event: OrderedRunEvent = decode(&body)?;
        if scalar_run_id != run_id.to_string()
            || event.run_id != run_id
            || event.sequence != scalar_sequence
        {
            return Err(PersistenceError::CorruptOwnership("semantic event"));
        }
        encoded_bytes = next_bytes;
        events.push(event);
    }
    Ok(events)
}

fn decode_versioned_run(
    run_id: Uuid,
    body: &str,
    scalar_version: u64,
) -> Result<Run, PersistenceError> {
    let run: Run = decode(body)?;
    if run.id != run_id {
        return Err(PersistenceError::CorruptOwnership("run"));
    }
    ensure_version_agreement("run", run_id, scalar_version, run.version())?;
    Ok(run)
}

fn load_repository(
    connection: &Connection,
    repository_id: Uuid,
) -> Result<Repository, PersistenceError> {
    let body = connection.query_row(
        "SELECT body FROM repositories WHERE id = ?1",
        [repository_id.to_string()],
        |row| row.get::<_, String>(0),
    )?;
    let repository: Repository = decode(&body)?;
    if repository.id != repository_id {
        return Err(PersistenceError::CorruptOwnership("repository"));
    }
    Ok(repository)
}

fn load_latest_worktree(
    connection: &Connection,
    changeset_id: Uuid,
) -> Result<Option<Worktree>, PersistenceError> {
    let row = connection
        .query_row(
            "SELECT id, changeset_id, state, version, body
             FROM worktrees
             WHERE changeset_id = ?1
             ORDER BY rowid DESC
             LIMIT 1",
            [changeset_id.to_string()],
            worktree_row,
        )
        .optional()?;
    row.map(validate_worktree).transpose()
}

fn load_optional_worktree(
    connection: &Connection,
    worktree_id: Uuid,
) -> Result<Option<Worktree>, PersistenceError> {
    connection
        .query_row(
            "SELECT id, changeset_id, state, version, body FROM worktrees WHERE id = ?1",
            [worktree_id.to_string()],
            worktree_row,
        )
        .optional()?
        .map(validate_worktree)
        .transpose()
}

fn load_worktree(
    connection: &Connection,
    worktree_id: Uuid,
) -> Result<(Worktree, u64), PersistenceError> {
    let worktree = load_optional_worktree(connection, worktree_id)?
        .ok_or(PersistenceError::NotFound("worktree"))?;
    let version = worktree.version();
    Ok((worktree, version))
}

type WorktreeRow = (String, String, String, u64, String);

fn worktree_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorktreeRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
}

fn validate_worktree(
    (id, changeset_id, state, version, body): WorktreeRow,
) -> Result<Worktree, PersistenceError> {
    let scalar_id =
        Uuid::parse_str(&id).map_err(|_| PersistenceError::CorruptIdentifier("worktree id"))?;
    let scalar_changeset_id = Uuid::parse_str(&changeset_id)
        .map_err(|_| PersistenceError::CorruptIdentifier("worktree changeset id"))?;
    let scalar_state: WorktreeState = decode(&state)?;
    let worktree: Worktree = decode(&body)?;
    if worktree.id != scalar_id
        || worktree.changeset_id() != scalar_changeset_id
        || worktree.state() != scalar_state
    {
        return Err(PersistenceError::CorruptOwnership("worktree"));
    }
    ensure_version_agreement("worktree", worktree.id, version, worktree.version())?;
    Ok(worktree)
}

fn load_latest_checkpoint(
    connection: &Connection,
    run_id: Uuid,
) -> Result<Option<Checkpoint>, PersistenceError> {
    let row = connection
        .query_row(
            "SELECT owner, body FROM checkpoints WHERE owner = ?1 ORDER BY rowid DESC LIMIT 1",
            [run_id.to_string()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    row.map(|(scalar_run_id, body)| {
        let checkpoint: Checkpoint = decode(&body)?;
        if scalar_run_id != run_id.to_string() || checkpoint.run_id != run_id {
            return Err(PersistenceError::CorruptOwnership("checkpoint"));
        }
        Ok(checkpoint)
    })
    .transpose()
}

fn load_latest_approval(
    connection: &Connection,
    run_id: Uuid,
) -> Result<Option<Approval>, PersistenceError> {
    connection
        .query_row(
            "SELECT id, owner, body FROM approvals WHERE owner = ?1 ORDER BY rowid DESC LIMIT 1",
            [run_id.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?
        .map(|row| decode_approval_row(run_id, row))
        .transpose()
}

fn load_approval(
    connection: &Connection,
    run_id: Uuid,
    approval_id: Uuid,
) -> Result<Approval, PersistenceError> {
    let row = connection.query_row(
        "SELECT id, owner, body FROM approvals WHERE id = ?1 AND owner = ?2",
        params![approval_id.to_string(), run_id.to_string()],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        },
    )?;
    let approval = decode_approval_row(run_id, row)?;
    if approval.id != approval_id {
        return Err(PersistenceError::CorruptOwnership("approval"));
    }
    Ok(approval)
}

fn decode_approval_row(
    run_id: Uuid,
    (scalar_approval_id, scalar_run_id, body): (String, String, String),
) -> Result<Approval, PersistenceError> {
    let approval: Approval = decode(&body)?;
    if scalar_approval_id != approval.id.to_string()
        || scalar_run_id != run_id.to_string()
        || approval.run_id() != run_id
    {
        return Err(PersistenceError::CorruptOwnership("approval"));
    }
    Ok(approval)
}

fn validate_pending_approval(run: &Run, approval: &Approval) -> Result<(), PersistenceError> {
    if run.state() != RunState::AwaitingApproval
        || approval.run_id() != run.id
        || approval.is_consumed()
        || approval.scope().changeset_id != run.changeset_id()
        || approval.digest() != approval.scope().digest()
        || run.proposal_digest() != Some(approval.digest())
    {
        return Err(PersistenceError::ApprovalDoesNotMatchRun);
    }
    Ok(())
}

fn load_pending_approval(
    connection: &Connection,
    run: &Run,
) -> Result<Option<Approval>, PersistenceError> {
    if run.state() != RunState::AwaitingApproval {
        return Ok(None);
    }
    let approval = load_latest_approval(connection, run.id)?
        .ok_or(PersistenceError::ApprovalDoesNotMatchRun)?;
    validate_pending_approval(run, &approval)?;
    Ok(Some(approval))
}

fn load_checkpoint_for_changeset(
    connection: &Connection,
    checkpoint_id: Uuid,
    changeset_id: Uuid,
) -> Result<Checkpoint, PersistenceError> {
    let (scalar_checkpoint_id, scalar_run_id, scalar_changeset_id, body): (
        String,
        String,
        String,
        String,
    ) = connection.query_row(
        "SELECT checkpoints.id, checkpoints.owner, runs.changeset_id, checkpoints.body
         FROM checkpoints
         JOIN runs ON runs.id = checkpoints.owner
         WHERE checkpoints.id = ?1",
        [checkpoint_id.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let checkpoint: Checkpoint = decode(&body)?;
    if scalar_checkpoint_id != checkpoint_id.to_string()
        || checkpoint.id != checkpoint_id
        || scalar_run_id != checkpoint.run_id.to_string()
        || scalar_changeset_id != changeset_id.to_string()
    {
        return Err(PersistenceError::CorruptOwnership("checkpoint"));
    }
    Ok(checkpoint)
}

fn load_findings(
    connection: &Connection,
    changeset_id: Uuid,
) -> Result<Vec<Finding>, PersistenceError> {
    let mut statement = connection.prepare(
        "SELECT owner, body, version FROM findings WHERE owner = ?1 ORDER BY rowid LIMIT ?2",
    )?;
    let rows = statement.query_map(
        params![
            changeset_id.to_string(),
            domain::limits::MAX_FINDINGS_PER_CHANGESET
        ],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u64>(2)?,
            ))
        },
    )?;
    rows.map(|row| {
        let (scalar_changeset_id, body, scalar_version) = row?;
        let finding: Finding = decode(&body)?;
        if scalar_changeset_id != changeset_id.to_string() || finding.changeset_id() != changeset_id
        {
            return Err(PersistenceError::CorruptOwnership("finding"));
        }
        ensure_version_agreement("finding", finding.id, scalar_version, finding.version())?;
        Ok(finding)
    })
    .collect()
}

fn load_run(connection: &Connection, run_id: Uuid) -> Result<(Run, u64), PersistenceError> {
    let (body, scalar_version): (String, u64) = connection.query_row(
        "SELECT body, version FROM runs WHERE id = ?1",
        [run_id.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let run: Run = decode(&body)?;
    ensure_version_agreement("run", run_id, scalar_version, run.version())?;
    Ok((run, scalar_version))
}

fn load_changeset(
    connection: &Connection,
    changeset_id: Uuid,
) -> Result<(Changeset, u64), PersistenceError> {
    let (body, scalar_version): (String, u64) = connection.query_row(
        "SELECT body, version FROM changesets WHERE id = ?1",
        [changeset_id.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let changeset: Changeset = decode(&body)?;
    ensure_version_agreement(
        "changeset",
        changeset_id,
        scalar_version,
        changeset.version(),
    )?;
    Ok((changeset, scalar_version))
}

fn load_finding(
    connection: &Connection,
    finding_id: Uuid,
) -> Result<(Finding, u64), PersistenceError> {
    let (body, scalar_version): (String, u64) = connection.query_row(
        "SELECT body, version FROM findings WHERE id = ?1",
        [finding_id.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let finding: Finding = decode(&body)?;
    ensure_version_agreement("finding", finding_id, scalar_version, finding.version())?;
    Ok((finding, scalar_version))
}

fn ensure_version_agreement(
    aggregate: &'static str,
    id: Uuid,
    scalar: u64,
    serialized: u64,
) -> Result<(), PersistenceError> {
    if scalar != serialized {
        return Err(PersistenceError::CorruptVersion {
            aggregate,
            id,
            scalar,
            serialized,
        });
    }
    Ok(())
}

fn validate_state_transition<State: Copy>(
    aggregate: &'static str,
    id: Uuid,
    stored_state: State,
    proposed_state: State,
    stored_version: u64,
    proposed_version: u64,
    allows: impl FnOnce(State, State) -> bool,
) -> Result<(), PersistenceError> {
    let expected_version = proposed_version.saturating_sub(1);
    if stored_version != expected_version {
        return Err(PersistenceError::VersionConflict {
            aggregate,
            id,
            expected: expected_version,
            actual: stored_version,
        });
    }
    if proposed_version != stored_version.saturating_add(1) || !allows(stored_state, proposed_state)
    {
        return Err(PersistenceError::InvalidPersistedTransition(aggregate));
    }
    Ok(())
}

fn validate_versioned_transition<State: Copy + PartialEq>(
    aggregate: &'static str,
    id: Uuid,
    stored_state: State,
    proposed_state: State,
    stored_version: u64,
    proposed_version: u64,
    allows: impl FnOnce(State, State) -> bool,
) -> Result<(), PersistenceError> {
    let expected_version = if stored_state == proposed_state {
        proposed_version
    } else {
        proposed_version.saturating_sub(1)
    };
    if stored_version != expected_version {
        return Err(PersistenceError::VersionConflict {
            aggregate,
            id,
            expected: expected_version,
            actual: stored_version,
        });
    }
    if stored_state == proposed_state {
        if proposed_version != stored_version {
            return Err(PersistenceError::InvalidPersistedTransition(aggregate));
        }
        return Ok(());
    }
    if proposed_version != stored_version.saturating_add(1) || !allows(stored_state, proposed_state)
    {
        return Err(PersistenceError::InvalidPersistedTransition(aggregate));
    }
    Ok(())
}

fn validate_worktree_transition(
    stored: &Worktree,
    proposed: &Worktree,
) -> Result<(), PersistenceError> {
    if stored.id != proposed.id || stored.changeset_id() != proposed.changeset_id() {
        return Err(PersistenceError::CorruptOwnership("worktree"));
    }
    let expected_version = if stored.state() == proposed.state() {
        proposed.version()
    } else if proposed.state() == WorktreeState::Removed
        && matches!(stored.state(), WorktreeState::Ready | WorktreeState::Failed)
    {
        proposed.version().saturating_sub(2)
    } else {
        proposed.version().saturating_sub(1)
    };
    if stored.version() != expected_version {
        return Err(PersistenceError::VersionConflict {
            aggregate: "worktree",
            id: proposed.id,
            expected: expected_version,
            actual: stored.version(),
        });
    }
    let version_delta = proposed.version().saturating_sub(stored.version());
    let valid = if stored.state() == proposed.state() {
        false
    } else if proposed.state() == WorktreeState::Removed
        && matches!(stored.state(), WorktreeState::Ready | WorktreeState::Failed)
    {
        version_delta == 2
    } else {
        version_delta == 1
            && matches!(
                (stored.state(), proposed.state()),
                (
                    WorktreeState::Creating,
                    WorktreeState::Ready | WorktreeState::Failed
                ) | (
                    WorktreeState::Ready,
                    WorktreeState::Removing | WorktreeState::Failed | WorktreeState::Quarantined
                ) | (
                    WorktreeState::Removing,
                    WorktreeState::Removed | WorktreeState::Ready | WorktreeState::Failed
                ) | (
                    WorktreeState::Failed,
                    WorktreeState::Removing | WorktreeState::Quarantined
                )
            )
    };
    if !valid {
        return Err(PersistenceError::InvalidPersistedTransition("worktree"));
    }
    Ok(())
}

fn update_run(
    transaction: &Transaction<'_>,
    run: &Run,
    expected_version: u64,
) -> Result<(), PersistenceError> {
    let changed = transaction.execute(
        "UPDATE runs SET state = ?2, version = ?3, body = ?4 WHERE id = ?1 AND version = ?5",
        params![
            run.id.to_string(),
            state_json(&run.state())?,
            run.version(),
            encode(run)?,
            expected_version
        ],
    )?;
    check_versioned_update(
        transaction,
        "runs",
        "run",
        run.id,
        expected_version,
        changed,
    )
}

fn update_changeset(
    transaction: &Transaction<'_>,
    changeset: &Changeset,
    expected_version: u64,
) -> Result<(), PersistenceError> {
    let changed = transaction.execute(
        "UPDATE changesets SET state = ?2, version = ?3, body = ?4 WHERE id = ?1 AND version = ?5",
        params![
            changeset.id.to_string(),
            state_json(&changeset.state())?,
            changeset.version(),
            encode(changeset)?,
            expected_version
        ],
    )?;
    check_versioned_update(
        transaction,
        "changesets",
        "changeset",
        changeset.id,
        expected_version,
        changed,
    )
}

fn update_worktree(
    transaction: &Transaction<'_>,
    worktree: &Worktree,
    expected_version: u64,
) -> Result<(), PersistenceError> {
    let changed = transaction.execute(
        "UPDATE worktrees SET state = ?2, version = ?3, body = ?4 WHERE id = ?1 AND version = ?5",
        params![
            worktree.id.to_string(),
            state_json(&worktree.state())?,
            worktree.version(),
            encode(worktree)?,
            expected_version
        ],
    )?;
    check_versioned_update(
        transaction,
        "worktrees",
        "worktree",
        worktree.id,
        expected_version,
        changed,
    )
}

fn update_finding(
    transaction: &Transaction<'_>,
    finding: &Finding,
    expected_version: u64,
) -> Result<(), PersistenceError> {
    let changed = transaction.execute(
        "UPDATE findings SET version = ?2, body = ?3 WHERE id = ?1 AND version = ?4",
        params![
            finding.id.to_string(),
            finding.version(),
            encode(finding)?,
            expected_version
        ],
    )?;
    check_versioned_update(
        transaction,
        "findings",
        "finding",
        finding.id,
        expected_version,
        changed,
    )
}

fn check_versioned_update(
    connection: &Connection,
    table: &'static str,
    aggregate: &'static str,
    id: Uuid,
    expected_version: u64,
    changed: usize,
) -> Result<(), PersistenceError> {
    if changed == 1 {
        return Ok(());
    }
    let query = format!("SELECT version FROM {table} WHERE id = ?1");
    let actual = connection
        .query_row(&query, [id.to_string()], |row| row.get::<_, u64>(0))
        .optional()?;
    match actual {
        Some(actual) => Err(PersistenceError::VersionConflict {
            aggregate,
            id,
            expected: expected_version,
            actual,
        }),
        None => Err(PersistenceError::NotFound(aggregate)),
    }
}

type ProcessSupervisionRow = (String, String, String, i64, String);

fn process_supervision_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProcessSupervisionRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
}

fn load_process_supervision_from(
    connection: &Connection,
    run_id: Uuid,
    supervision_id: Uuid,
) -> Result<Option<ProcessSupervisionRecord>, PersistenceError> {
    let row = connection
        .query_row(
            "SELECT id, run_id, status, updated_at_unix_ms, body
             FROM process_supervision_records
             WHERE id = ?1 AND run_id = ?2",
            params![supervision_id.to_string(), run_id.to_string()],
            process_supervision_row,
        )
        .optional()?;
    row.map(validate_process_supervision_record).transpose()
}

fn validate_process_supervision_record(
    (id, run_id, status, updated_at_unix_ms, body): ProcessSupervisionRow,
) -> Result<ProcessSupervisionRecord, PersistenceError> {
    let record: ProcessSupervisionRecord = decode(&body)?;
    let scalar_id =
        Uuid::parse_str(&id).map_err(|_| PersistenceError::CorruptProcessSupervision)?;
    let scalar_run_id =
        Uuid::parse_str(&run_id).map_err(|_| PersistenceError::CorruptProcessSupervision)?;
    let scalar_state: SupervisionState =
        decode(&status).map_err(|_| PersistenceError::CorruptProcessSupervision)?;
    if record.run_id != scalar_run_id
        || record.metadata.supervision_id != scalar_id
        || record.metadata.state != scalar_state
        || record.updated_at_unix_ms != updated_at_unix_ms
    {
        return Err(PersistenceError::CorruptProcessSupervision);
    }
    Ok(record)
}

fn same_process_supervision_identity(
    stored: &SupervisionMetadata,
    proposed: &SupervisionMetadata,
) -> bool {
    stored.supervision_id == proposed.supervision_id
        && stored.pid == proposed.pid
        && stored.process_group == proposed.process_group
        && stored.process_start == proposed.process_start
        && stored.executable == proposed.executable
        && stored.command_digest == proposed.command_digest
        && stored.environment_digest == proposed.environment_digest
        && stored.confinement == proposed.confinement
        && stored.supervision_token == proposed.supervision_token
}

fn supervision_state_json(state: SupervisionState) -> Result<String, PersistenceError> {
    encode(&state)
}

type RecoveryActionRow = (String, Option<String>, Option<String>, String, i64, String);

fn recovery_action_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecoveryActionRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn validate_recovery_action_record(
    row: RecoveryActionRow,
) -> Result<RecoveryActionRecord, PersistenceError> {
    let (id, run_id, changeset_id, status, created_at_unix_ms, body) = row;
    let record: RecoveryActionRecord = decode(&body)?;
    let scalar_run_id = run_id
        .map(|value| Uuid::parse_str(&value))
        .transpose()
        .map_err(|_| PersistenceError::CorruptRecoveryAction)?;
    let scalar_changeset_id = changeset_id
        .map(|value| Uuid::parse_str(&value))
        .transpose()
        .map_err(|_| PersistenceError::CorruptRecoveryAction)?;
    let scalar_status: RecoveryActionStatus = decode(&status)?;
    if record.idempotency_key != id
        || record.run_id != scalar_run_id
        || record.changeset_id != scalar_changeset_id
        || record.status != scalar_status
        || record.created_at_unix_ms != created_at_unix_ms
    {
        return Err(PersistenceError::CorruptRecoveryAction);
    }
    Ok(record)
}

type MutationLeaseRow = (String, String, u64, i64, String);

fn mutation_lease_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MutationLeaseRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
}

fn validate_mutation_lease(row: MutationLeaseRow) -> Result<MutationLease, PersistenceError> {
    let (changeset_id, run_id, fencing_epoch, expires_at_unix_ms, body) = row;
    let scalar_changeset_id =
        Uuid::parse_str(&changeset_id).map_err(|_| PersistenceError::CorruptLease)?;
    let scalar_run_id = Uuid::parse_str(&run_id).map_err(|_| PersistenceError::CorruptLease)?;
    let lease: MutationLease = decode(&body)?;
    if lease.changeset_id != scalar_changeset_id
        || lease.run_id != scalar_run_id
        || lease.fencing_epoch != fencing_epoch
        || lease.expires_at_unix_ms != expires_at_unix_ms
    {
        return Err(PersistenceError::CorruptLease);
    }
    Ok(lease)
}

fn load_mutation_lease_from(
    connection: &Connection,
    changeset_id: Uuid,
) -> Result<Option<MutationLease>, PersistenceError> {
    connection
        .query_row(
            "SELECT changeset_id, run_id, fencing_epoch, expires_at_unix_ms, body
             FROM mutation_leases WHERE changeset_id = ?1",
            [changeset_id.to_string()],
            mutation_lease_row,
        )
        .optional()?
        .map(validate_mutation_lease)
        .transpose()
}

type ChangesetFinalizationRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    u64,
    i64,
    i64,
    String,
);

fn changeset_finalization_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ChangesetFinalizationRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
    ))
}

fn prepared_finalization(
    confirmation_digest: &str,
    scope: &ChangesetMutationScope,
    worktree_id: Uuid,
    expected_worktree_version: u64,
    now_unix_ms: i64,
) -> ChangesetFinalization {
    ChangesetFinalization {
        confirmation_digest: confirmation_digest.to_owned(),
        scope: scope.clone(),
        worktree_id,
        expected_worktree_version,
        state: ChangesetFinalizationState::Prepared,
        result: None,
        divergence_detail: None,
        version: 0,
        created_at_unix_ms: now_unix_ms,
        updated_at_unix_ms: now_unix_ms,
    }
}

fn finalization_matches_request(
    stored: &ChangesetFinalization,
    confirmation_digest: &str,
    scope: &ChangesetMutationScope,
    worktree_id: Uuid,
    expected_worktree_version: u64,
) -> bool {
    stored.confirmation_digest == confirmation_digest
        && stored.scope == *scope
        && stored.worktree_id == worktree_id
        && stored.expected_worktree_version == expected_worktree_version
}

fn validate_finalization_scope(scope: &ChangesetMutationScope) -> Result<(), PersistenceError> {
    if !is_git_object_id(&scope.base_sha)
        || !is_git_object_id(&scope.expected_head_sha)
        || !is_sha256_digest(&scope.manifest_sha256)
    {
        return Err(PersistenceError::InvalidFinalizationInput("mutation scope"));
    }
    Ok(())
}

fn validate_prepared_finalization_aggregates(
    finalization: &ChangesetFinalization,
    changeset: &Changeset,
    changeset_version: u64,
    worktree: &Worktree,
    worktree_version: u64,
) -> Result<(), PersistenceError> {
    if changeset.state() != ChangesetState::Reviewable
        || changeset_version != finalization.scope.expected_version
        || changeset.repository_id() != finalization.scope.repository_id
        || changeset.base_sha() != finalization.scope.base_sha
        || changeset.head_sha() != finalization.scope.expected_head_sha
        || worktree.changeset_id() != changeset.id
        || worktree.state() != WorktreeState::Ready
        || worktree_version != finalization.expected_worktree_version
        || worktree.base_sha != finalization.scope.base_sha
    {
        return Err(PersistenceError::FinalizationConflict);
    }
    Ok(())
}

fn validate_completed_finalization_replay(
    finalization: &ChangesetFinalization,
    changeset: &Changeset,
    changeset_version: u64,
    worktree: &Worktree,
    worktree_version: u64,
) -> Result<(), PersistenceError> {
    let expected_changeset_version = finalization.scope.expected_version.saturating_add(1);
    let expected_worktree_version = finalization.expected_worktree_version.saturating_add(2);
    let aggregate_identity_matches = changeset.repository_id() == finalization.scope.repository_id
        && changeset.base_sha() == finalization.scope.base_sha
        && worktree.changeset_id() == changeset.id
        && worktree.base_sha == finalization.scope.base_sha
        && changeset_version == expected_changeset_version
        && worktree_version == expected_worktree_version
        && worktree.state() == WorktreeState::Removed;
    let result_matches = match finalization.result.as_ref() {
        Some(ChangesetFinalizationResult::Commit {
            resulting_head_sha,
            app_ref,
        }) => {
            changeset.state() == ChangesetState::Committed
                && changeset.head_sha() == resulting_head_sha
                && app_ref == &expected_changeset_ref(changeset.id)
        }
        Some(ChangesetFinalizationResult::Discard) => {
            changeset.state() == ChangesetState::Discarded
                && changeset.head_sha() == finalization.scope.expected_head_sha
        }
        None => false,
    };
    if !aggregate_identity_matches || !result_matches {
        return Err(PersistenceError::FinalizationConflict);
    }
    Ok(())
}

fn validate_divergent_finalization_replay(
    finalization: &ChangesetFinalization,
    changeset: &Changeset,
    changeset_version: u64,
    worktree: &Worktree,
    worktree_version: u64,
) -> Result<(), PersistenceError> {
    if changeset.state() != ChangesetState::Divergent
        || changeset.repository_id() != finalization.scope.repository_id
        || changeset.base_sha() != finalization.scope.base_sha
        || changeset_version != finalization.scope.expected_version.saturating_add(1)
        || worktree.state() != WorktreeState::Quarantined
        || worktree.changeset_id() != changeset.id
        || worktree.base_sha != finalization.scope.base_sha
        || worktree_version != finalization.expected_worktree_version.saturating_add(1)
    {
        return Err(PersistenceError::FinalizationConflict);
    }
    Ok(())
}

fn expected_changeset_ref(changeset_id: Uuid) -> String {
    format!("refs/jet-black/changesets/{changeset_id}")
}

fn is_sha256_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_git_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_changeset_finalization(
    row: ChangesetFinalizationRow,
) -> Result<ChangesetFinalization, PersistenceError> {
    let (
        confirmation_digest,
        changeset_id,
        repository_id,
        checkpoint_id,
        worktree_id,
        kind,
        status,
        version,
        created_at_unix_ms,
        updated_at_unix_ms,
        body,
    ) = row;
    let finalization: ChangesetFinalization = decode(&body)?;
    let scalar_changeset_id = Uuid::parse_str(&changeset_id)
        .map_err(|_| PersistenceError::CorruptIdentifier("finalization changeset id"))?;
    let scalar_repository_id = Uuid::parse_str(&repository_id)
        .map_err(|_| PersistenceError::CorruptIdentifier("finalization repository id"))?;
    let scalar_checkpoint_id = Uuid::parse_str(&checkpoint_id)
        .map_err(|_| PersistenceError::CorruptIdentifier("finalization checkpoint id"))?;
    let scalar_worktree_id = Uuid::parse_str(&worktree_id)
        .map_err(|_| PersistenceError::CorruptIdentifier("finalization worktree id"))?;
    let scalar_kind: ChangesetMutationKind = decode(&kind)?;
    let scalar_status: ChangesetFinalizationState = decode(&status)?;
    if finalization.confirmation_digest != confirmation_digest
        || finalization.scope.changeset_id != scalar_changeset_id
        || finalization.scope.repository_id != scalar_repository_id
        || finalization.scope.checkpoint_id != scalar_checkpoint_id
        || finalization.worktree_id != scalar_worktree_id
        || finalization.scope.kind != scalar_kind
        || finalization.state != scalar_status
        || finalization.version != version
        || finalization.created_at_unix_ms != created_at_unix_ms
        || finalization.updated_at_unix_ms != updated_at_unix_ms
        || finalization.updated_at_unix_ms < finalization.created_at_unix_ms
        || finalization.confirmation_digest != finalization.scope.digest()
        || validate_finalization_scope(&finalization.scope).is_err()
    {
        return Err(PersistenceError::CorruptFinalization);
    }
    let valid_state = match finalization.state {
        ChangesetFinalizationState::Prepared => {
            finalization.version == 0
                && finalization.result.is_none()
                && finalization.divergence_detail.is_none()
                && finalization.updated_at_unix_ms == finalization.created_at_unix_ms
        }
        ChangesetFinalizationState::Completed => {
            finalization.version == 1
                && finalization.divergence_detail.is_none()
                && finalization.result.as_ref().is_some_and(|result| {
                    validate_finalization_result(&finalization.scope, result).is_ok()
                })
        }
        ChangesetFinalizationState::Divergent => {
            finalization.version == 1
                && finalization.result.is_none()
                && finalization
                    .divergence_detail
                    .as_ref()
                    .is_some_and(|detail| {
                        !detail.is_empty()
                            && detail.len() <= domain::limits::MAX_SEMANTIC_TEXT_BYTES
                    })
        }
    };
    if !valid_state {
        return Err(PersistenceError::CorruptFinalization);
    }
    Ok(finalization)
}

fn load_changeset_finalization(
    connection: &Connection,
    confirmation_digest: &str,
) -> Result<Option<ChangesetFinalization>, PersistenceError> {
    connection
        .query_row(
            "SELECT confirmation_digest, changeset_id, repository_id, checkpoint_id, worktree_id, kind, status, version, created_at_unix_ms, updated_at_unix_ms, body
             FROM changeset_finalizations
             WHERE confirmation_digest = ?1",
            [confirmation_digest],
            changeset_finalization_row,
        )
        .optional()?
        .map(validate_changeset_finalization)
        .transpose()
}

fn load_changeset_finalization_for_changeset(
    connection: &Connection,
    changeset_id: Uuid,
) -> Result<Option<ChangesetFinalization>, PersistenceError> {
    connection
        .query_row(
            "SELECT confirmation_digest, changeset_id, repository_id, checkpoint_id, worktree_id, kind, status, version, created_at_unix_ms, updated_at_unix_ms, body
             FROM changeset_finalizations
             WHERE changeset_id = ?1",
            [changeset_id.to_string()],
            changeset_finalization_row,
        )
        .optional()?
        .map(validate_changeset_finalization)
        .transpose()
}

fn insert_changeset_finalization(
    transaction: &Transaction<'_>,
    finalization: &ChangesetFinalization,
) -> Result<(), PersistenceError> {
    transaction.execute(
        "INSERT INTO changeset_finalizations (
             confirmation_digest, changeset_id, repository_id, checkpoint_id, worktree_id,
             kind, status, version, created_at_unix_ms, updated_at_unix_ms, body
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            finalization.confirmation_digest,
            finalization.scope.changeset_id.to_string(),
            finalization.scope.repository_id.to_string(),
            finalization.scope.checkpoint_id.to_string(),
            finalization.worktree_id.to_string(),
            state_json(&finalization.scope.kind)?,
            state_json(&finalization.state)?,
            finalization.version,
            finalization.created_at_unix_ms,
            finalization.updated_at_unix_ms,
            encode(finalization)?,
        ],
    )?;
    Ok(())
}

fn update_changeset_finalization(
    transaction: &Transaction<'_>,
    finalization: &ChangesetFinalization,
    expected_version: u64,
) -> Result<(), PersistenceError> {
    let changed = transaction.execute(
        "UPDATE changeset_finalizations
         SET status = ?2, version = ?3, updated_at_unix_ms = ?4, body = ?5
         WHERE confirmation_digest = ?1 AND version = ?6",
        params![
            finalization.confirmation_digest,
            state_json(&finalization.state)?,
            finalization.version,
            finalization.updated_at_unix_ms,
            encode(finalization)?,
            expected_version,
        ],
    )?;
    if changed == 1 {
        return Ok(());
    }
    let actual = transaction
        .query_row(
            "SELECT version FROM changeset_finalizations WHERE confirmation_digest = ?1",
            [finalization.confirmation_digest.as_str()],
            |row| row.get::<_, u64>(0),
        )
        .optional()?;
    match actual {
        Some(actual) => Err(PersistenceError::VersionConflict {
            aggregate: "changeset finalization",
            id: finalization.scope.changeset_id,
            expected: expected_version,
            actual,
        }),
        None => Err(PersistenceError::NotFound("changeset finalization")),
    }
}

fn validate_finalization_result(
    scope: &ChangesetMutationScope,
    result: &ChangesetFinalizationResult,
) -> Result<(), PersistenceError> {
    match (scope.kind, result) {
        (
            ChangesetMutationKind::Commit,
            ChangesetFinalizationResult::Commit {
                resulting_head_sha,
                app_ref,
            },
        ) if is_git_object_id(resulting_head_sha)
            && app_ref == &expected_changeset_ref(scope.changeset_id) =>
        {
            Ok(())
        }
        (ChangesetMutationKind::Commit, ChangesetFinalizationResult::Commit { .. }) => {
            Err(PersistenceError::InvalidFinalizationInput("commit result"))
        }
        (ChangesetMutationKind::Discard, ChangesetFinalizationResult::Discard) => Ok(()),
        _ => Err(PersistenceError::FinalizationKindMismatch),
    }
}

fn validate_semantic_event(event: &SemanticEventKind) -> Result<(), PersistenceError> {
    if matches!(event, SemanticEventKind::Text { text } if text.len() > domain::limits::MAX_SEMANTIC_TEXT_BYTES)
    {
        return Err(PersistenceError::ResourceLimit("semantic event text"));
    }
    Ok(())
}

fn validate_finding_owners(findings: &[Finding]) -> Result<Uuid, PersistenceError> {
    let changeset_id = findings
        .first()
        .ok_or(PersistenceError::InvalidPersistedTransition(
            "empty finding batch",
        ))?
        .changeset_id();
    if findings
        .iter()
        .any(|finding| finding.changeset_id() != changeset_id)
    {
        return Err(PersistenceError::MixedFindingOwners);
    }
    Ok(changeset_id)
}

fn save_findings(
    transaction: &Transaction<'_>,
    changeset_id: Uuid,
    findings: &[Finding],
) -> Result<Vec<Finding>, PersistenceError> {
    let mut existing = HashMap::new();
    let mut statement =
        transaction.prepare("SELECT body FROM findings WHERE owner = ?1 ORDER BY rowid")?;
    let rows = statement.query_map([changeset_id.to_string()], |row| row.get::<_, String>(0))?;
    for row in rows {
        let finding: Finding = decode(&row?)?;
        existing.insert(finding_dedup_key(&finding)?, finding);
    }
    drop(statement);

    let mut canonical = Vec::with_capacity(findings.len());
    for finding in findings {
        let dedup_key = finding_dedup_key(finding)?;
        if let Some(existing_finding) = existing.get(&dedup_key) {
            canonical.push(existing_finding.clone());
            continue;
        }
        if existing.len() >= domain::limits::MAX_FINDINGS_PER_CHANGESET {
            return Err(PersistenceError::ResourceLimit("findings per changeset"));
        }
        transaction.execute(
            "INSERT INTO findings (id, owner, version, body) VALUES (?1, ?2, ?3, ?4)",
            params![
                finding.id.to_string(),
                changeset_id.to_string(),
                finding.version(),
                encode(finding)?
            ],
        )?;
        existing.insert(dedup_key, finding.clone());
        canonical.push(finding.clone());
    }
    Ok(canonical)
}

fn finding_dedup_key(finding: &Finding) -> Result<String, PersistenceError> {
    encode(&(
        &finding.path,
        &finding.blob_identity,
        &finding.line_range,
        &finding.category,
        &finding.severity,
        &finding.message,
        &finding.evidence,
        finding.state(),
    ))
}

fn encode<T: serde::Serialize>(value: &T) -> Result<String, PersistenceError> {
    Ok(serde_json::to_string(value)?)
}
fn decode<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, PersistenceError> {
    Ok(serde_json::from_str(value)?)
}
fn state_json<T: serde::Serialize>(value: &T) -> Result<String, PersistenceError> {
    encode(value)
}
fn append_aggregate_event(
    transaction: &rusqlite::Transaction<'_>,
    kind: &str,
    id: Uuid,
    state: &str,
) -> Result<(), PersistenceError> {
    let sequence: u64 = transaction.query_row("SELECT COALESCE(MAX(sequence), 0) + 1 FROM aggregate_events WHERE aggregate_kind = ?1 AND aggregate_id = ?2", params![kind, id.to_string()], |row| row.get(0))?;
    transaction.execute("INSERT INTO aggregate_events (aggregate_kind, aggregate_id, sequence, body) VALUES (?1, ?2, ?3, ?4)", params![kind, id.to_string(), sequence, format!("{{\"state\":\"{state}\"}}")])?;
    Ok(())
}

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0} was not found")]
    NotFound(&'static str),
    #[error("domain transition failed: {0}")]
    Domain(#[from] domain::DomainError),
    #[error("invalid persisted {0} transition")]
    InvalidPersistedTransition(&'static str),
    #[error("approval does not match the run's pending proposal")]
    ApprovalDoesNotMatchRun,
    #[error("run provider execution was already claimed")]
    RunDriveAlreadyClaimed,
    #[error("resource limit exceeded for {0}")]
    ResourceLimit(&'static str),
    #[error("a finding batch must belong to one changeset")]
    MixedFindingOwners,
    #[error("database schema version {database} is newer than runtime version {runtime}")]
    UnsupportedSchemaVersion { database: u32, runtime: u32 },
    #[error(
        "stale {aggregate} writer for {id}: expected database version {expected}, found {actual}"
    )]
    VersionConflict {
        aggregate: &'static str,
        id: Uuid,
        expected: u64,
        actual: u64,
    },
    #[error(
        "stored {aggregate} {id} has scalar version {scalar} but serialized version {serialized}"
    )]
    CorruptVersion {
        aggregate: &'static str,
        id: Uuid,
        scalar: u64,
        serialized: u64,
    },
    #[error(
        "changeset {changeset_id} has an active mutation lease owned by run {run_id} until {expires_at_unix_ms}"
    )]
    MutationLeaseHeld {
        changeset_id: Uuid,
        run_id: Uuid,
        expires_at_unix_ms: i64,
    },
    #[error("mutation lease does not match the requested owner and fencing epoch")]
    MutationLeaseMismatch,
    #[error("mutation lease has expired")]
    MutationLeaseExpired,
    #[error("mutation lease TTL must be greater than zero and fit in milliseconds")]
    InvalidLeaseTtl,
    #[error("stored mutation lease scalar fields do not match its serialized body")]
    CorruptLease,
    #[error("mutation lease fencing epoch is exhausted")]
    FencingEpochExhausted,
    #[error("invalid process supervision metadata transition or identity")]
    InvalidProcessSupervisionTransition,
    #[error("stored process supervision scalar fields do not match its serialized body")]
    CorruptProcessSupervision,
    #[error("recovery action idempotency key must not be empty")]
    InvalidRecoveryAction,
    #[error("recovery action idempotency key was reused for different content")]
    RecoveryActionConflict,
    #[error("stored recovery action scalar fields do not match its serialized body")]
    CorruptRecoveryAction,
    #[error("changeset finalization confirmation digest does not match its scope")]
    FinalizationDigestMismatch,
    #[error("invalid changeset finalization input: {0}")]
    InvalidFinalizationInput(&'static str),
    #[error("changeset finalization conflicts with an existing operation or aggregate state")]
    FinalizationConflict,
    #[error("changeset finalization result does not match the prepared operation")]
    FinalizationKindMismatch,
    #[error("stored changeset finalization scalar fields do not match its serialized body")]
    CorruptFinalization,
    #[error("artifact policy limits or retention are invalid")]
    InvalidArtifactPolicy,
    #[error("artifact root must be absolute")]
    InvalidArtifactRoot,
    #[error("stored artifact metadata is corrupt")]
    ArtifactMetadataCorrupt,
    #[error("artifact content failed integrity verification")]
    ArtifactIntegrityMismatch,
    #[error("stored {0} ownership fields do not agree")]
    CorruptOwnership(&'static str),
    #[error("stored {0} is not a valid UUID")]
    CorruptIdentifier(&'static str),
}
