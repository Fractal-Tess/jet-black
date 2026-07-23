mod auth;

use agents::AgentProvider;
use async_stream::stream;
use auth::{AuthError, SessionManager, session_cookie};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response, Sse, sse::Event},
    routing::{any, get, post},
};
use config::{PublicBootstrap, Secret};
use domain::{Id, RunState, limits::MAX_COMMAND_BODY_BYTES};
use orchestration::{CommandOutcome, LocalOrchestrator, OrchestrationError};
use protocol::{
    Envelope, EventPage, LocalCommand, LocalCommandResponse, ResponseEnvelope, SemanticEventKind,
    StructuredError,
};
use serde::{Deserialize, Serialize};
use std::{
    convert::Infallible,
    net::SocketAddr,
    path::{Path as FilePath, PathBuf},
    sync::Arc,
    time::Duration,
};
use thiserror::Error;
use tokio::{
    net::TcpListener,
    runtime::Handle,
    sync::{OwnedSemaphorePermit, Semaphore},
};
use tower_http::services::ServeDir;

const MAX_CONCURRENT_COMMANDS: usize = 4;
const MAX_CONCURRENT_RUN_WORKERS: usize = 4;
const MAX_QUEUED_RUN_WORKERS: usize = 4;
const MAX_ADMITTED_RUNS: usize = MAX_CONCURRENT_RUN_WORKERS + MAX_QUEUED_RUN_WORKERS;
const SSE_PAGE_SIZE: usize = 100;
const SSE_POLL_INTERVAL: Duration = Duration::from_millis(250);

pub trait Runtime: Send + Sync + 'static {
    fn dispatch(&self, command: LocalCommand) -> Result<LocalCommandResponse, StructuredError>;
    fn drive_run_to_approval(&self, run_id: Id) -> Result<(), StructuredError>;
    fn run_state(&self, run_id: Id) -> Result<RunState, StructuredError>;
    fn events_after(
        &self,
        run_id: Id,
        after_sequence: u64,
        limit: usize,
    ) -> Result<EventPage, StructuredError>;
}

impl<P> Runtime for LocalOrchestrator<P>
where
    P: AgentProvider + Send + Sync + 'static,
{
    fn dispatch(&self, command: LocalCommand) -> Result<LocalCommandResponse, StructuredError> {
        self.handle(command)
            .map(command_response)
            .map_err(structured_orchestration_error)
    }

    fn drive_run_to_approval(&self, run_id: Id) -> Result<(), StructuredError> {
        LocalOrchestrator::drive_run_to_approval(self, run_id)
            .map(|_| ())
            .map_err(structured_orchestration_error)
    }

    fn run_state(&self, run_id: Id) -> Result<RunState, StructuredError> {
        LocalOrchestrator::run_state(self, run_id).map_err(structured_orchestration_error)
    }

    fn events_after(
        &self,
        run_id: Id,
        after_sequence: u64,
        limit: usize,
    ) -> Result<EventPage, StructuredError> {
        LocalOrchestrator::events_after(self, run_id, after_sequence, limit)
            .map_err(structured_orchestration_error)
    }
}

#[derive(Clone)]
struct ServerState {
    authority: Arc<str>,
    origin: Arc<str>,
    bootstrap: Arc<PublicBootstrap>,
    sessions: Arc<SessionManager>,
    command_slots: Arc<Semaphore>,
    run_admission_slots: Arc<Semaphore>,
    run_slots: Arc<Semaphore>,
    runtime: Arc<dyn Runtime>,
}

#[derive(Clone)]
pub struct StaticAssets {
    directory: PathBuf,
}

impl StaticAssets {
    pub fn open(directory: impl AsRef<FilePath>) -> Result<Self, ServerError> {
        let directory = directory
            .as_ref()
            .canonicalize()
            .map_err(|source| ServerError::StaticAssets {
                path: directory.as_ref().to_path_buf(),
                source,
            })?;
        let index_path = directory.join("index.html");
        if !index_path.is_file() {
            return Err(ServerError::MissingStaticIndex(index_path));
        }
        Ok(Self { directory })
    }
}

