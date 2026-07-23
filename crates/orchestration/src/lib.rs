use agents::AgentProvider;
use domain::{
    Approval, ApprovalScope, Changeset, ChangesetState, Checkpoint, Id, Repository, Run, RunState,
    TicketRef, Worktree, WorktreeState,
};
use execution::{
    CancellationToken, ProcessSpec, ProcessSupervisor, SupervisionState, TerminalOutcome,
    TerminationReason, TerminationStatus,
};
use git::GitService;
use persistence::{MutationLease, RecoveryReport, SqliteStore};
use protocol::{
    ApprovalRequest, CheckpointResponse, DiffResponse, EventCursor, EventPage, FindingsResponse,
    LocalCommand, OrderedRunEvent, RecoveryAction, RecoveryResponse, RunCompletedResponse,
    RunStartedResponse, SemanticEventKind,
};
use std::{path::Path, sync::Mutex, time::Duration};
use thiserror::Error;
use uuid::Uuid;

pub const DEFAULT_APPROVAL_TTL: Duration = Duration::from_secs(300);
pub const DEFAULT_MUTATION_LEASE_TTL: Duration = Duration::from_secs(600);

pub struct LocalOrchestrator<P> {
    store: SqliteStore,
    git: GitService,
    provider: P,
    approval_ttl: Duration,
    supervisor: ProcessSupervisor,
    mutation_lease_ttl: Duration,
    supervision_transition_gate: Mutex<()>,
}

impl<P: AgentProvider> LocalOrchestrator<P> {
    pub fn new(store: SqliteStore, git: GitService, provider: P, approval_ttl: Duration) -> Self {
        Self::with_process_supervision(
            store,
            git,
            provider,
            approval_ttl,
            DEFAULT_MUTATION_LEASE_TTL,
            ProcessSupervisor::new(),
        )
    }

    pub fn with_process_supervision(
        store: SqliteStore,
        git: GitService,
        provider: P,
        approval_ttl: Duration,
        mutation_lease_ttl: Duration,
        supervisor: ProcessSupervisor,
    ) -> Self {
        Self {
            store,
            git,
            provider,
            approval_ttl,
            supervisor,
            mutation_lease_ttl,
            supervision_transition_gate: Mutex::new(()),
        }
    }

    pub fn handle(&self, command: LocalCommand) -> Result<CommandOutcome, OrchestrationError> {
        match command {
            LocalCommand::RegisterRepository { path } => Ok(CommandOutcome::RepositoryRegistered(
                self.register_repository(&path)?,
            )),
            LocalCommand::CreateChangeset {
                repository_id,
                base_sha,
                ticket,
            } => Ok(CommandOutcome::ChangesetCreated(self.create_changeset(
                repository_id,
                &base_sha,
                ticket,
            )?)),
            LocalCommand::StartRun { changeset_id } => {
                Ok(CommandOutcome::RunStarted(self.start_run(changeset_id)?))
            }
            LocalCommand::InterruptRun { run_id } => {
                Ok(CommandOutcome::RunInterrupted(self.interrupt_run(run_id)?))
            }
            LocalCommand::RespondToApproval {
                run_id,
                scope,
                approved,
            } => {
                if approved {
                    Ok(CommandOutcome::RunCompleted(
                        self.approve_and_complete(run_id, &scope)?,
                    ))
                } else {
                    Ok(CommandOutcome::ApprovalRejected(
                        self.reject_approval(run_id, &scope)?,
                    ))
                }
            }
            LocalCommand::GetCheckpoint { run_id } => {
                let checkpoint = self
                    .store
                    .checkpoint_for_run(run_id)?
                    .ok_or(OrchestrationError::NotFound("checkpoint"))?;
                Ok(CommandOutcome::Checkpoint(CheckpointResponse {
                    checkpoint,
                }))
            }
            LocalCommand::GetDiff { changeset_id } => {
                let checkpoint = self
                    .store
                    .checkpoint_for_changeset(changeset_id)?
                    .ok_or(OrchestrationError::NotFound("checkpoint"))?;
                Ok(CommandOutcome::Diff(DiffResponse {
                    changeset_id,
                    unified_diff: checkpoint.diff,
                }))
            }
            LocalCommand::GetRecovery => Ok(CommandOutcome::Recovery(RecoveryResponse {
                actions: self
                    .store
                    .recovery_actions()?
                    .into_iter()
                    .map(|record| record.action)
                    .collect(),
            })),
            LocalCommand::GetFindings { changeset_id } => {
                Ok(CommandOutcome::Findings(FindingsResponse {
                    changeset_id,
                    findings: self.store.findings_for_changeset(changeset_id)?,
                }))
            }
            LocalCommand::GetEvents { .. }
            | LocalCommand::GetSnapshot { .. }
            | LocalCommand::GetHistory { .. }
            | LocalCommand::PreviewCommit { .. }
            | LocalCommand::CommitChangeset { .. }
            | LocalCommand::PreviewDiscard { .. }
            | LocalCommand::DiscardChangeset { .. } => Err(OrchestrationError::UnsupportedCommand),
        }
    }

