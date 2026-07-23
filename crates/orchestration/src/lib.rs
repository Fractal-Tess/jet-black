use agents::AgentProvider;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use domain::{
    Approval, ApprovalScope, Changeset, ChangesetMutationKind, ChangesetMutationScope,
    ChangesetState, Checkpoint, Id, Repository, Run, RunState, TicketRef, Worktree, WorktreeState,
};
use execution::{
    CancellationToken, ProcessConfinement, ProcessResult, ProcessSpec, ProcessSupervisor,
    SupervisionState, TerminalOutcome, TerminationReason, TerminationStatus,
};
use git::GitService;
use persistence::{
    ArtifactSegment, ArtifactStream, ChangesetFinalization, ChangesetFinalizationResult,
    ChangesetFinalizationState, CompletedChangesetFinalization, LocalArtifactStore, MutationLease,
    RecoveryReport, RunArtifact, SqliteStore, VerifiedArtifactSegment,
};
use protocol::{
    ApprovalRequest, CheckpointResponse, DiffResponse, EventCursor, EventPage, FindingsResponse,
    HistoryResponse, LocalCommand, MutationPreview, MutationResult, OrderedRunEvent,
    RecoveryAction, RecoveryResponse, ReviewCheckKind, ReviewReport, RunArtifactSegmentMetadata,
    RunArtifactSegmentResponse, RunArtifactStream, RunArtifactSummary, RunArtifactsDeletedResponse,
    RunArtifactsResponse, RunCompletedResponse, RunSnapshot, RunStartedResponse, SemanticEventKind,
};
use review::{ReviewOptions, ReviewService};
use std::{fs, path::Path, sync::Mutex, time::Duration};
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
    artifact_store: Option<LocalArtifactStore>,
    review_service: Option<ReviewService>,
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
            artifact_store: None,
            review_service: None,
            supervision_transition_gate: Mutex::new(()),
        }
    }

    pub fn with_artifact_store(mut self, artifact_store: LocalArtifactStore) -> Self {
        self.artifact_store = Some(artifact_store);
        self
    }

    pub fn with_review_options(
        mut self,
        options: ReviewOptions,
        search_path: &str,
    ) -> Result<Self, OrchestrationError> {
        let review_service = if options.allow_unsandboxed_checks {
            ReviewService::with_search_path(
                self.store.clone(),
                self.git.clone(),
                options,
                search_path,
            )
            .map_err(map_review_error)?
        } else {
            ReviewService::without_executable_checks(self.store.clone(), self.git.clone(), options)
                .map_err(map_review_error)?
        };
        self.review_service = Some(review_service);
        Ok(self)
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
                Ok(CommandOutcome::RunStarted(self.begin_run(changeset_id)?))
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
            LocalCommand::ReviewChangeset {
                changeset_id,
                expected_version,
                expected_head_sha,
                checks,
            } => Ok(CommandOutcome::ReviewCompleted(self.review_changeset(
                changeset_id,
                expected_version,
                &expected_head_sha,
                &checks,
            )?)),
            LocalCommand::GetRunArtifacts { run_id } => {
                Ok(CommandOutcome::RunArtifacts(self.run_artifacts(run_id)?))
            }
            LocalCommand::ReadRunArtifactSegment {
                run_id,
                artifact_id,
                segment_sequence,
            } => Ok(CommandOutcome::RunArtifactSegment(
                self.read_run_artifact_segment(run_id, artifact_id, segment_sequence)?,
            )),
            LocalCommand::DeleteRunArtifacts { run_id } => Ok(CommandOutcome::RunArtifactsDeleted(
                self.delete_run_artifacts(run_id)?,
            )),
            LocalCommand::GetEvents {
                run_id,
                after_sequence,
                limit,
            } => Ok(CommandOutcome::Events(
                self.events_after(
                    run_id,
                    after_sequence,
                    usize::try_from(limit)
                        .map_err(|_| OrchestrationError::ResourceLimit("event page size"))?,
                )?,
            )),
            LocalCommand::GetSnapshot { run_id } => {
                Ok(CommandOutcome::Snapshot(Box::new(self.snapshot(run_id)?)))
            }
            LocalCommand::GetHistory {
                changeset_id,
                limit,
            } => Ok(CommandOutcome::History(
                self.history(
                    changeset_id,
                    usize::try_from(limit)
                        .map_err(|_| OrchestrationError::ResourceLimit("run history page size"))?,
                )?,
            )),
            LocalCommand::PreviewCommit {
                changeset_id,
                expected_version,
                expected_head_sha,
            } => Ok(CommandOutcome::MutationPreview(
                self.preview_changeset_mutation(
                    ChangesetMutationKind::Commit,
                    changeset_id,
                    expected_version,
                    &expected_head_sha,
                )?,
            )),
            LocalCommand::CommitChangeset {
                changeset_id,
                expected_version,
                expected_head_sha,
                confirmation_digest,
            } => Ok(CommandOutcome::MutationCompleted(self.finalize_changeset(
                ChangesetMutationKind::Commit,
                changeset_id,
                expected_version,
                &expected_head_sha,
                &confirmation_digest,
            )?)),
            LocalCommand::PreviewDiscard {
                changeset_id,
                expected_version,
                expected_head_sha,
            } => Ok(CommandOutcome::MutationPreview(
                self.preview_changeset_mutation(
                    ChangesetMutationKind::Discard,
                    changeset_id,
                    expected_version,
                    &expected_head_sha,
                )?,
            )),
            LocalCommand::DiscardChangeset {
                changeset_id,
                expected_version,
                expected_head_sha,
                confirmation_digest,
            } => Ok(CommandOutcome::MutationCompleted(self.finalize_changeset(
                ChangesetMutationKind::Discard,
                changeset_id,
                expected_version,
                &expected_head_sha,
                &confirmation_digest,
            )?)),
        }
    }

    pub fn review_changeset(
        &self,
        changeset_id: Id,
        expected_version: u64,
        expected_head_sha: &str,
        requested_checks: &[ReviewCheckKind],
    ) -> Result<ReviewReport, OrchestrationError> {
        let service = self
            .review_service
            .as_ref()
            .ok_or(OrchestrationError::ReviewServiceUnavailable)?;
        let changeset = self.exact_changeset(changeset_id, expected_version, expected_head_sha)?;
        let repository = self.repository(changeset.repository_id())?;
        let worktree = self.worktree(changeset.id)?;
        service
            .review(&repository, &changeset, &worktree, requested_checks)
            .map_err(map_review_error)
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
        let started = self.begin_run(changeset_id)?;
        self.drive_run_to_approval(started.run_id)
    }

    pub fn begin_run(&self, changeset_id: Id) -> Result<RunStarted, OrchestrationError> {
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
        if let Err(error) = self.store.persist_run_transition(
            &run,
            SemanticEventKind::Lifecycle {
                state: RunState::Starting,
            },
        ) {
            self.compensate_failed_start(&mut changeset, &repository, &mut worktree, run.id)?;
            return Err(error.into());
        }

        Ok(RunStarted {
            run_id: run.id,
            changeset_id: changeset.id,
            worktree_id: worktree.id,
            approval_id: None,
            approval_request: None,
        })
    }

    pub fn drive_run_to_approval(&self, run_id: Id) -> Result<RunStarted, OrchestrationError> {
        let mut run = self.run(run_id)?;
        if run.state() != RunState::Starting {
            return Err(OrchestrationError::RunDriveUnavailable);
        }
        let mut changeset = self.changeset(run.changeset_id())?;
        let repository = self.repository(changeset.repository_id())?;
        let mut worktree = self.worktree(changeset.id)?;
        if worktree.changeset_id() != changeset.id || worktree.state() != WorktreeState::Ready {
            return Err(OrchestrationError::RunDriveUnavailable);
        }
        self.store.acquire_mutation_lease(
            changeset.id,
            run.id,
            current_unix_ms(),
            self.mutation_lease_ttl,
        )?;

        let drive_claim_guard = self
            .supervision_transition_gate
            .lock()
            .map_err(|_| OrchestrationError::SupervisionStateUnavailable)?;
        run.running()?;
        match self.store.claim_run_for_drive(&run) {
            Ok(_) => {}
            Err(
                persistence::PersistenceError::RunDriveAlreadyClaimed
                | persistence::PersistenceError::VersionConflict { .. },
            ) => return Err(OrchestrationError::RunDriveUnavailable),
            Err(error) => return Err(error.into()),
        }
        drop(drive_claim_guard);

        match self.drive_claimed_run_to_approval(&repository, &changeset, &worktree, &mut run) {
            Ok(started) => Ok(started),
            Err(error) => {
                let interrupted = matches!(
                    self.store.run(run.id),
                    Ok(Some(current)) if current.state() == RunState::Interrupted
                );
                self.compensate_failed_start(&mut changeset, &repository, &mut worktree, run.id)?;
                if interrupted {
                    Err(OrchestrationError::RunInterruptedDuringExecution)
                } else {
                    Err(error)
                }
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
        let transition_guard = self
            .supervision_transition_gate
            .lock()
            .map_err(|_| OrchestrationError::SupervisionStateUnavailable)?;
        let mut run = self.run(run_id)?;
        let cleanup_pre_drive_worktree = if run.state() == RunState::Interrupted {
            !self.run_provider_drive_started(run.id)?
        } else {
            let interrupted_before_drive = run.state() == RunState::Starting;
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
            interrupted_before_drive
        };
        drop(transition_guard);

        let delete_proposal = self.store.delete_proposed_change_for_run(run.id);
        let recover_changeset = self.mark_changeset_recoverable(run.changeset_id());
        let release_lease = self.release_run_lease(run.changeset_id(), run.id);
        delete_proposal?;
        recover_changeset?;
        release_lease?;
        if cleanup_pre_drive_worktree {
            self.cleanup_interrupted_pre_drive_worktree(run.changeset_id())?;
        }
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

    pub fn preview_changeset_mutation(
        &self,
        kind: ChangesetMutationKind,
        changeset_id: Id,
        expected_version: u64,
        expected_head_sha: &str,
    ) -> Result<MutationPreview, OrchestrationError> {
        self.build_mutation_preview(kind, changeset_id, expected_version, expected_head_sha)
            .map(|(preview, _)| preview)
    }

    pub fn finalize_changeset(
        &self,
        kind: ChangesetMutationKind,
        changeset_id: Id,
        expected_version: u64,
        expected_head_sha: &str,
        confirmation_digest: &str,
    ) -> Result<MutationResult, OrchestrationError> {
        let _recovery_lock = self.store.acquire_recovery_lock()?;

        if let Some(finalization) = self.store.changeset_finalization(confirmation_digest)? {
            self.validate_finalization_command(
                &finalization,
                kind,
                changeset_id,
                expected_version,
                expected_head_sha,
            )?;
            return match finalization.state {
                ChangesetFinalizationState::Prepared => {
                    self.execute_prepared_finalization(&finalization, true)
                }
                ChangesetFinalizationState::Completed => {
                    self.completed_mutation_result(&finalization)
                }
                ChangesetFinalizationState::Divergent => {
                    Err(OrchestrationError::ChangesetFinalizationDivergent)
                }
            };
        }

        let (preview, scope) =
            self.build_mutation_preview(kind, changeset_id, expected_version, expected_head_sha)?;
        if preview.confirmation_digest != confirmation_digest {
            return Err(OrchestrationError::MutationConfirmationMismatch);
        }
        let worktree = self.worktree(changeset_id)?;
        let finalization = self
            .store
            .prepare_changeset_finalization(
                confirmation_digest,
                &scope,
                worktree.id,
                worktree.version(),
                current_unix_ms(),
            )
            .map_err(map_finalization_persistence_error)?;
        self.execute_prepared_finalization(&finalization, false)
    }

    fn build_mutation_preview(
        &self,
        kind: ChangesetMutationKind,
        changeset_id: Id,
        expected_version: u64,
        expected_head_sha: &str,
    ) -> Result<(MutationPreview, ChangesetMutationScope), OrchestrationError> {
        let changeset = self.exact_changeset(changeset_id, expected_version, expected_head_sha)?;
        if changeset.state() != ChangesetState::Reviewable {
            return Err(OrchestrationError::ChangesetNotReviewable);
        }
        let repository = self.repository(changeset.repository_id())?;
        if repository.base_sha != changeset.base_sha() {
            return Err(OrchestrationError::StaleBase);
        }
        let worktree = self.worktree(changeset.id)?;
        if worktree.state() != WorktreeState::Ready {
            return Err(OrchestrationError::ChangesetNotReviewable);
        }
        let checkpoint = self
            .store
            .checkpoint_for_changeset(changeset.id)?
            .ok_or(OrchestrationError::NotFound("checkpoint"))?;
        if checkpoint.base_sha != changeset.base_sha() || checkpoint.head_sha != expected_head_sha {
            return Err(OrchestrationError::MutationPreviewMismatch);
        }
        let manifest = self
            .git
            .mutation_manifest(&repository, &worktree)
            .map_err(map_mutation_git_error)?;
        if manifest.head_sha != expected_head_sha {
            return Err(OrchestrationError::StaleChangesetHead);
        }
        let scope = ChangesetMutationScope {
            kind,
            repository_id: repository.id,
            changeset_id,
            checkpoint_id: checkpoint.id,
            expected_version,
            base_sha: changeset.base_sha().to_owned(),
            expected_head_sha: expected_head_sha.to_owned(),
            manifest_sha256: manifest.sha256,
        };
        let preview = MutationPreview {
            kind,
            changeset_id,
            checkpoint_id: checkpoint.id,
            expected_version,
            expected_head_sha: expected_head_sha.to_owned(),
            manifest_sha256: scope.manifest_sha256.clone(),
            confirmation_digest: scope.digest(),
        };
        Ok((preview, scope))
    }

    fn validate_finalization_command(
        &self,
        finalization: &ChangesetFinalization,
        kind: ChangesetMutationKind,
        changeset_id: Id,
        expected_version: u64,
        expected_head_sha: &str,
    ) -> Result<(), OrchestrationError> {
        if finalization.scope.kind != kind
            || finalization.scope.changeset_id != changeset_id
            || finalization.scope.expected_version != expected_version
            || finalization.scope.expected_head_sha != expected_head_sha
        {
            return Err(OrchestrationError::MutationConfirmationMismatch);
        }
        Ok(())
    }

    fn execute_prepared_finalization(
        &self,
        finalization: &ChangesetFinalization,
        recover_existing_commit: bool,
    ) -> Result<MutationResult, OrchestrationError> {
        let repository = self.repository(finalization.scope.repository_id)?;
        let mut worktree = self
            .store
            .worktree(finalization.worktree_id)?
            .ok_or(OrchestrationError::NotFound("worktree"))?;
        let result = match finalization.scope.kind {
            ChangesetMutationKind::Commit => {
                let recovered = recover_existing_commit
                    .then(|| {
                        self.git.recover_exact_commit_result(
                            &repository,
                            finalization.scope.changeset_id,
                            &finalization.scope.expected_head_sha,
                            &finalization.scope.manifest_sha256,
                        )
                    })
                    .transpose()
                    .map_err(map_mutation_git_error)?
                    .flatten();
                let exact = match recovered {
                    Some(exact) => exact,
                    None => self
                        .git
                        .commit_exact(
                            &repository,
                            &worktree,
                            &finalization.scope.expected_head_sha,
                            &finalization.scope.manifest_sha256,
                        )
                        .map_err(map_mutation_git_error)?,
                };
                self.git.cleanup_worktree(&repository, &mut worktree)?;
                ChangesetFinalizationResult::Commit {
                    resulting_head_sha: exact.resulting_head_sha,
                    app_ref: exact.app_ref,
                }
            }
            ChangesetMutationKind::Discard => {
                match self.git.discard_exact(
                    &repository,
                    &mut worktree,
                    &finalization.scope.expected_head_sha,
                    &finalization.scope.manifest_sha256,
                ) {
                    Ok(()) => {}
                    Err(git::GitError::Io(error))
                        if error.kind() == std::io::ErrorKind::NotFound =>
                    {
                        self.git.cleanup_worktree(&repository, &mut worktree)?;
                    }
                    Err(error) => return Err(map_mutation_git_error(error)),
                }
                ChangesetFinalizationResult::Discard
            }
        };
        let completed = self
            .store
            .complete_changeset_finalization(
                &finalization.confirmation_digest,
                result,
                current_unix_ms(),
            )
            .map_err(map_finalization_persistence_error)?;
        mutation_result(&completed)
    }

    fn completed_mutation_result(
        &self,
        finalization: &ChangesetFinalization,
    ) -> Result<MutationResult, OrchestrationError> {
        let completed = self
            .store
            .completed_changeset_finalization(&finalization.confirmation_digest)
            .map_err(map_finalization_persistence_error)?;
        mutation_result(&completed)
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
        if limit == 0 || limit > domain::limits::MAX_EVENT_PAGE_SIZE {
            return Err(OrchestrationError::ResourceLimit("event page size"));
        }
        self.run(run_id)?;
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

    pub fn snapshot(&self, run_id: Id) -> Result<RunSnapshot, OrchestrationError> {
        let snapshot = self
            .store
            .run_snapshot(run_id)?
            .ok_or(OrchestrationError::NotFound("run"))?;
        Ok(RunSnapshot {
            repository: snapshot.repository,
            changeset: snapshot.changeset,
            run: snapshot.run,
            worktree: snapshot.worktree,
            checkpoint: snapshot.checkpoint,
            pending_approval: snapshot
                .pending_approval
                .as_ref()
                .map(ApprovalRequest::from),
            findings: snapshot.findings,
            events: snapshot.events,
        })
    }

    pub fn history(
        &self,
        changeset_id: Id,
        limit: usize,
    ) -> Result<HistoryResponse, OrchestrationError> {
        if limit == 0 || limit > domain::limits::MAX_RUN_HISTORY_PAGE_SIZE {
            return Err(OrchestrationError::ResourceLimit("run history page size"));
        }
        self.changeset(changeset_id)?;
        Ok(HistoryResponse {
            changeset_id,
            runs: self.store.runs_for_changeset(changeset_id, limit)?,
        })
    }

    pub fn run_artifacts(&self, run_id: Id) -> Result<RunArtifactsResponse, OrchestrationError> {
        self.run(run_id)?;
        let artifacts = self
            .artifact_store()?
            .complete_artifacts_for_run(run_id)
            .map_err(map_artifact_error)?
            .iter()
            .map(artifact_summary)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(RunArtifactsResponse { run_id, artifacts })
    }

    pub fn read_run_artifact_segment(
        &self,
        run_id: Id,
        artifact_id: Id,
        segment_sequence: u32,
    ) -> Result<RunArtifactSegmentResponse, OrchestrationError> {
        self.run(run_id)?;
        let verified = self
            .artifact_store()?
            .read_verified_segment(run_id, artifact_id, segment_sequence)
            .map_err(map_artifact_error)?;
        artifact_segment_response(verified)
    }

    pub fn delete_run_artifacts(
        &self,
        run_id: Id,
    ) -> Result<RunArtifactsDeletedResponse, OrchestrationError> {
        let run = self.run(run_id)?;
        if !matches!(
            run.state(),
            RunState::Completed | RunState::Interrupted | RunState::Failed
        ) {
            return Err(OrchestrationError::ArtifactDeletionRequiresTerminalRun);
        }
        let deleted_count = self
            .artifact_store()?
            .delete_run_artifacts(run_id)
            .map_err(map_artifact_error)?;
        Ok(RunArtifactsDeletedResponse {
            run_id,
            deleted_count: u32::try_from(deleted_count)
                .map_err(|_| OrchestrationError::ResourceLimit("artifact count"))?,
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

        self.reconcile_prepared_finalizations(now_unix_ms, &mut report)?;

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

        if let Some(artifact_store) = &self.artifact_store {
            artifact_store.recover_incomplete()?;
            artifact_store.prune_expired(now_unix_ms)?;
        }

        Ok(report)
    }

    fn observed_finalization_head(
        &self,
        finalization: &ChangesetFinalization,
    ) -> Result<String, OrchestrationError> {
        let repository = self.repository(finalization.scope.repository_id)?;
        let worktree = self
            .store
            .worktree(finalization.worktree_id)?
            .ok_or(OrchestrationError::NotFound("worktree"))?;
        self.git
            .status_snapshot(&repository, &worktree)
            .map(|snapshot| snapshot.head_sha)
            .map_err(OrchestrationError::Git)
    }

    fn reconcile_prepared_finalizations(
        &self,
        now_unix_ms: i64,
        report: &mut RecoveryReport,
    ) -> Result<(), OrchestrationError> {
        for finalization in self.store.prepared_changeset_finalizations()? {
            match self.execute_prepared_finalization(&finalization, true) {
                Ok(_) => {
                    report.actions.push(
                        self.record_recovery_action(
                            format!(
                                "changeset:{}:finalization:{}:completed",
                                finalization.scope.changeset_id, finalization.confirmation_digest
                            ),
                            None,
                            Some(finalization.scope.changeset_id),
                            RecoveryAction {
                                aggregate_kind: "changeset".to_owned(),
                                aggregate_id: finalization.scope.changeset_id,
                                action: "finalization_completed".to_owned(),
                                detail:
                                    "prepared changeset finalization was verified and completed"
                                        .to_owned(),
                            },
                            now_unix_ms,
                        )?,
                    );
                }
                Err(error) if finalization_error_is_divergent(&error) => {
                    let observed_head_sha = self.observed_finalization_head(&finalization)?;
                    let detail = format!(
                        "prepared finalization diverged at worktree HEAD {observed_head_sha}: {error}"
                    );
                    self.store.mark_changeset_finalization_divergent(
                        &finalization.confirmation_digest,
                        observed_head_sha,
                        detail.clone(),
                        now_unix_ms,
                    )?;
                    report.actions.push(self.record_recovery_action(
                        format!(
                            "changeset:{}:finalization:{}:divergent",
                            finalization.scope.changeset_id, finalization.confirmation_digest
                        ),
                        None,
                        Some(finalization.scope.changeset_id),
                        RecoveryAction {
                            aggregate_kind: "changeset".to_owned(),
                            aggregate_id: finalization.scope.changeset_id,
                            action: "finalization_divergent".to_owned(),
                            detail,
                        },
                        now_unix_ms,
                    )?);
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
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

    fn drive_claimed_run_to_approval(
        &self,
        repository: &Repository,
        changeset: &Changeset,
        worktree: &Worktree,
        run: &mut Run,
    ) -> Result<RunStarted, OrchestrationError> {
        let process_result = self.execute_provider_process(run.id, repository, worktree)?;
        if self.run(run.id)?.state() != RunState::Running {
            return Err(OrchestrationError::RunInterruptedDuringExecution);
        }

        let proposed_change = self.provider.propose(process_result.as_ref())?;
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
            run,
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
            approval_id: Some(approval.id),
            approval_request: Some(ApprovalRequest::from(&approval)),
        })
    }

    fn execute_provider_process(
        &self,
        run_id: Id,
        repository: &Repository,
        worktree: &Worktree,
    ) -> Result<Option<ProcessResult>, OrchestrationError> {
        let preparation_guard = self
            .supervision_transition_gate
            .lock()
            .map_err(|_| OrchestrationError::SupervisionStateUnavailable)?;
        if self.run(run_id)?.state() != RunState::Running {
            return Err(OrchestrationError::RunInterruptedDuringExecution);
        }
        let Some(mut spec) = self.provider.process_spec(&worktree.path) else {
            drop(preparation_guard);
            return Ok(None);
        };
        self.bind_process_to_worktree(&mut spec, repository, worktree)?;
        let mut sensitive_values = spec
            .sensitive_environment_keys
            .iter()
            .filter_map(|key| spec.environment.get(key))
            .filter(|value| !value.is_empty())
            .map(|value| value.as_bytes().to_vec())
            .collect::<Vec<_>>();

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

        let transition_guard = self
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
        drop(transition_guard);

        if let Some(artifact_store) = &self.artifact_store {
            sensitive_values.push(supervised.metadata.supervision_token.as_bytes().to_vec());
            let run = self.run(run_id)?;
            artifact_store.capture_process_result(
                run_id,
                run.changeset_id(),
                supervised.metadata.supervision_id,
                &supervised.result,
                &sensitive_values,
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
        if supervised.result.stdout_truncated || supervised.result.stderr_truncated {
            return Err(OrchestrationError::ProviderProcessOutputTruncated);
        }
        Ok(Some(supervised.result))
    }

    fn bind_process_to_worktree(
        &self,
        spec: &mut ProcessSpec,
        repository: &Repository,
        worktree: &Worktree,
    ) -> Result<(), OrchestrationError> {
        match &spec.current_dir {
            Some(current_dir) if current_dir != &worktree.path => {
                return Err(OrchestrationError::ProviderProcessOutsideWorktree);
            }
            Some(_) => {}
            None => spec.current_dir = Some(worktree.path.clone()),
        }

        if !self.provider.requires_process_confinement() {
            return Ok(());
        }
        let ProcessConfinement::LinuxFilesystem(confinement) = &mut spec.confinement else {
            return Err(OrchestrationError::ProviderProcessUnconfined);
        };
        if confinement.workspace != worktree.path {
            return Err(OrchestrationError::ProviderConfinementMismatch);
        }
        let canonical_worktree = fs::canonicalize(&worktree.path)
            .map_err(|_| OrchestrationError::ProviderConfinementMismatch)?;
        let canonical_workspace = fs::canonicalize(&confinement.workspace)
            .map_err(|_| OrchestrationError::ProviderConfinementMismatch)?;
        if canonical_worktree != worktree.path || canonical_workspace != canonical_worktree {
            return Err(OrchestrationError::ProviderConfinementMismatch);
        }
        confinement
            .runtime_read_only
            .extend(self.git.provider_read_only_paths(repository, worktree)?);
        Ok(())
    }

    fn compensate_failed_start(
        &self,
        changeset: &mut Changeset,
        repository: &Repository,
        worktree: &mut Worktree,
        run_id: Id,
    ) -> Result<(), OrchestrationError> {
        let delete_proposal = self.store.delete_proposed_change_for_run(run_id);
        let fail_run = self.fail_active_run(run_id);
        let recover_changeset = self.mark_changeset_recoverable(changeset.id);
        let release_lease = self.release_run_lease(changeset.id, run_id);
        let cleanup_worktree = self.cleanup_setup_worktree(repository, worktree);

        delete_proposal?;
        fail_run?;
        recover_changeset?;
        release_lease?;
        cleanup_worktree
    }

    fn run_provider_drive_started(&self, run_id: Id) -> Result<bool, OrchestrationError> {
        Ok(self.store.events(run_id)?.iter().any(|event| {
            matches!(
                event.event,
                SemanticEventKind::Lifecycle {
                    state: RunState::Running
                }
            )
        }))
    }

    fn cleanup_interrupted_pre_drive_worktree(
        &self,
        changeset_id: Id,
    ) -> Result<(), OrchestrationError> {
        let changeset = self.changeset(changeset_id)?;
        let repository = self.repository(changeset.repository_id())?;
        let mut worktree = self.worktree(changeset.id)?;
        if worktree.state() == WorktreeState::Removed {
            return Ok(());
        }
        self.cleanup_setup_worktree(&repository, &mut worktree)
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

    fn exact_changeset(
        &self,
        changeset_id: Id,
        expected_version: u64,
        expected_head_sha: &str,
    ) -> Result<Changeset, OrchestrationError> {
        let changeset = self.changeset(changeset_id)?;
        if changeset.version() != expected_version {
            return Err(OrchestrationError::StaleChangesetVersion);
        }
        if changeset.head_sha() != expected_head_sha {
            return Err(OrchestrationError::StaleChangesetHead);
        }
        Ok(changeset)
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

    fn artifact_store(&self) -> Result<&LocalArtifactStore, OrchestrationError> {
        self.artifact_store
            .as_ref()
            .ok_or(OrchestrationError::ArtifactStoreUnavailable)
    }
}

pub type RunStarted = RunStartedResponse;
pub type RunCompleted = RunCompletedResponse;

fn artifact_summary(artifact: &RunArtifact) -> Result<RunArtifactSummary, OrchestrationError> {
    Ok(RunArtifactSummary {
        artifact_id: artifact.id,
        changeset_id: artifact.changeset_id,
        run_id: artifact.run_id,
        supervision_id: artifact.supervision_id,
        stream: artifact_stream(artifact.stream),
        source_bytes: artifact_byte_count(artifact.source_bytes)?,
        stored_bytes: artifact_byte_count(artifact.stored_bytes)?,
        segments: artifact
            .segments
            .iter()
            .map(artifact_segment_metadata)
            .collect::<Result<Vec<_>, _>>()?,
        sha256: artifact.sha256.clone(),
        redacted: artifact.redacted,
        process_truncated: artifact.process_truncated,
        quota_limited: artifact.quota_limited,
        created_at_unix_ms: artifact.created_at_unix_ms,
        updated_at_unix_ms: artifact.updated_at_unix_ms,
        expires_at_unix_ms: artifact.expires_at_unix_ms,
    })
}

fn artifact_segment_response(
    verified: VerifiedArtifactSegment,
) -> Result<RunArtifactSegmentResponse, OrchestrationError> {
    Ok(RunArtifactSegmentResponse {
        run_id: verified.artifact.run_id,
        artifact_id: verified.artifact.id,
        stream: artifact_stream(verified.artifact.stream),
        segment: artifact_segment_metadata(&verified.segment)?,
        artifact_sha256: verified.artifact.sha256,
        content_base64: STANDARD.encode(verified.bytes),
    })
}

fn artifact_segment_metadata(
    segment: &ArtifactSegment,
) -> Result<RunArtifactSegmentMetadata, OrchestrationError> {
    Ok(RunArtifactSegmentMetadata {
        sequence: segment.sequence,
        stored_bytes: artifact_byte_count(segment.stored_bytes)?,
        sha256: segment.sha256.clone(),
    })
}

const fn artifact_stream(stream: ArtifactStream) -> RunArtifactStream {
    match stream {
        ArtifactStream::Stdout => RunArtifactStream::Stdout,
        ArtifactStream::Stderr => RunArtifactStream::Stderr,
    }
}

fn artifact_byte_count(value: usize) -> Result<u64, OrchestrationError> {
    u64::try_from(value).map_err(|_| OrchestrationError::ResourceLimit("artifact byte count"))
}

fn map_review_error(error: review::ReviewError) -> OrchestrationError {
    match error {
        review::ReviewError::InvalidReviewState => OrchestrationError::InvalidReviewState,
        review::ReviewError::StaleChangesetVersion => OrchestrationError::StaleChangesetVersion,
        review::ReviewError::HeadChanged => OrchestrationError::StaleChangesetHead,
        review::ReviewError::WorktreeChanged => OrchestrationError::ReviewWorktreeChanged,
        review::ReviewError::NoChangedPaths => OrchestrationError::ReviewNoChanges,
        review::ReviewError::TooManyChecks => OrchestrationError::ReviewCheckLimit,
        review::ReviewError::DuplicateCheck => OrchestrationError::DuplicateReviewCheck,
        review::ReviewError::UnsupportedChangedPath(_) => OrchestrationError::UnsupportedReviewPath,
        review::ReviewError::ChangedFileTooLarge(_) => OrchestrationError::ReviewFileTooLarge,
        review::ReviewError::InvalidOptions
        | review::ReviewError::MissingSearchPath
        | review::ReviewError::UnsafeSearchPath => OrchestrationError::ReviewConfiguration,
        review::ReviewError::Io(_)
        | review::ReviewError::Git(_)
        | review::ReviewError::Execution(_)
        | review::ReviewError::Persistence(_) => OrchestrationError::ReviewExecution,
    }
}

fn map_artifact_error(error: persistence::PersistenceError) -> OrchestrationError {
    match error {
        persistence::PersistenceError::NotFound(resource) => OrchestrationError::NotFound(resource),
        persistence::PersistenceError::ArtifactMetadataCorrupt
        | persistence::PersistenceError::ArtifactIntegrityMismatch => {
            OrchestrationError::ArtifactIntegrity
        }
        error => OrchestrationError::Persistence(error),
    }
}

fn map_mutation_git_error(error: git::GitError) -> OrchestrationError {
    match error {
        git::GitError::NoChanges => OrchestrationError::NoChangesToFinalize,
        git::GitError::MutationPreviewMismatch | git::GitError::WorktreeChangedDuringRead => {
            OrchestrationError::MutationPreviewMismatch
        }
        error => OrchestrationError::Git(error),
    }
}

fn map_finalization_persistence_error(error: persistence::PersistenceError) -> OrchestrationError {
    match error {
        persistence::PersistenceError::FinalizationConflict
        | persistence::PersistenceError::FinalizationDigestMismatch => {
            OrchestrationError::ChangesetFinalizationConflict
        }
        persistence::PersistenceError::VersionConflict {
            aggregate: "changeset",
            ..
        } => OrchestrationError::StaleChangesetVersion,
        error => OrchestrationError::Persistence(error),
    }
}

fn finalization_error_is_divergent(error: &OrchestrationError) -> bool {
    match error {
        OrchestrationError::MutationPreviewMismatch | OrchestrationError::NoChangesToFinalize => {
            true
        }
        OrchestrationError::Git(
            git::GitError::RepositoryIdentityChanged
            | git::GitError::WorktreeChangedDuringRead
            | git::GitError::NoChanges
            | git::GitError::IgnoredContent
            | git::GitError::UnsupportedIndexState
            | git::GitError::MutationPreviewMismatch
            | git::GitError::ChangesetRefConflict
            | git::GitError::CommitVerificationFailed,
        ) => true,
        OrchestrationError::Git(git::GitError::Io(error)) => {
            error.kind() == std::io::ErrorKind::NotFound
        }
        _ => false,
    }
}

fn mutation_result(
    completed: &CompletedChangesetFinalization,
) -> Result<MutationResult, OrchestrationError> {
    let confirmation_digest = completed.finalization.confirmation_digest.clone();
    let checkpoint_id = completed.finalization.scope.checkpoint_id;
    let manifest_sha256 = completed.finalization.scope.manifest_sha256.clone();
    match completed
        .finalization
        .result
        .as_ref()
        .ok_or(OrchestrationError::ChangesetFinalizationCorrupt)?
    {
        ChangesetFinalizationResult::Commit {
            resulting_head_sha,
            app_ref,
        } => Ok(MutationResult::Commit {
            confirmation_digest,
            checkpoint_id,
            manifest_sha256,
            changeset: completed.changeset.clone(),
            worktree_state: completed.worktree.state(),
            resulting_head_sha: resulting_head_sha.clone(),
            app_ref: app_ref.clone(),
        }),
        ChangesetFinalizationResult::Discard => Ok(MutationResult::Discard {
            confirmation_digest,
            checkpoint_id,
            manifest_sha256,
            changeset: completed.changeset.clone(),
            worktree_state: completed.worktree.state(),
        }),
    }
}

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
    Events(EventPage),
    Snapshot(Box<RunSnapshot>),
    History(HistoryResponse),
    Recovery(RecoveryResponse),
    Findings(FindingsResponse),
    ReviewCompleted(ReviewReport),
    RunArtifacts(RunArtifactsResponse),
    RunArtifactSegment(RunArtifactSegmentResponse),
    RunArtifactsDeleted(RunArtifactsDeletedResponse),
    MutationPreview(MutationPreview),
    MutationCompleted(MutationResult),
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
    #[error("changeset is not reviewable")]
    ChangesetNotReviewable,
    #[error("changeset version changed after the request was prepared")]
    StaleChangesetVersion,
    #[error("changeset head changed after the request was prepared")]
    StaleChangesetHead,
    #[error("changeset mutation confirmation did not match the exact preview")]
    MutationConfirmationMismatch,
    #[error("changeset mutation state did not match its checkpoint or Git manifest")]
    MutationPreviewMismatch,
    #[error("worktree has no changes to finalize")]
    NoChangesToFinalize,
    #[error("changeset finalization conflicts with durable state")]
    ChangesetFinalizationConflict,
    #[error("changeset finalization is divergent and requires manual resolution")]
    ChangesetFinalizationDivergent,
    #[error("changeset finalization record is incomplete or corrupt")]
    ChangesetFinalizationCorrupt,
    #[error("approval response does not match the pending request")]
    ApprovalMismatch,
    #[error("command is defined by the protocol but is not implemented by this runtime phase")]
    UnsupportedCommand,
    #[error("provider contract violation: {0}")]
    ProviderContract(&'static str),
    #[error("resource limit exceeded for {0}")]
    ResourceLimit(&'static str),
    #[error("run artifacts are unavailable in this runtime")]
    ArtifactStoreUnavailable,
    #[error("run artifacts can only be deleted after the run reaches a terminal state")]
    ArtifactDeletionRequiresTerminalRun,
    #[error("artifact content failed integrity verification")]
    ArtifactIntegrity,
    #[error("provider process execution failed")]
    Execution(#[source] execution::ExecutionError),
    #[error("provider process exited without a successful status: {0:?}")]
    ProviderProcessFailed(TerminalOutcome),
    #[error("provider process output exceeded the configured limit")]
    ProviderProcessOutputTruncated,
    #[error("provider response was invalid: {0}")]
    Provider(#[from] agents::ProviderError),
    #[error("provider process current directory does not match the registered worktree")]
    ProviderProcessOutsideWorktree,
    #[error("provider requires operating-system process confinement")]
    ProviderProcessUnconfined,
    #[error("provider confinement does not match the canonical registered worktree")]
    ProviderConfinementMismatch,
    #[error("provider process supervision identity did not match the persisted record")]
    ProcessSupervisionMismatch,
    #[error("provider process termination could not be verified")]
    ProcessTerminationUnverified,
    #[error("run was interrupted while provider execution was active")]
    RunInterruptedDuringExecution,
    #[error("run is not available for provider execution")]
    RunDriveUnavailable,
    #[error("process supervision state is unavailable")]
    SupervisionStateUnavailable,
    #[error("the run's mutation lease is missing or owned by another fencing epoch")]
    MutationLeaseUnavailable,
    #[error("active changeset could not be moved to recoverable state")]
    ChangesetRecoveryUnavailable,
    #[error("review service is unavailable in this runtime")]
    ReviewServiceUnavailable,
    #[error("changeset or worktree is not ready for review")]
    InvalidReviewState,
    #[error("worktree changed during review")]
    ReviewWorktreeChanged,
    #[error("review requires at least one changed path")]
    ReviewNoChanges,
    #[error("too many review checks were requested")]
    ReviewCheckLimit,
    #[error("a review check was requested more than once")]
    DuplicateReviewCheck,
    #[error("a changed path cannot be reviewed safely")]
    UnsupportedReviewPath,
    #[error("a changed file exceeds the review size limit")]
    ReviewFileTooLarge,
    #[error("review service configuration is invalid")]
    ReviewConfiguration,
    #[error("review execution failed")]
    ReviewExecution,
    #[error("domain operation failed: {0}")]
    Domain(#[from] domain::DomainError),
    #[error("persistence operation failed: {0}")]
    Persistence(#[from] persistence::PersistenceError),
    #[error("Git operation failed: {0}")]
    Git(#[from] git::GitError),
}