pub struct StandaloneServer {
    state: ServerState,
    launch_token: Secret,
    static_assets: Option<StaticAssets>,
}

impl StandaloneServer {
    pub fn new(
        address: SocketAddr,
        bootstrap: PublicBootstrap,
        runtime: Arc<dyn Runtime>,
    ) -> Result<Self, ServerError> {
        if !address.ip().is_loopback() {
            return Err(ServerError::NonLoopbackAddress(address));
        }
        let authority: Arc<str> = address.to_string().into();
        let origin: Arc<str> = format!("http://{authority}").into();
        let (sessions, launch_token) = SessionManager::new()?;
        Ok(Self {
            state: ServerState {
                authority,
                origin,
                bootstrap: Arc::new(bootstrap),
                sessions: Arc::new(sessions),
                command_slots: Arc::new(Semaphore::new(MAX_CONCURRENT_COMMANDS)),
                run_admission_slots: Arc::new(Semaphore::new(MAX_ADMITTED_RUNS)),
                run_slots: Arc::new(Semaphore::new(MAX_CONCURRENT_RUN_WORKERS)),
                runtime,
            },
            launch_token,
            static_assets: None,
        })
    }

    pub fn with_static_assets(mut self, static_assets: StaticAssets) -> Self {
        self.static_assets = Some(static_assets);
        self
    }

    pub fn launch_token(&self) -> &Secret {
        &self.launch_token
    }

    pub fn router(&self) -> Router {
        let router = Router::new()
            .route("/api/bootstrap", get(bootstrap))
            .route("/api/session/exchange", post(exchange_session))
            .route("/api/commands", post(command))
            .route("/api/recovery", get(recovery))
            .route("/api/runs/{run_id}/events", get(run_events))
            .route("/api/{*path}", any(api_not_found))
            .layer(DefaultBodyLimit::max(MAX_COMMAND_BODY_BYTES))
            .with_state(self.state.clone());
        let router = match &self.static_assets {
            Some(static_assets) => {
                router.fallback_service(ServeDir::new(&static_assets.directory))
            }
            None => router,
        };
        router.layer(middleware::from_fn_with_state(
            self.state.clone(),
            require_valid_host,
        ))
    }

    pub async fn serve(self, listener: TcpListener) -> Result<(), std::io::Error> {
        let address = listener.local_addr()?;
        if address.to_string() != self.state.authority.as_ref() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "listener address does not match server authority",
            ));
        }
        axum::serve(listener, self.router()).await
    }
}

async fn api_not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}

#[derive(Debug, Serialize)]
struct BootstrapResponse {
    #[serde(flatten)]
    bootstrap: PublicBootstrap,
    session_required: bool,
}

async fn bootstrap(
    State(state): State<ServerState>,
) -> Result<Json<BootstrapResponse>, ApiError> {
    Ok(Json(BootstrapResponse {
        bootstrap: (*state.bootstrap).clone(),
        session_required: true,
    }))
}

#[derive(Debug, Deserialize)]
struct ExchangeRequest {
    token: String,
}

async fn exchange_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<ExchangeRequest>,
) -> Result<Response, ApiError> {
    validate_origin(&state, &headers)?;
    let (session_token, exchange) = state.sessions.exchange(&request.token)?;
    let cookie = HeaderValue::from_str(&session_cookie(&session_token))
        .map_err(|_| ApiError::internal("session cookie could not be created"))?;
    let mut response = Json(exchange).into_response();
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    Ok(response)
}