    pub fn register_repository(&self, path: &Path) -> Result<Repository, OrchestrationError> {
        let repository = self.git.register(path)?;
        self.store.save_repository(&repository)?;
        Ok(repository)
    }

    pub fn create_changeset(
        &self,
        repository_id: Id,
        expected_base_sha: &str,
        ticket: Option<TicketRef>,
    ) -> Result<Changeset, OrchestrationError> {
        let repository = self.repository(repository_id)?;
        if repository.base_sha != expected_base_sha {
            return Err(OrchestrationError::StaleBase);
        }
        let changeset = match ticket {
            Some(ticket) => Changeset::new(repository.id, repository.base_sha).with_ticket(ticket),
            None => Changeset::new(repository.id, repository.base_sha),
        };
        self.store.save_changeset(&changeset)?;
        Ok(changeset)
    }

    pub fn start_run(&self, changeset_id: Id) -> Result<RunStarted, OrchestrationError> {
        let mut changeset = self.changeset(changeset_id)?;
        let repository = self.repository(changeset.repository_id())?;
        let mut worktree = self.git.create_worktree(&repository, changeset.id)?;
        if let Err(error) = self.store.save_worktree(&worktree) {
            self.git.cleanup_worktree(&repository, &mut worktree)?;
            return Err(error.into());
        }

        let mut run = Run::new(changeset.id);
        run.start()?;
        if let Err(error) = self.store.save_run(&run) {
            self.cleanup_setup_worktree(&repository, &mut worktree)?;
            return Err(error.into());
        }
        if let Err(error) = changeset.activate() {
            self.fail_active_run(run.id)?;
            self.cleanup_setup_worktree(&repository, &mut worktree)?;
            return Err(error.into());
        }
        if let Err(error) = self.store.persist_changeset_transition(&changeset) {
            self.fail_active_run(run.id)?;
            self.cleanup_setup_worktree(&repository, &mut worktree)?;
            return Err(error.into());
        }
        if let Err(error) = self.store.acquire_mutation_lease(
            changeset.id,
            run.id,
            current_unix_ms(),
            self.mutation_lease_ttl,
        ) {
            self.compensate_failed_start(&mut changeset, &repository, &mut worktree, run.id)?;
            return Err(error.into());
        }
        match self.prepare_approval(&repository, &changeset, &worktree, &mut run) {
            Ok(started) => Ok(started),
            Err(error) => {
                self.compensate_failed_start(&mut changeset, &repository, &mut worktree, run.id)?;
                Err(error)
            }
        }
    }

    pub fn reject_approval(
        &self,
        run_id: Id,
        presented: &ApprovalScope,
    ) -> Result<Run, OrchestrationError> {
        let approval = self.approval(run_id)?;
        if approval.scope() != presented || approval.digest() != presented.digest() {
            return Err(OrchestrationError::ApprovalMismatch);
        }
        let _transition_guard = self
            .supervision_transition_gate
            .lock()
            .map_err(|_| OrchestrationError::SupervisionStateUnavailable)?;
        let run = self.store.reject_and_interrupt(run_id, approval.id)?;
        let recover_changeset = self.mark_changeset_recoverable(run.changeset_id());
        let release_lease = self.release_run_lease(run.changeset_id(), run.id);
        recover_changeset?;
        release_lease?;
        Ok(run)
    }

