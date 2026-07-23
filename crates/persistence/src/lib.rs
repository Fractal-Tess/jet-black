use domain::{
    ActionProposal, Approval, ApprovalScope, ApprovedAction, Changeset, ChangesetState, Checkpoint,
    Finding, Repository, Run, RunState, Worktree,
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

const CURRENT_SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone)]
pub struct SqliteStore {
    path: PathBuf,
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
                "INSERT INTO worktrees (id, changeset_id, body) VALUES (?1, ?2, ?3)",
                params![
                    worktree.id.to_string(),
                    worktree.changeset_id().to_string(),
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
        self.with_connection(|connection| {
            let body = connection.query_row("SELECT body FROM worktrees WHERE changeset_id = ?1 ORDER BY rowid DESC LIMIT 1", [changeset_id.to_string()], |row| row.get::<_, String>(0)).optional()?;
            body.map(|value| decode(&value)).transpose()
        })
    }

    pub fn update_worktree(&self, worktree: &Worktree) -> Result<(), PersistenceError> {
        self.with_connection(|connection| {
            let changed = connection.execute(
                "UPDATE worktrees SET body = ?2 WHERE id = ?1",
                params![worktree.id.to_string(), encode(worktree)?],
            )?;
            if changed != 1 {
                return Err(PersistenceError::NotFound("worktree"));
            }
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
            if stored.state() != RunState::Running
                || run.state() != RunState::AwaitingApproval
                || run.proposal_digest() != Some(approval.digest())
                || approval.run_id() != run.id
            {
                return Err(PersistenceError::ApprovalDoesNotMatchRun);
            }
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
            let mut statement = connection.prepare(
                "SELECT body FROM semantic_events WHERE run_id = ?1 AND sequence > ?2 ORDER BY sequence LIMIT ?3",
            )?;
            let rows = statement.query_map(
                params![run_id.to_string(), after_sequence.min(i64::MAX as u64) as i64, limit as i64],
                |row| row.get::<_, String>(0),
            )?;
            rows.map(|row| decode(&row?)).collect()
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

    pub fn save_proposed_change(
        &self,
        run_id: Uuid,
        proposal: &ActionProposal,
        content: &[u8],
    ) -> Result<(), PersistenceError> {
        if content.len() > domain::limits::MAX_APPROVED_FILE_BYTES {
            return Err(PersistenceError::ResourceLimit("approved file content"));
        }
        let proposed_change = StoredProposedChange {
            proposal: proposal.clone(),
            content: content.to_vec(),
        };
        self.with_connection(|connection| {
            connection.execute(
                "INSERT INTO proposed_changes (owner, body) VALUES (?1, ?2)",
                params![run_id.to_string(), encode(&proposed_change)?],
            )?;
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
        self.with_connection(|connection| {
            let body = connection
                .query_row(
                    "SELECT body FROM approvals WHERE owner = ?1 ORDER BY rowid DESC LIMIT 1",
                    [run_id.to_string()],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            body.map(|value| decode(&value)).transpose()
        })
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
            let approval_body: String = transaction.query_row(
                "SELECT body FROM approvals WHERE id = ?1 AND owner = ?2",
                params![approval_id.to_string(), run_id.to_string()],
                |row| row.get(0),
            )?;
            let approval: Approval = decode(&approval_body)?;
            if run.state() != RunState::AwaitingApproval
                || run.proposal_digest() != Some(approval.digest())
            {
                return Err(PersistenceError::ApprovalDoesNotMatchRun);
            }
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
            let approval_body: String = transaction.query_row(
                "SELECT body FROM approvals WHERE id = ?1 AND owner = ?2",
                params![approval_id.to_string(), run_id.to_string()],
                |row| row.get(0),
            )?;
            let mut approval: Approval = decode(&approval_body)?;
            if run.state() != RunState::AwaitingApproval
                || run.changeset_id() != presented.changeset_id
                || run.proposal_digest() != Some(approval.digest())
            {
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
        let changeset_id = findings[0].changeset_id();
        if findings
            .iter()
            .any(|finding| finding.changeset_id() != changeset_id)
        {
            return Err(PersistenceError::MixedFindingOwners);
        }

        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let mut existing = HashMap::new();
            let mut statement =
                transaction.prepare("SELECT body FROM findings WHERE owner = ?1 ORDER BY rowid")?;
            let rows =
                statement.query_map([changeset_id.to_string()], |row| row.get::<_, String>(0))?;
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
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
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
            _ => unreachable!("all schema migrations are explicitly ordered"),
        }
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

fn validate_semantic_event(event: &SemanticEventKind) -> Result<(), PersistenceError> {
    if matches!(event, SemanticEventKind::Text { text } if text.len() > domain::limits::MAX_SEMANTIC_TEXT_BYTES)
    {
        return Err(PersistenceError::ResourceLimit("semantic event text"));
    }
    Ok(())
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
}