async fn command(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(envelope): Json<Envelope<LocalCommand>>,
) -> Result<Json<ResponseEnvelope<LocalCommandResponse>>, ApiError> {
    validate_mutation_request(&state, &headers)?;
    let request_id = envelope.request_id;
    if let Err(error) = envelope.validate_version() {
        return Ok(Json(ResponseEnvelope::error(request_id, error)));
    }
    let starts_run = matches!(&envelope.payload, LocalCommand::StartRun { .. });
    let run_admission = if starts_run {
        Some(
            Arc::clone(&state.run_admission_slots)
                .acquire_owned()
                .await
                .map_err(|_| ApiError::internal("run executor is unavailable"))?,
        )
    } else {
        None
    };
    let runtime = Arc::clone(&state.runtime);
    let run_slots = Arc::clone(&state.run_slots);
    let runtime_handle = Handle::current();
    let command_permit = Arc::clone(&state.command_slots)
        .acquire_owned()
        .await
        .map_err(|_| ApiError::internal("command executor is unavailable"))?;
    let result = tokio::task::spawn_blocking(move || {
        let _command_permit = command_permit;
        let result = runtime.dispatch(envelope.payload);
        if let (Some(admission_permit), Ok(LocalCommandResponse::RunStarted(started))) =
            (run_admission, &result)
        {
            schedule_run_worker(
                &runtime_handle,
                runtime,
                run_slots,
                started.run_id,
                admission_permit,
            );
        }
        result
    })
    .await
    .map_err(|_| ApiError::internal("command worker failed"))?;
    Ok(Json(match result {
        Ok(response) => ResponseEnvelope::success(request_id, response),
        Err(error) => ResponseEnvelope::error(request_id, error),
    }))
}

fn schedule_run_worker(
    runtime_handle: &Handle,
    runtime: Arc<dyn Runtime>,
    run_slots: Arc<Semaphore>,
    run_id: Id,
    admission_permit: OwnedSemaphorePermit,
) {
    drop(runtime_handle.spawn(async move {
        let Ok(worker_permit) = run_slots.acquire_owned().await else {
            return;
        };
        let _worker_result = tokio::task::spawn_blocking(move || {
            let _worker_permit = worker_permit;
            let _admission_permit = admission_permit;
            runtime.drive_run_to_approval(run_id)
        })
        .await;
    }));
}

async fn recovery(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<LocalCommandResponse>, ApiError> {
    validate_read_request(&state, &headers)?;
    let runtime = Arc::clone(&state.runtime);
    let response = tokio::task::spawn_blocking(move || runtime.dispatch(LocalCommand::GetRecovery))
        .await
        .map_err(|_| ApiError::internal("recovery worker failed"))?
        .map_err(ApiError::runtime)?;
    Ok(Json(response))
}

#[derive(Debug, Default, Deserialize)]
struct EventQuery {
    after_sequence: Option<u64>,
}

async fn run_events(
    State(state): State<ServerState>,
    Path(run_id): Path<Id>,
    Query(query): Query<EventQuery>,
    headers: HeaderMap,
) -> Result<Sse<impl futures_core::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    validate_read_request(&state, &headers)?;
    let last_event_id = headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let mut after_sequence = query.after_sequence.unwrap_or(0).max(last_event_id);
    let runtime = Arc::clone(&state.runtime);
    let state_runtime = Arc::clone(&runtime);
    let initial_state = tokio::task::spawn_blocking(move || state_runtime.run_state(run_id))
        .await
        .map_err(|_| ApiError::internal("event worker failed"))?
        .map_err(ApiError::runtime)?;
    let terminal_at_start = matches!(
        initial_state,
        RunState::Completed | RunState::Interrupted | RunState::Failed
    );
    let events = stream! {
        loop {
            let runtime = Arc::clone(&runtime);
            let page = tokio::task::spawn_blocking(move || {
                runtime.events_after(run_id, after_sequence, SSE_PAGE_SIZE)
            }).await;
            match page {
                Ok(Ok(page)) => {
                    let page_len = page.events.len();
                    let terminal = page.events.iter().any(|ordered| {
                        matches!(
                            ordered.event,
                            SemanticEventKind::Lifecycle {
                                state: RunState::Completed | RunState::Interrupted | RunState::Failed
                            }
                        )
                    });
                    let mut serialization_failed = false;
                    for ordered in page.events {
                        let data = match serde_json::to_string(&ordered) {
                            Ok(data) => data,
                            Err(_) => {
                                serialization_failed = true;
                                break;
                            }
                        };
                        yield Ok(Event::default().id(ordered.sequence.to_string()).data(data));
                    }
                    if serialization_failed {
                        yield Ok(Event::default().event("error").data("event serialization failed"));
                        break;
                    }
                    after_sequence = page.next_cursor.after_sequence;
                    if terminal || (terminal_at_start && page_len == 0) {
                        break;
                    }
                    if page_len == SSE_PAGE_SIZE {
                        continue;
                    }
                }
                Ok(Err(error)) => {
                    if let Ok(data) = serde_json::to_string(&error) {
                        yield Ok(Event::default().event("error").data(data));
                    }
                    break;
                }
                Err(_) => break,
            }
            tokio::time::sleep(SSE_POLL_INTERVAL).await;
        }
    };
    Ok(Sse::new(events).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

async fn require_valid_host(
    State(state): State<ServerState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    validate_host(&state, &headers)?;
    Ok(next.run(request).await)
}

fn validate_mutation_request(state: &ServerState, headers: &HeaderMap) -> Result<(), ApiError> {
    validate_origin(state, headers)?;
    state.sessions.authenticate(headers, true)?;
    Ok(())
}

fn validate_read_request(state: &ServerState, headers: &HeaderMap) -> Result<(), ApiError> {
    state.sessions.authenticate(headers, false)?;
    Ok(())
}

fn validate_host(state: &ServerState, headers: &HeaderMap) -> Result<(), ApiError> {
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ApiError::forbidden("missing_host", "Host header is required"))?;
    if host != state.authority.as_ref() {
        return Err(ApiError::forbidden(
            "invalid_host",
            "Host header does not match the local listener",
        ));
    }
    Ok(())
}

fn validate_origin(state: &ServerState, headers: &HeaderMap) -> Result<(), ApiError> {
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ApiError::forbidden("missing_origin", "Origin header is required"))?;
    if origin != state.origin.as_ref() {
        return Err(ApiError::forbidden(
            "invalid_origin",
            "Origin header does not match the local listener",
        ));
    }
    Ok(())
}