    pub fn approve_and_complete(
        &self,
        run_id: Id,
        presented: &ApprovalScope,
    ) -> Result<RunCompleted, OrchestrationError> {
        let approval = self.approval(run_id)?;
        if approval.scope() != presented || approval.digest() != presented.digest() {
            return Err(OrchestrationError::ApprovalMismatch);
        }
        let _transition_guard = self
            .supervision_transition_gate
            .lock()
            .map_err(|_| OrchestrationError::SupervisionStateUnavailable)?;
        let pending_run = self.run(run_id)?;
        let lease = self
            .store
            .mutation_lease(pending_run.changeset_id())?
            .ok_or(OrchestrationError::MutationLeaseUnavailable)?;
        self.store.verify_mutation_lease(
            pending_run.changeset_id(),
            run_id,
            lease.fencing_epoch,
            current_unix_ms(),
        )?;
        let mut changeset = self.changeset(pending_run.changeset_id())?;
        let repository = self.repository(changeset.repository_id())?;
        let worktree = self.worktree(changeset.id)?;
        let proposed_change = self
            .store
            .proposed_change_for_run(run_id)?
            .ok_or(OrchestrationError::NotFound("proposed change"))?;

        if proposed_change.proposal != presented.proposal
            || git::content_digest(&proposed_change.content) != presented.proposal.content_sha256
        {
            return Err(OrchestrationError::ApprovalMismatch);
        }
        if self.git.head_sha(&worktree)? != presented.head_sha {
            return Err(OrchestrationError::ApprovalMismatch);
        }

        let now_unix_ms = current_unix_ms();
        policy::authorize_exact_action(&approval, run_id, presented, now_unix_ms)?;
        let authorized =
            self.store
                .authorize_and_resume(run_id, approval.id, presented, now_unix_ms)?;
        let mut run = authorized.run;
        let action_digest = authorized.approved_action.action_digest().to_owned();
        let changeset_id = changeset.id;
        let result: Result<RunCompleted, OrchestrationError> = (|| {
            self.store.verify_mutation_lease(
                changeset.id,
                run.id,
                lease.fencing_epoch,
                current_unix_ms(),
            )?;
            self.git.write_approved_file(
                &repository,
                &worktree,
                authorized.approved_action,
                &proposed_change.content,
            )?;
            self.store.persist_run_transition(
                &run,
                SemanticEventKind::ActionResult {
                    digest: action_digest,
                    success: true,
                },
            )?;
            self.store.persist_run_transition(
                &run,
                SemanticEventKind::FileChange {
                    path: proposed_change.proposal.target_path.clone(),
                },
            )?;

            let diff = self.git.diff(
                &worktree,
                std::slice::from_ref(&proposed_change.proposal.target_path),
            )?;
            let head_sha = self.git.head_sha(&worktree)?;
            changeset.mark_reviewable(head_sha.clone())?;
            run.mark_reviewable()?;
            run.complete()?;
            let checkpoint = Checkpoint {
                id: Uuid::new_v4(),
                run_id: run.id,
                base_sha: repository.base_sha,
                head_sha,
                diff,
            };
            self.store
                .complete_with_checkpoint(&changeset, &run, &checkpoint)?;

            Ok(RunCompleted {
                run,
                changeset,
                checkpoint,
            })
        })();

        match result {
            Ok(completed) => {
                self.release_exact_lease(&lease)?;
                Ok(completed)
            }
            Err(error) => {
                let fail_run = self.fail_active_run(run_id);
                let recover_changeset = self.mark_changeset_recoverable(changeset_id);
                let release_lease = self.release_exact_lease(&lease);
                fail_run?;
                recover_changeset?;
                release_lease?;
                Err(error)
            }
        }
    }

    pub fn interrupt_run(&self, run_id: Id) -> Result<Run, OrchestrationError> {
        let _transition_guard = self
            .supervision_transition_gate
            .lock()
            .map_err(|_| OrchestrationError::SupervisionStateUnavailable)?;
        let mut run = self.run(run_id)?;
        if run.state() == RunState::Interrupted {
            self.release_run_lease(run.changeset_id(), run.id)?;
            return Ok(run);
        }

        if let Some(record) = self.store.latest_process_supervision_for_run(run_id)?
            && record.metadata.state != SupervisionState::Terminated
        {
            let termination = self
                .supervisor
                .terminate(&record.metadata)
                .map_err(OrchestrationError::Execution)?;
            if !matches!(
                termination.status,
                TerminationStatus::Terminated | TerminationStatus::AlreadyExited
            ) {
                return Err(OrchestrationError::ProcessTerminationUnverified);
            }
            let mut terminated_metadata = record.metadata;
            terminated_metadata.state = SupervisionState::Terminated;
            terminated_metadata.termination_reason = Some(TerminationReason::Requested);
            self.store.mark_process_supervision_terminated(
                run_id,
                &terminated_metadata,
                current_unix_ms(),
            )?;
        }

        run.interrupt()?;
        self.store.persist_run_transition(
            &run,
            SemanticEventKind::Lifecycle {
                state: RunState::Interrupted,
            },
        )?;
        let recover_changeset = self.mark_changeset_recoverable(run.changeset_id());
        let release_lease = self.release_run_lease(run.changeset_id(), run.id);
        recover_changeset?;
        release_lease?;
        Ok(run)
    }

    pub fn cleanup_changeset(&self, changeset_id: Id) -> Result<Worktree, OrchestrationError> {
        let changeset = self.changeset(changeset_id)?;
        let repository = self.repository(changeset.repository_id())?;
        let mut worktree = self.worktree(changeset.id)?;
        if worktree.state() == WorktreeState::Removed {
            return Ok(worktree);
        }
        self.git.cleanup_worktree(&repository, &mut worktree)?;
        self.store.update_worktree(&worktree)?;
        Ok(worktree)
    }

    pub fn events(&self, run_id: Id) -> Result<Vec<OrderedRunEvent>, OrchestrationError> {
        Ok(self.store.events(run_id)?)
    }

    pub fn run_state(&self, run_id: Id) -> Result<RunState, OrchestrationError> {
        Ok(self.run(run_id)?.state())
    }