fn command_response(outcome: CommandOutcome) -> LocalCommandResponse {
    match outcome {
        CommandOutcome::RepositoryRegistered(repository) => {
            LocalCommandResponse::RepositoryRegistered(repository)
        }
        CommandOutcome::ChangesetCreated(changeset) => {
            LocalCommandResponse::ChangesetCreated(changeset)
        }
        CommandOutcome::RunStarted(started) => LocalCommandResponse::RunStarted(started),
        CommandOutcome::RunInterrupted(run) => LocalCommandResponse::RunInterrupted(run),
        CommandOutcome::ApprovalRejected(run) => LocalCommandResponse::ApprovalRejected(run),
        CommandOutcome::RunCompleted(completed) => LocalCommandResponse::RunCompleted(completed),
        CommandOutcome::Checkpoint(checkpoint) => LocalCommandResponse::Checkpoint(checkpoint),
        CommandOutcome::Diff(diff) => LocalCommandResponse::Diff(diff),
        CommandOutcome::Events(events) => LocalCommandResponse::Events(events),
        CommandOutcome::Snapshot(snapshot) => LocalCommandResponse::Snapshot(snapshot),
        CommandOutcome::History(history) => LocalCommandResponse::History(history),
        CommandOutcome::Recovery(recovery) => LocalCommandResponse::Recovery(recovery),
        CommandOutcome::Findings(findings) => LocalCommandResponse::Findings(findings),
        CommandOutcome::RunArtifacts(artifacts) => LocalCommandResponse::RunArtifacts(artifacts),
        CommandOutcome::RunArtifactSegment(segment) => {
            LocalCommandResponse::RunArtifactSegment(segment)
        }
        CommandOutcome::RunArtifactsDeleted(deleted) => {
            LocalCommandResponse::RunArtifactsDeleted(deleted)
        }
        CommandOutcome::MutationPreview(preview) => LocalCommandResponse::MutationPreview(preview),
        CommandOutcome::MutationCompleted(result) => {
            LocalCommandResponse::MutationCompleted(result)
        }
    }
}