    pub fn events_after(
        &self,
        run_id: Id,
        after_sequence: u64,
        limit: usize,
    ) -> Result<EventPage, OrchestrationError> {
        let events = self.store.events_after(run_id, after_sequence, limit)?;
        let next_sequence = events.last().map_or(after_sequence, |event| event.sequence);
        Ok(EventPage {
            events,
            next_cursor: EventCursor {
                run_id,
                after_sequence: next_sequence,
            },
        })
    }

    pub fn recover(&self) -> Result<RecoveryReport, OrchestrationError> {
        let _transition_guard = self
            .supervision_transition_gate
            .lock()
            .map_err(|_| OrchestrationError::SupervisionStateUnavailable)?;
        let _recovery_lock = self.store.acquire_recovery_lock()?;
        let now_unix_ms = current_unix_ms();
        let mut report = RecoveryReport::default();
        self.reconcile_orphan_worktrees(now_unix_ms, &mut report)?;

        for record in self.store.nonterminated_process_supervisions()? {
            let termination = self
                .supervisor
                .terminate(&record.metadata)
                .map_err(OrchestrationError::Execution)?;
            let reason = match termination.status {
                TerminationStatus::Terminated | TerminationStatus::AlreadyExited => {
                    TerminationReason::Requested
                }
                TerminationStatus::IdentityMismatch => TerminationReason::IdentityMismatch,
                TerminationStatus::Unsupported | TerminationStatus::ProcessesRemain => {
                    return Err(OrchestrationError::ProcessTerminationUnverified);
                }
            };
            let mut terminated = record.metadata;
            terminated.state = SupervisionState::Terminated;
            terminated.termination_reason = Some(reason);
            self.store.mark_process_supervision_terminated(
                record.run_id,
                &terminated,
                now_unix_ms,
            )?;
            report.actions.push(self.record_recovery_action(
                format!("process:{}:terminated", terminated.supervision_id),
                Some(record.run_id),
                self.store.run(record.run_id)?.map(|run| run.changeset_id()),
                RecoveryAction {
                    aggregate_kind: "run".to_owned(),
                    aggregate_id: record.run_id,
                    action: "process_terminated".to_owned(),
                    detail: match termination.status {
                        TerminationStatus::IdentityMismatch => {
                            "persisted process identity no longer matched the live PID; no signal was sent"
                        }
                        TerminationStatus::AlreadyExited => {
                            "persisted process had already exited before startup reconciliation"
                        }
                        TerminationStatus::Terminated => {
                            "surviving provider process tree was terminated and verified"
                        }
                        TerminationStatus::Unsupported | TerminationStatus::ProcessesRemain => {
                            unreachable!("unverified termination returned before recovery action")
                        }
                    }
                    .to_owned(),
                },
                now_unix_ms,
            )?);
        }

        for run in self.store.runs()? {
            match run.state() {
                RunState::Completed | RunState::Interrupted | RunState::Failed => {
                    report.terminal.push(run.id);
                    self.reconcile_active_changeset(&run, now_unix_ms, &mut report)?;
                }
                RunState::Queued
                | RunState::Starting
                | RunState::Running
                | RunState::AwaitingApproval
                | RunState::Reviewable => {
                    self.reconcile_nonterminal_run(run, now_unix_ms, &mut report)?;
                }
            }
        }

        for lease in self.store.mutation_leases()? {
            let owner_terminal = self.store.run(lease.run_id)?.is_none_or(|run| {
                matches!(
                    run.state(),
                    RunState::Completed | RunState::Interrupted | RunState::Failed
                )
            });
            if (owner_terminal || lease.expires_at_unix_ms <= now_unix_ms)
                && self.store.release_mutation_lease(
                    lease.changeset_id,
                    lease.run_id,
                    lease.fencing_epoch,
                )?
            {
                report.actions.push(self.record_recovery_action(
                    format!(
                        "lease:{}:{}:released",
                        lease.changeset_id, lease.fencing_epoch
                    ),
                    Some(lease.run_id),
                    Some(lease.changeset_id),
                    RecoveryAction {
                        aggregate_kind: "changeset".to_owned(),
                        aggregate_id: lease.changeset_id,
                        action: "lease_released".to_owned(),
                        detail: "terminal or expired mutation lease was released using its exact fencing epoch".to_owned(),
                    },
                    now_unix_ms,
                )?);
            }
        }

        Ok(report)
    }

    fn reconcile_orphan_worktrees(
        &self,
        now_unix_ms: i64,
        report: &mut RecoveryReport,
    ) -> Result<(), OrchestrationError> {
        for mut changeset in self.store.changesets()? {
            if changeset.state() != ChangesetState::Created
                || self.store.worktree_for_changeset(changeset.id)?.is_some()
            {
                continue;
            }
            let repository = self.repository(changeset.repository_id())?;
            let Some(worktree) = self
                .git
                .recover_orphan_worktree(&repository, changeset.id)?
            else {
                continue;
            };
            match worktree.state() {
                WorktreeState::Removed => {
                    report.actions.push(self.record_recovery_action(
                        format!("changeset:{}:orphan-worktree-removed", changeset.id),
                        None,
                        Some(changeset.id),
                        RecoveryAction {
                            aggregate_kind: "changeset".to_owned(),
                            aggregate_id: changeset.id,
                            action: "orphan_worktree_removed".to_owned(),
                            detail:
                                "unpersisted clean worktree was verified and removed".to_owned(),
                        },
                        now_unix_ms,
                    )?);
                }
                WorktreeState::Quarantined => {
                    self.store.save_worktree(&worktree)?;
                    changeset.fail()?;
                    self.store.persist_changeset_transition(&changeset)?;
                    report.actions.push(self.record_recovery_action(
                        format!("changeset:{}:orphan-worktree-quarantined", changeset.id),
                        None,
                        Some(changeset.id),
                        RecoveryAction {
                            aggregate_kind: "changeset".to_owned(),
                            aggregate_id: changeset.id,
                            action: "orphan_worktree_quarantined".to_owned(),
                            detail: "unpersisted worktree could not be verified as clean and was quarantined".to_owned(),
                        },
                        now_unix_ms,
                    )?);
                }
                WorktreeState::Creating
                | WorktreeState::Ready
                | WorktreeState::Removing
                | WorktreeState::Failed => {
                    unreachable!("orphan recovery returns only removed or quarantined worktrees")
                }
            }
        }
        Ok(())
    }

    fn reconcile_nonterminal_run(
        &self,
        mut run: Run,
        now_unix_ms: i64,
        report: &mut RecoveryReport,
    ) -> Result<(), OrchestrationError> {
        let changeset_id = run.changeset_id();
        let original_state = run.state();
        let approval_detail = self.recovery_approval_detail(&run, now_unix_ms)?;

        run.interrupt()?;
        self.store.persist_run_transition(
            &run,
            SemanticEventKind::Lifecycle {
                state: RunState::Interrupted,
            },
        )?;
        report.interrupted.push(run.id);
        report.actions.push(self.record_recovery_action(
            format!("run:{}:interrupted", run.id),
            Some(run.id),
            Some(changeset_id),
            RecoveryAction {
                aggregate_kind: "run".to_owned(),
                aggregate_id: run.id,
                action: "interrupted".to_owned(),
                detail: format!(
                    "startup reconciled nonterminal state {original_state:?}; {approval_detail}"
                ),
            },
            now_unix_ms,
        )?);

        self.reconcile_active_changeset(&run, now_unix_ms, report)
    }

    fn reconcile_active_changeset(
        &self,
        run: &Run,
        now_unix_ms: i64,
        report: &mut RecoveryReport,
    ) -> Result<(), OrchestrationError> {
        let mut changeset = self.changeset(run.changeset_id())?;
        if changeset.state() != ChangesetState::Active {
            return Ok(());
        }
        let repository = self.repository(changeset.repository_id())?;
        let mut worktree = self.worktree(changeset.id)?;
        let reconciliation = self.git.reconciliation_state(&repository, &worktree);
        let divergence = match reconciliation {
            Ok(state) if state.repository_head_sha != repository.base_sha => Some((
                state.worktree_head_sha,
                "registered repository HEAD changed",
            )),
            Ok(state) if state.worktree_head_sha != changeset.head_sha() => {
                Some((state.worktree_head_sha, "worktree HEAD changed"))
            }
            Ok(state) if state.worktree_dirty => Some((
                state.worktree_head_sha,
                "worktree contains unapproved changes",
            )),
            Ok(state) => {
                self.git.cleanup_worktree(&repository, &mut worktree)?;
                self.store.update_worktree(&worktree)?;
                changeset.mark_recoverable(state.worktree_head_sha)?;
                self.store.persist_changeset_transition(&changeset)?;
                report.recoverable.push(run.id);
                report.actions.push(self.record_recovery_action(
                    format!("changeset:{}:recoverable", changeset.id),
                    Some(run.id),
                    Some(changeset.id),
                    RecoveryAction {
                        aggregate_kind: "changeset".to_owned(),
                        aggregate_id: changeset.id,
                        action: "worktree_cleaned".to_owned(),
                        detail: "verified clean worktree was removed and the changeset was made recoverable".to_owned(),
                    },
                    now_unix_ms,
                )?);
                None
            }
            Err(_) => Some((
                changeset.head_sha().to_owned(),
                "repository or worktree identity could not be verified",
            )),
        };

        if let Some((head_sha, detail)) = divergence {
            if matches!(
                worktree.state(),
                WorktreeState::Ready | WorktreeState::Failed
            ) {
                worktree.quarantine()?;
                self.store.update_worktree(&worktree)?;
            }
            changeset.mark_divergent(head_sha)?;
            self.store.persist_changeset_transition(&changeset)?;
            report.actions.push(self.record_recovery_action(
                format!("changeset:{}:divergent", changeset.id),
                Some(run.id),
                Some(changeset.id),
                RecoveryAction {
                    aggregate_kind: "changeset".to_owned(),
                    aggregate_id: changeset.id,
                    action: "quarantined".to_owned(),
                    detail: detail.to_owned(),
                },
                now_unix_ms,
            )?);
        }
        Ok(())
    }