fn structured_orchestration_error(error: OrchestrationError) -> StructuredError {
    let (code, message, retryable) = match error {
        OrchestrationError::NotFound(_) => ("not_found", "requested resource was not found", false),
        OrchestrationError::StaleBase => ("stale_base", "repository base changed", false),
        OrchestrationError::ChangesetNotReviewable => (
            "changeset_not_reviewable",
            "changeset is not ready to be committed or discarded",
            false,
        ),
        OrchestrationError::StaleChangesetVersion => (
            "stale_changeset_version",
            "changeset version changed after preview",
            false,
        ),
        OrchestrationError::StaleChangesetHead => (
            "stale_changeset_head",
            "changeset head changed after preview",
            false,
        ),
        OrchestrationError::MutationConfirmationMismatch => (
            "mutation_confirmation_mismatch",
            "confirmation did not match the exact mutation preview",
            false,
        ),
        OrchestrationError::MutationPreviewMismatch => (
            "mutation_preview_mismatch",
            "changeset no longer matches the mutation preview",
            false,
        ),
        OrchestrationError::ChangesetFinalizationDivergent => (
            "changeset_divergent",
            "changeset finalization requires manual resolution",
            false,
        ),
        OrchestrationError::ChangesetFinalizationCorrupt => (
            "changeset_finalization_corrupt",
            "changeset finalization state is invalid",
            false,
        ),
        OrchestrationError::ApprovalMismatch => (
            "approval_mismatch",
            "approval did not match the pending action",
            false,
        ),
        OrchestrationError::UnsupportedCommand => {
            ("unsupported_command", "command is not implemented", false)
        }
        OrchestrationError::ResourceLimit(_) => {
            ("resource_limit", "resource limit exceeded", false)
        }
        OrchestrationError::ArtifactStoreUnavailable => (
            "artifact_store_unavailable",
            "run artifacts are unavailable",
            false,
        ),
        OrchestrationError::ArtifactDeletionRequiresTerminalRun => (
            "artifact_run_active",
            "run artifacts cannot be deleted while the run is active",
            false,
        ),
        OrchestrationError::ArtifactIntegrity => (
            "artifact_integrity_failed",
            "artifact content failed integrity verification",
            false,
        ),
        OrchestrationError::MutationLeaseUnavailable => (
            "mutation_lease_unavailable",
            "mutation lease is unavailable",
            true,
        ),
        OrchestrationError::NoChangesToFinalize => (
            "no_changes_to_finalize",
            "worktree has no changes to finalize",
            false,
        ),
        OrchestrationError::ChangesetFinalizationConflict => (
            "changeset_finalization_conflict",
            "changeset finalization conflicts with durable state",
            false,
        ),
        _ => ("runtime_error", "local runtime operation failed", false),
    };
    StructuredError {
        code: code.to_owned(),
        message: message.to_owned(),
        retryable,
    }
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    error: StructuredError,
}

impl ApiError {
    fn forbidden(code: &str, message: &str) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            error: StructuredError {
                code: code.to_owned(),
                message: message.to_owned(),
                retryable: false,
            },
        }
    }

    fn internal(message: &str) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: StructuredError {
                code: "internal_server_error".to_owned(),
                message: message.to_owned(),
                retryable: false,
            },
        }
    }

    fn runtime(error: StructuredError) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error,
        }
    }
}

impl From<AuthError> for ApiError {
    fn from(error: AuthError) -> Self {
        let status = match error {
            AuthError::StateUnavailable | AuthError::RandomnessUnavailable => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            AuthError::InvalidExchangeToken
            | AuthError::ExchangeConsumed
            | AuthError::MissingSession
            | AuthError::InvalidSession
            | AuthError::MissingCsrf
            | AuthError::InvalidCsrf => StatusCode::UNAUTHORIZED,
        };
        Self {
            status,
            error: StructuredError {
                code: "local_authentication_failed".to_owned(),
                message: error.to_string(),
                retryable: false,
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.error)).into_response()
    }
}

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("standalone server address must be loopback: {0}")]
    NonLoopbackAddress(SocketAddr),
    #[error("local authentication could not be initialized: {0}")]
    Authentication(#[from] AuthError),
    #[error("static assets directory could not be opened at {path}: {source}")]
    StaticAssets {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("static assets index is missing: {0}")]
    MissingStaticIndex(PathBuf),
}