    fn recovery_approval_detail(
        &self,
        run: &Run,
        now_unix_ms: i64,
    ) -> Result<&'static str, OrchestrationError> {
        if run.state() != RunState::AwaitingApproval {
            return Ok("no approval was pending");
        }
        let Some(approval) = self.store.approval_for_run(run.id)? else {
            return Ok("pending approval record was missing");
        };
        if approval.is_consumed() {
            return Ok("pending approval had already been consumed");
        }
        if approval.scope().expires_at_unix_ms <= now_unix_ms {
            return Ok("pending approval had expired");
        }
        Ok("pending approval was invalidated by restart")
    }

    fn record_recovery_action(
        &self,
        idempotency_key: String,
        run_id: Option<Id>,
        changeset_id: Option<Id>,
        action: RecoveryAction,
        now_unix_ms: i64,
    ) -> Result<RecoveryAction, OrchestrationError> {
        Ok(self
            .store
            .record_recovery_action(&idempotency_key, run_id, changeset_id, action, now_unix_ms)?
            .action)
    }

    fn prepare_approval(
        &self,
        repository: &Repository,
        changeset: &Changeset,
        worktree: &Worktree,
        run: &mut Run,
    ) -> Result<RunStarted, OrchestrationError> {
        self.store.persist_run_transition(
            run,
            SemanticEventKind::Lifecycle {
                state: RunState::Starting,
            },
        )?;
        run.running()?;
        self.store.persist_run_transition(
            run,
            SemanticEventKind::Lifecycle {
                state: RunState::Running,
            },
        )?;

        self.execute_provider_process(run.id, worktree)?;
        if self.run(run.id)?.state() == RunState::Interrupted {
            return Err(OrchestrationError::RunInterruptedDuringExecution);
        }

        let proposed_change = self.provider.propose();
        if proposed_change.content.len() > domain::limits::MAX_APPROVED_FILE_BYTES {
            return Err(OrchestrationError::ResourceLimit("approved file content"));
        }
        if git::content_digest(&proposed_change.content) != proposed_change.proposal.content_sha256
        {
            return Err(OrchestrationError::ProviderContract(
                "provider emitted content that does not match its proposal",
            ));
        }
        self.store.save_proposed_change(
            run.id,
            &proposed_change.proposal,
            &proposed_change.content,
        )?;

        let expires_at_unix_ms = current_unix_ms()
            .saturating_add(self.approval_ttl.as_millis().try_into().unwrap_or(i64::MAX));
        let scope = ApprovalScope {
            repository_id: repository.id,
            changeset_id: changeset.id,
            base_sha: repository.base_sha.clone(),
            head_sha: self.git.head_sha(worktree)?,
            proposal: proposed_change.proposal.clone(),
            expires_at_unix_ms,
        };
        let approval = Approval::new(run.id, scope.clone());
        let approval_digest = approval.digest().to_owned();
        let mut proposal_persisted = false;

        for event in self
            .provider
            .normalized_events(&proposed_change, &approval_digest)
        {
            match &event {
                SemanticEventKind::ActionProposal { proposal, digest }
                    if proposal == &proposed_change.proposal && digest == &approval_digest =>
                {
                    run.await_approval(approval_digest.clone())?;
                    self.store.request_approval(run, &approval, event)?;
                    proposal_persisted = true;
                }
                SemanticEventKind::ActionProposal { .. } => {
                    return Err(OrchestrationError::ProviderContract(
                        "provider emitted a mismatched action proposal",
                    ));
                }
                _ if !proposal_persisted => {
                    self.store.persist_run_transition(run, event)?;
                }
                _ => {
                    return Err(OrchestrationError::ProviderContract(
                        "provider emitted events after the approval proposal",
                    ));
                }
            }
        }

        if !proposal_persisted {
            return Err(OrchestrationError::ProviderContract(
                "provider did not emit an action proposal",
            ));
        }

        Ok(RunStarted {
            run_id: run.id,
            changeset_id: changeset.id,
            worktree_id: worktree.id,
            approval_id: approval.id,
            approval_request: ApprovalRequest {
                run_id: run.id,
                scope,
                digest: approval_digest,
            },
        })
    }

    fn execute_provider_process(
        &self,
        run_id: Id,
        worktree: &Worktree,
    ) -> Result<(), OrchestrationError> {
        let Some(mut spec) = self.provider.process_spec(&worktree.path) else {
            return Ok(());
        };
        self.bind_process_to_worktree(&mut spec, worktree)?;
        let preparation_guard = self
            .supervision_transition_gate
            .lock()
            .map_err(|_| OrchestrationError::SupervisionStateUnavailable)?;

        let (prepared, prepared_metadata) = self
            .supervisor
            .prepare(&spec)
            .map_err(OrchestrationError::Execution)?;
        self.store.insert_prepared_process_supervision(
            run_id,
            &prepared_metadata,
            current_unix_ms(),
        )?;

        let running = match prepared.release() {
            Ok(running) => running,
            Err(error) => {
                let mut terminated = prepared_metadata;
                terminated.state = SupervisionState::Terminated;
                terminated.termination_reason = Some(TerminationReason::LaunchFailed);
                self.store.mark_process_supervision_terminated(
                    run_id,
                    &terminated,
                    current_unix_ms(),
                )?;
                return Err(OrchestrationError::Execution(error));
            }
        };
        let running_metadata = running.metadata().clone();
        if let Err(error) = self.store.mark_process_supervision_running(
            run_id,
            &running_metadata,
            current_unix_ms(),
        ) {
            drop(running);
            let mut terminated = prepared_metadata;
            terminated.state = SupervisionState::Terminated;
            terminated.termination_reason = Some(TerminationReason::LaunchFailed);
            let _ = self.store.mark_process_supervision_terminated(
                run_id,
                &terminated,
                current_unix_ms(),
            );
            return Err(error.into());
        }
        drop(preparation_guard);

        let supervised = match self.supervisor.wait(running, &CancellationToken::default()) {
            Ok(supervised) => supervised,
            Err(error) => {
                let mut terminated = running_metadata;
                terminated.state = SupervisionState::Terminated;
                terminated.termination_reason = Some(TerminationReason::LaunchFailed);
                let _transition_guard = self
                    .supervision_transition_gate
                    .lock()
                    .map_err(|_| OrchestrationError::SupervisionStateUnavailable)?;
                let latest = self
                    .store
                    .latest_process_supervision_for_run(run_id)?
                    .ok_or(OrchestrationError::NotFound("process supervision record"))?;
                if latest.metadata.state == SupervisionState::Running {
                    self.store.mark_process_supervision_terminated(
                        run_id,
                        &terminated,
                        current_unix_ms(),
                    )?;
                }
                return Err(OrchestrationError::Execution(error));
            }
        };

        let _transition_guard = self
            .supervision_transition_gate
            .lock()
            .map_err(|_| OrchestrationError::SupervisionStateUnavailable)?;
        let latest = self
            .store
            .latest_process_supervision_for_run(run_id)?
            .ok_or(OrchestrationError::NotFound("process supervision record"))?;
        if latest.metadata.supervision_id != supervised.metadata.supervision_id {
            return Err(OrchestrationError::ProcessSupervisionMismatch);
        }
        if latest.metadata.state != SupervisionState::Terminated {
            self.store.mark_process_supervision_terminated(
                run_id,
                &supervised.metadata,
                current_unix_ms(),
            )?;
        }

        if self.run(run_id)?.state() == RunState::Interrupted {
            return Err(OrchestrationError::RunInterruptedDuringExecution);
        }
        if supervised.result.outcome != TerminalOutcome::Completed(0) {
            return Err(OrchestrationError::ProviderProcessFailed(
                supervised.result.outcome,
            ));
        }
        Ok(())
    }

    fn bind_process_to_worktree(
        &self,
        spec: &mut ProcessSpec,
        worktree: &Worktree,
    ) -> Result<(), OrchestrationError> {
        match &spec.current_dir {
            Some(current_dir) if current_dir != &worktree.path => {
                Err(OrchestrationError::ProviderProcessOutsideWorktree)
            }
            Some(_) => Ok(()),
            None => {
                spec.current_dir = Some(worktree.path.clone());
                Ok(())
            }
        }
    }

    fn compensate_failed_start(
        &self,
        changeset: &mut Changeset,
        repository: &Repository,
        worktree: &mut Worktree,
        run_id: Id,
    ) -> Result<(), OrchestrationError> {
        let fail_run = self.fail_active_run(run_id);
        let recover_changeset = self.mark_changeset_recoverable(changeset.id);
        let release_lease = self.release_run_lease(changeset.id, run_id);
        let cleanup_worktree = self.cleanup_setup_worktree(repository, worktree);

        fail_run?;
        recover_changeset?;
        release_lease?;
        cleanup_worktree
    }

    fn cleanup_setup_worktree(
        &self,
        repository: &Repository,
        worktree: &mut Worktree,
    ) -> Result<(), OrchestrationError> {
        self.git.cleanup_worktree(repository, worktree)?;
        self.store.update_worktree(worktree)?;
        Ok(())
    }

    fn mark_changeset_recoverable(&self, changeset_id: Id) -> Result<(), OrchestrationError> {
        let mut changeset = self.changeset(changeset_id)?;
        if changeset.state() == domain::ChangesetState::Recoverable {
            return Ok(());
        }
        if changeset.state() != domain::ChangesetState::Active {
            return Err(OrchestrationError::ChangesetRecoveryUnavailable);
        }
        changeset.mark_recoverable(changeset.head_sha().to_owned())?;
        self.store.persist_changeset_transition(&changeset)?;
        Ok(())
    }

    fn release_run_lease(&self, changeset_id: Id, run_id: Id) -> Result<(), OrchestrationError> {
        let Some(lease) = self.store.mutation_lease(changeset_id)? else {
            return Ok(());
        };
        if lease.run_id != run_id {
            return Err(OrchestrationError::MutationLeaseUnavailable);
        }
        self.release_exact_lease(&lease)
    }

    fn release_exact_lease(&self, lease: &MutationLease) -> Result<(), OrchestrationError> {
        if !self.store.release_mutation_lease(
            lease.changeset_id,
            lease.run_id,
            lease.fencing_epoch,
        )? {
            return Err(OrchestrationError::MutationLeaseUnavailable);
        }
        Ok(())
    }

    fn fail_active_run(&self, run_id: Id) -> Result<(), OrchestrationError> {
        let mut run = self.run(run_id)?;
        if matches!(
            run.state(),
            RunState::Starting | RunState::Running | RunState::AwaitingApproval
        ) {
            run.fail()?;
            self.store.persist_run_transition(
                &run,
                SemanticEventKind::Lifecycle {
                    state: RunState::Failed,
                },
            )?;
        }
        Ok(())
    }

    fn repository(&self, repository_id: Id) -> Result<Repository, OrchestrationError> {
        self.store
            .repository(repository_id)?
            .ok_or(OrchestrationError::NotFound("repository"))
    }

    fn changeset(&self, changeset_id: Id) -> Result<Changeset, OrchestrationError> {
        self.store
            .changeset(changeset_id)?
            .ok_or(OrchestrationError::NotFound("changeset"))
    }

    fn run(&self, run_id: Id) -> Result<Run, OrchestrationError> {
        self.store
            .run(run_id)?
            .ok_or(OrchestrationError::NotFound("run"))
    }

    fn worktree(&self, changeset_id: Id) -> Result<Worktree, OrchestrationError> {
        self.store
            .worktree_for_changeset(changeset_id)?
            .ok_or(OrchestrationError::NotFound("worktree"))
    }

    fn approval(&self, run_id: Id) -> Result<Approval, OrchestrationError> {
        self.store
            .approval_for_run(run_id)?
            .ok_or(OrchestrationError::NotFound("approval"))
    }
}

pub type RunStarted = RunStartedResponse;
pub type RunCompleted = RunCompletedResponse;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutcome {
    RepositoryRegistered(Repository),
    ChangesetCreated(Changeset),
    RunStarted(RunStarted),
    RunInterrupted(Run),
    ApprovalRejected(Run),
    RunCompleted(RunCompleted),
    Checkpoint(CheckpointResponse),
    Diff(DiffResponse),
    Recovery(RecoveryResponse),
    Findings(FindingsResponse),
}

fn current_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[derive(Debug, Error)]
pub enum OrchestrationError {
    #[error("{0} was not found")]
    NotFound(&'static str),
    #[error("repository base SHA changed before changeset creation")]
    StaleBase,
    #[error("approval response does not match the pending request")]
    ApprovalMismatch,
    #[error("command is defined by the protocol but is not implemented by this runtime phase")]
    UnsupportedCommand,
    #[error("provider contract violation: {0}")]
    ProviderContract(&'static str),
    #[error("resource limit exceeded for {0}")]
    ResourceLimit(&'static str),
    #[error("provider process execution failed")]
    Execution(#[source] execution::ExecutionError),
    #[error("provider process exited without a successful status: {0:?}")]
    ProviderProcessFailed(TerminalOutcome),
    #[error("provider process current directory does not match the registered worktree")]
    ProviderProcessOutsideWorktree,
    #[error("provider process supervision identity did not match the persisted record")]
    ProcessSupervisionMismatch,
    #[error("provider process termination could not be verified")]
    ProcessTerminationUnverified,
    #[error("run was interrupted while provider execution was active")]
    RunInterruptedDuringExecution,
    #[error("process supervision state is unavailable")]
    SupervisionStateUnavailable,
    #[error("the run's mutation lease is missing or owned by another fencing epoch")]
    MutationLeaseUnavailable,
    #[error("active changeset could not be moved to recoverable state")]
    ChangesetRecoveryUnavailable,
    #[error("domain operation failed: {0}")]
    Domain(#[from] domain::DomainError),
    #[error("persistence operation failed: {0}")]
    Persistence(#[from] persistence::PersistenceError),
    #[error("Git operation failed: {0}")]
    Git(#[from] git::GitError),
}
