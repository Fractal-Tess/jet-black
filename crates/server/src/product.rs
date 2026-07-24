use super::{
    MAX_ADMITTED_RUNS, MAX_CONCURRENT_COMMANDS, MAX_CONCURRENT_REVIEWS, MAX_CONCURRENT_RUN_WORKERS,
    Runtime, StaticAssets, apply_security_headers, schedule_run_worker,
};
use axum::{
    Json, Router,
    extract::{
        DefaultBodyLimit, Query, Request, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{any, get, post},
};
use config::PublicBootstrap;
use control_plane::{
    AuthenticatedSession, ControlPlaneError, ControlPlaneStore, EventRecordsPage, IntakeItem,
    Project, ProjectModule, ProjectPage, QuotaLimits, Sprint, Ticket, TicketPriority, User,
    WorkerAssignment, WorkerDevice, WorkflowState, WorkspaceAccess, WorkspaceRole,
};
use protocol::{
    CreateRemoteAssignmentRequest, EnrollWorkerRequest, EnrolledWorker, Envelope, LocalCommand,
    LocalCommandResponse, PROTOCOL_VERSION, ProductClientMessage, ProductCommand,
    ProductCommandResponse, ProductEvent, ProductEventPage, ProductIntakeItem, ProductModule,
    ProductPage, ProductProject, ProductServerMessage, ProductSnapshot, ProductSprint,
    ProductTicket, ProductTicketPriority, ProductUser, ProductWorker, ProductWorkflowState,
    ProductWorkspace, ProductWorkspaceRole, RemoteAssignment, ResponseEnvelope, StructuredError,
    WorkerClaimResponse, WorkerEventAck, WorkerEventRequest, WorkerHeartbeatRequest,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use thiserror::Error;
use tokio::{
    net::TcpListener,
    runtime::Handle,
    sync::{
        Semaphore,
        broadcast::{self, Receiver, Sender},
    },
};
use tower_http::services::{ServeDir, ServeFile};
use uuid::Uuid;

const SESSION_COOKIE: &str = "jet_black_session";
const CSRF_HEADER: &str = "x-csrf-token";
const MAX_PRODUCT_BODY_BYTES: usize = 64 * 1024;
const SNAPSHOT_LIMIT: usize = 1_000;
const EVENT_PAGE_SIZE: usize = 100;
const EVENT_BROADCAST_CAPACITY: usize = 256;
const WORKER_LEASE_DURATION: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct ProductServerConfig {
    pub address: SocketAddr,
    pub public_origin: String,
    pub profile: String,
}

#[derive(Clone)]
struct ProductState {
    listen_address: SocketAddr,
    authority: Arc<str>,
    origin: Arc<str>,
    profile: Arc<str>,
    store: ControlPlaneStore,
    event_sender: Sender<EventSignal>,
    execution_bootstrap: Option<Arc<PublicBootstrap>>,
    runtime: Option<Arc<dyn Runtime>>,
    command_slots: Arc<Semaphore>,
    review_slots: Arc<Semaphore>,
    run_admission_slots: Arc<Semaphore>,
    run_slots: Arc<Semaphore>,
}

#[derive(Debug, Clone, Copy)]
struct EventSignal {
    workspace_id: Uuid,
    cursor: u64,
}

pub struct ProductServer {
    state: ProductState,
    static_assets: Option<StaticAssets>,
}

impl ProductServer {
    pub fn new(
        config: ProductServerConfig,
        store: ControlPlaneStore,
    ) -> Result<Self, ProductServerError> {
        let origin = config.public_origin.trim_end_matches('/').to_owned();
        let origin_authority = origin
            .split_once("://")
            .map(|(_, authority)| authority)
            .filter(|authority| !authority.is_empty() && !authority.contains('/'))
            .ok_or_else(|| ProductServerError::InvalidOrigin(config.public_origin.clone()))?;
        let (event_sender, _) = broadcast::channel(EVENT_BROADCAST_CAPACITY);
        Ok(Self {
            state: ProductState {
                listen_address: config.address,
                authority: origin_authority.to_owned().into(),
                origin: origin.into(),
                profile: config.profile.into(),
                store,
                event_sender,
                execution_bootstrap: None,
                runtime: None,
                command_slots: Arc::new(Semaphore::new(MAX_CONCURRENT_COMMANDS)),
                review_slots: Arc::new(Semaphore::new(MAX_CONCURRENT_REVIEWS)),
                run_admission_slots: Arc::new(Semaphore::new(MAX_ADMITTED_RUNS)),
                run_slots: Arc::new(Semaphore::new(MAX_CONCURRENT_RUN_WORKERS)),
            },
            static_assets: None,
        })
    }

    pub fn with_static_assets(mut self, static_assets: StaticAssets) -> Self {
        self.static_assets = Some(static_assets);
        self
    }

    pub fn with_execution(mut self, bootstrap: PublicBootstrap, runtime: Arc<dyn Runtime>) -> Self {
        self.state.execution_bootstrap = Some(Arc::new(bootstrap));
        self.state.runtime = Some(runtime);
        self
    }

    pub fn router(&self) -> Router {
        let router = Router::new()
            .route("/api/health", get(health))
            .route("/api/bootstrap", get(bootstrap))
            .route("/api/auth/password", post(password_login))
            .route("/api/auth/launch", post(launch_login))
            .route("/api/auth/session", get(current_session))
            .route("/api/auth/rotate", post(rotate_session))
            .route("/api/auth/logout", post(logout))
            .route("/api/product/snapshot", get(snapshot))
            .route("/api/product/commands", post(product_command))
            .route("/api/product/events", get(product_events))
            .route("/api/execution/bootstrap", get(execution_bootstrap))
            .route("/api/execution/commands", post(execution_command))
            .route("/api/workers", get(list_workers))
            .route("/api/workers/enroll", post(enroll_worker))
            .route("/api/workers/assignments", post(create_worker_assignment))
            .route("/api/worker/heartbeat", post(worker_heartbeat))
            .route("/api/worker/claim", post(worker_claim))
            .route("/api/worker/events", post(worker_event))
            .route("/api/ws", get(websocket_upgrade))
            .route("/api", any(api_not_found))
            .route("/api/{*path}", any(api_not_found))
            .layer(DefaultBodyLimit::max(MAX_PRODUCT_BODY_BYTES))
            .with_state(self.state.clone());
        let router = match &self.static_assets {
            Some(static_assets) => {
                let index = static_assets.directory.join("index.html");
                router.fallback_service(
                    ServeDir::new(&static_assets.directory).fallback(ServeFile::new(index)),
                )
            }
            None => router,
        };
        router
            .layer(middleware::from_fn_with_state(
                self.state.clone(),
                require_product_host,
            ))
            .layer(middleware::from_fn(apply_security_headers))
    }

    pub async fn serve(self, listener: TcpListener) -> Result<(), std::io::Error> {
        if listener.local_addr()? != self.state.listen_address {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "listener address does not match product server configuration",
            ));
        }
        axum::serve(listener, self.router()).await
    }
}

async fn api_not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    database: &'static str,
}

async fn health(
    State(state): State<ProductState>,
) -> Result<Json<HealthResponse>, ProductApiError> {
    state.store.integrity_check()?;
    Ok(Json(HealthResponse {
        status: "ok",
        database: "ok",
    }))
}

#[derive(Debug, Serialize)]
struct ProductBootstrap {
    protocol_version: &'static str,
    profile: String,
    authentication: &'static str,
    websocket_path: &'static str,
}

async fn bootstrap(State(state): State<ProductState>) -> Json<ProductBootstrap> {
    Json(ProductBootstrap {
        protocol_version: PROTOCOL_VERSION,
        profile: state.profile.to_string(),
        authentication: "required",
        websocket_path: "/api/ws",
    })
}

#[derive(Debug, Deserialize)]
struct PasswordLoginRequest {
    email: String,
    password: String,
}

async fn password_login(
    State(state): State<ProductState>,
    headers: HeaderMap,
    Json(request): Json<PasswordLoginRequest>,
) -> Result<Response, ProductApiError> {
    validate_origin(&state, &headers)?;
    let issued = state
        .store
        .authenticate_password(&request.email, &request.password)?;
    session_response(&state, issued)
}

#[derive(Debug, Deserialize)]
struct LaunchLoginRequest {
    token: String,
}

async fn launch_login(
    State(state): State<ProductState>,
    headers: HeaderMap,
    Json(request): Json<LaunchLoginRequest>,
) -> Result<Response, ProductApiError> {
    validate_origin(&state, &headers)?;
    let issued = state.store.exchange_launch_token(&request.token)?;
    session_response(&state, issued)
}

fn session_response(
    state: &ProductState,
    issued: control_plane::IssuedSession,
) -> Result<Response, ProductApiError> {
    let session = state
        .store
        .validate_session(&issued.token, Some(&issued.csrf_token))?;
    let cookie = HeaderValue::from_str(&session_cookie(
        &issued.token,
        issued.expires_at_ms,
        state.origin.starts_with("https://"),
    ))
    .map_err(|_| ProductApiError::internal("session cookie could not be created"))?;
    let mut response = Json(SessionResponse {
        csrf_token: issued.csrf_token,
        expires_at_ms: issued.expires_at_ms,
        user: product_user(session.user),
    })
    .into_response();
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    Ok(response)
}

#[derive(Debug, Serialize)]
struct SessionResponse {
    csrf_token: String,
    expires_at_ms: i64,
    user: ProductUser,
}

async fn current_session(
    State(state): State<ProductState>,
    headers: HeaderMap,
) -> Result<Json<ProductUser>, ProductApiError> {
    Ok(Json(product_user(
        authenticate(&state, &headers, false)?.user,
    )))
}

async fn rotate_session(
    State(state): State<ProductState>,
    headers: HeaderMap,
) -> Result<Response, ProductApiError> {
    validate_origin(&state, &headers)?;
    let token = cookie_value(&headers, SESSION_COOKIE)
        .ok_or_else(|| ProductApiError::unauthorized("missing_session", "Session is required"))?;
    let csrf = headers
        .get(CSRF_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ProductApiError::unauthorized("missing_csrf", "CSRF token is required"))?;
    let issued = state.store.rotate_session(token, csrf)?;
    session_response(&state, issued)
}

async fn logout(
    State(state): State<ProductState>,
    headers: HeaderMap,
) -> Result<Response, ProductApiError> {
    validate_origin(&state, &headers)?;
    let session = authenticate(&state, &headers, true)?;
    state.store.revoke_session(session.session_id)?;
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static(
            "jet_black_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0",
        ),
    );
    Ok(response)
}

#[derive(Debug, Default, Deserialize)]
struct SnapshotQuery {
    workspace_id: Option<Uuid>,
}

async fn snapshot(
    State(state): State<ProductState>,
    Query(query): Query<SnapshotQuery>,
    headers: HeaderMap,
) -> Result<Json<ProductSnapshot>, ProductApiError> {
    let session = authenticate(&state, &headers, false)?;
    if let Some(workspace_id) = query.workspace_id {
        state.store.require_permission(
            session.user.id,
            workspace_id,
            control_plane::Permission::Read,
        )?;
    }
    let workspaces = state
        .store
        .workspaces_for_user(session.user.id, SNAPSHOT_LIMIT + 1)?;
    let projects =
        state
            .store
            .projects_for_user(session.user.id, query.workspace_id, SNAPSHOT_LIMIT + 1)?;
    let tickets =
        state
            .store
            .tickets_for_user(session.user.id, query.workspace_id, SNAPSHOT_LIMIT + 1)?;
    let workflow_states = state.store.workflow_states_for_user(
        session.user.id,
        query.workspace_id,
        SNAPSHOT_LIMIT + 1,
    )?;
    let sprints =
        state
            .store
            .sprints_for_user(session.user.id, query.workspace_id, SNAPSHOT_LIMIT + 1)?;
    let modules =
        state
            .store
            .modules_for_user(session.user.id, query.workspace_id, SNAPSHOT_LIMIT + 1)?;
    let pages =
        state
            .store
            .pages_for_user(session.user.id, query.workspace_id, SNAPSHOT_LIMIT + 1)?;
    let intake =
        state
            .store
            .intake_for_user(session.user.id, query.workspace_id, SNAPSHOT_LIMIT + 1)?;
    let user_id = session.user.id;
    let truncated = workspaces.len() > SNAPSHOT_LIMIT
        || projects.len() > SNAPSHOT_LIMIT
        || tickets.len() > SNAPSHOT_LIMIT
        || workflow_states.len() > SNAPSHOT_LIMIT
        || sprints.len() > SNAPSHOT_LIMIT
        || modules.len() > SNAPSHOT_LIMIT
        || pages.len() > SNAPSHOT_LIMIT
        || intake.len() > SNAPSHOT_LIMIT;
    Ok(Json(ProductSnapshot {
        user: product_user(session.user),
        workspaces: workspaces
            .into_iter()
            .take(SNAPSHOT_LIMIT)
            .map(product_workspace)
            .collect(),
        projects: projects
            .into_iter()
            .take(SNAPSHOT_LIMIT)
            .map(product_project)
            .collect(),
        workflow_states: workflow_states
            .into_iter()
            .take(SNAPSHOT_LIMIT)
            .map(product_workflow_state)
            .collect(),
        tickets: tickets
            .into_iter()
            .take(SNAPSHOT_LIMIT)
            .map(product_ticket)
            .collect(),
        sprints: sprints
            .into_iter()
            .take(SNAPSHOT_LIMIT)
            .map(product_sprint)
            .collect(),
        modules: modules
            .into_iter()
            .take(SNAPSHOT_LIMIT)
            .map(product_module)
            .collect(),
        pages: pages
            .into_iter()
            .take(SNAPSHOT_LIMIT)
            .map(product_page)
            .collect(),
        intake: intake
            .into_iter()
            .take(SNAPSHOT_LIMIT)
            .map(product_intake)
            .collect(),
        event_cursor: state
            .store
            .latest_event_cursor(user_id, query.workspace_id)?,
        truncated,
    }))
}

async fn product_command(
    State(state): State<ProductState>,
    headers: HeaderMap,
    Json(envelope): Json<Envelope<ProductCommand>>,
) -> Result<Json<ResponseEnvelope<ProductCommandResponse>>, ProductApiError> {
    validate_origin(&state, &headers)?;
    let session = authenticate(&state, &headers, true)?;
    let result = dispatch_product_command(&state, session.user.id, envelope).await;
    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
struct EventsQuery {
    workspace_id: Uuid,
    #[serde(default)]
    after_cursor: u64,
}

async fn product_events(
    State(state): State<ProductState>,
    Query(query): Query<EventsQuery>,
    headers: HeaderMap,
) -> Result<Json<ProductEventPage>, ProductApiError> {
    let session = authenticate(&state, &headers, false)?;
    Ok(Json(event_page(state.store.events_after(
        session.user.id,
        query.workspace_id,
        query.after_cursor,
        EVENT_PAGE_SIZE,
    )?)))
}

async fn execution_bootstrap(
    State(state): State<ProductState>,
    headers: HeaderMap,
) -> Result<Json<PublicBootstrap>, ProductApiError> {
    authenticate(&state, &headers, false)?;
    state
        .execution_bootstrap
        .as_ref()
        .map(|bootstrap| Json((**bootstrap).clone()))
        .ok_or_else(|| {
            ProductApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "execution_unavailable",
                "This instance does not provide local execution",
            )
        })
}

async fn execution_command(
    State(state): State<ProductState>,
    headers: HeaderMap,
    Json(envelope): Json<Envelope<LocalCommand>>,
) -> Result<Json<ResponseEnvelope<LocalCommandResponse>>, ProductApiError> {
    validate_origin(&state, &headers)?;
    authenticate(&state, &headers, true)?;
    let request_id = envelope.request_id;
    if let Err(error) = envelope.validate_version() {
        return Ok(Json(ResponseEnvelope::error(request_id, error)));
    }
    let runtime = state.runtime.as_ref().cloned().ok_or_else(|| {
        ProductApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "execution_unavailable",
            "This instance does not provide local execution",
        )
    })?;
    let starts_run = matches!(&envelope.payload, LocalCommand::StartRun { .. });
    let runs_review = matches!(&envelope.payload, LocalCommand::ReviewChangeset { .. });
    let run_admission = if starts_run {
        Some(
            Arc::clone(&state.run_admission_slots)
                .acquire_owned()
                .await
                .map_err(|_| ProductApiError::internal("run executor is unavailable"))?,
        )
    } else {
        None
    };
    let runtime_handle = Handle::current();
    let run_slots = Arc::clone(&state.run_slots);
    let executor_slots = if runs_review {
        Arc::clone(&state.review_slots)
    } else {
        Arc::clone(&state.command_slots)
    };
    let executor_permit = executor_slots
        .acquire_owned()
        .await
        .map_err(|_| ProductApiError::internal("command executor is unavailable"))?;
    let result = tokio::task::spawn_blocking(move || {
        let _executor_permit = executor_permit;
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
    .map_err(|_| ProductApiError::internal("command worker failed"))?;
    Ok(Json(match result {
        Ok(response) => ResponseEnvelope::success(request_id, response),
        Err(error) => ResponseEnvelope::error(request_id, error),
    }))
}

async fn list_workers(
    State(state): State<ProductState>,
    Query(query): Query<EventsQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProductWorker>>, ProductApiError> {
    let session = authenticate(&state, &headers, false)?;
    Ok(Json(
        state
            .store
            .workers(session.user.id, query.workspace_id)?
            .into_iter()
            .map(product_worker)
            .collect(),
    ))
}

async fn enroll_worker(
    State(state): State<ProductState>,
    headers: HeaderMap,
    Json(request): Json<EnrollWorkerRequest>,
) -> Result<Json<EnrolledWorker>, ProductApiError> {
    validate_origin(&state, &headers)?;
    let session = authenticate(&state, &headers, true)?;
    let issued = state.store.enroll_worker(
        session.user.id,
        request.workspace_id,
        &request.name,
        &request.protocol_version,
        &request.capabilities,
        &request.repository_identities,
    )?;
    Ok(Json(EnrolledWorker {
        worker: product_worker(issued.worker),
        token: issued.token,
    }))
}

async fn create_worker_assignment(
    State(state): State<ProductState>,
    headers: HeaderMap,
    Json(request): Json<CreateRemoteAssignmentRequest>,
) -> Result<Json<RemoteAssignment>, ProductApiError> {
    validate_origin(&state, &headers)?;
    let session = authenticate(&state, &headers, true)?;
    Ok(Json(remote_assignment(
        state.store.create_worker_assignment(
            session.user.id,
            request.ticket_id,
            request.worker_id,
            &request.repository_identity,
            &request.provider,
            &request.command_json,
        )?,
    )))
}

async fn worker_heartbeat(
    State(state): State<ProductState>,
    headers: HeaderMap,
    Json(request): Json<WorkerHeartbeatRequest>,
) -> Result<Json<ProductWorker>, ProductApiError> {
    let token = worker_bearer_token(&headers)?;
    Ok(Json(product_worker(state.store.worker_heartbeat(
        token,
        &request.protocol_version,
        &request.capabilities,
        &request.repository_identities,
        request.draining,
    )?)))
}

async fn worker_claim(
    State(state): State<ProductState>,
    headers: HeaderMap,
) -> Result<Json<WorkerClaimResponse>, ProductApiError> {
    let token = worker_bearer_token(&headers)?;
    Ok(Json(WorkerClaimResponse {
        assignment: state
            .store
            .claim_worker_assignment(token, WORKER_LEASE_DURATION)?
            .map(remote_assignment),
    }))
}

async fn worker_event(
    State(state): State<ProductState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEventRequest>,
) -> Result<Json<WorkerEventAck>, ProductApiError> {
    let token = worker_bearer_token(&headers)?;
    let event = state.store.append_worker_event(
        token,
        request.assignment_id,
        request.fencing_epoch,
        request.sequence,
        &request.event_kind,
        &request.body,
        request.terminal_status.as_deref(),
    )?;
    Ok(Json(WorkerEventAck {
        assignment_id: event.assignment_id,
        sequence: event.sequence,
        fencing_epoch: event.fencing_epoch,
    }))
}

async fn websocket_upgrade(
    State(state): State<ProductState>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Result<Response, ProductApiError> {
    validate_origin(&state, &headers)?;
    let session = authenticate(&state, &headers, false)?;
    let events = state.event_sender.subscribe();
    Ok(upgrade
        .max_message_size(MAX_PRODUCT_BODY_BYTES)
        .max_frame_size(MAX_PRODUCT_BODY_BYTES)
        .on_upgrade(move |socket| websocket_session(state, session, events, socket))
        .into_response())
}

async fn websocket_session(
    state: ProductState,
    session: AuthenticatedSession,
    mut event_receiver: Receiver<EventSignal>,
    mut socket: WebSocket,
) {
    if send_message(
        &mut socket,
        &ProductServerMessage::Ready {
            session_id: session.session_id,
        },
    )
    .await
    .is_err()
    {
        return;
    }
    let mut subscription: Option<(Uuid, u64)> = None;
    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let Some(incoming) = incoming else {
                    break;
                };
                let Ok(message) = incoming else {
                    break;
                };
                match message {
                    Message::Text(text) => {
                        let parsed = serde_json::from_str::<ProductClientMessage>(&text);
                        let message = match parsed {
                            Ok(message) => message,
                            Err(_) => {
                                if send_error(&mut socket, "invalid_message", "WebSocket message is malformed").await.is_err() {
                                    break;
                                }
                                continue;
                            }
                        };
                        if handle_client_message(
                            &state,
                            &session,
                            &mut subscription,
                            &mut socket,
                            message,
                        )
                        .await
                        .is_err()
                        {
                            break;
                        }
                    }
                    Message::Ping(bytes) => {
                        if socket.send(Message::Pong(bytes)).await.is_err() {
                            break;
                        }
                    }
                    Message::Close(_) => break,
                    Message::Binary(_) | Message::Pong(_) => {}
                }
            }
            signal = event_receiver.recv(), if subscription.is_some() => {
                match signal {
                    Ok(signal) => {
                        let Some((workspace_id, cursor)) = subscription else {
                            continue;
                        };
                        if signal.workspace_id != workspace_id || signal.cursor <= cursor {
                            continue;
                        }
                        if send_replay(&state, &session, &mut subscription, &mut socket).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        if send_replay(&state, &session, &mut subscription, &mut socket).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

async fn handle_client_message(
    state: &ProductState,
    session: &AuthenticatedSession,
    subscription: &mut Option<(Uuid, u64)>,
    socket: &mut WebSocket,
    message: ProductClientMessage,
) -> Result<(), ()> {
    match message {
        ProductClientMessage::Subscribe {
            workspace_id,
            after_cursor,
        } => {
            if state
                .store
                .require_permission(
                    session.user.id,
                    workspace_id,
                    control_plane::Permission::Read,
                )
                .is_err()
            {
                return send_error(socket, "forbidden", "Workspace subscription is forbidden")
                    .await;
            }
            *subscription = Some((workspace_id, after_cursor));
            send_replay(state, session, subscription, socket).await?;
            let cursor = subscription.map_or(after_cursor, |(_, cursor)| cursor);
            send_message(
                socket,
                &ProductServerMessage::Subscribed {
                    workspace_id,
                    cursor,
                },
            )
            .await
        }
        ProductClientMessage::Command(envelope) => {
            let response = dispatch_product_command(state, session.user.id, envelope).await;
            send_message(socket, &ProductServerMessage::CommandResult(response)).await
        }
        ProductClientMessage::Acknowledge { cursor } => {
            if let Some((workspace_id, sent_cursor)) = subscription {
                if cursor > *sent_cursor {
                    return send_error(
                        socket,
                        "invalid_acknowledgement",
                        "Acknowledgement exceeds the last sent cursor",
                    )
                    .await;
                }
                *subscription = Some((*workspace_id, cursor.max(*sent_cursor)));
            }
            Ok(())
        }
        ProductClientMessage::Ping => send_message(socket, &ProductServerMessage::Pong).await,
    }
}

async fn send_replay(
    state: &ProductState,
    session: &AuthenticatedSession,
    subscription: &mut Option<(Uuid, u64)>,
    socket: &mut WebSocket,
) -> Result<(), ()> {
    let Some((workspace_id, cursor)) = *subscription else {
        return Ok(());
    };
    let page =
        match state
            .store
            .events_after(session.user.id, workspace_id, cursor, EVENT_PAGE_SIZE)
        {
            Ok(page) => event_page(page),
            Err(_) => return send_error(socket, "replay_failed", "Event replay failed").await,
        };
    let next_cursor = page.next_cursor;
    send_message(socket, &ProductServerMessage::Events(page)).await?;
    *subscription = Some((workspace_id, next_cursor));
    Ok(())
}

async fn dispatch_product_command(
    state: &ProductState,
    actor_id: Uuid,
    envelope: Envelope<ProductCommand>,
) -> ResponseEnvelope<ProductCommandResponse> {
    let request_id = envelope.request_id;
    if let Err(error) = envelope.validate_version() {
        return ResponseEnvelope::error(request_id, error);
    }
    let result = match envelope.payload {
        ProductCommand::CreateWorkspace { slug, name } => state
            .store
            .create_workspace(actor_id, &slug, &name)
            .map(|workspace| {
                ProductCommandResponse::WorkspaceCreated(product_workspace(WorkspaceAccess {
                    workspace,
                    role: WorkspaceRole::Owner,
                }))
            }),
        ProductCommand::CreateProject {
            workspace_id,
            identifier,
            name,
            description,
            repository_identity,
        } => state
            .store
            .create_project(
                actor_id,
                workspace_id,
                &identifier,
                &name,
                &description,
                repository_identity.as_deref(),
            )
            .map(|project| ProductCommandResponse::ProjectCreated(product_project(project))),
        ProductCommand::CreateTicket {
            project_id,
            title,
            description,
            priority,
            idempotency_key,
        } => state
            .store
            .create_ticket(
                actor_id,
                project_id,
                &title,
                &description,
                ticket_priority(priority),
                Some(&idempotency_key),
                QuotaLimits::default(),
            )
            .map(|ticket| ProductCommandResponse::TicketCreated(product_ticket(ticket))),
        ProductCommand::CreateSprint {
            project_id,
            name,
            description,
            starts_at_ms,
            ends_at_ms,
        } => state
            .store
            .create_sprint(
                actor_id,
                project_id,
                &name,
                &description,
                starts_at_ms,
                ends_at_ms,
            )
            .map(|sprint| ProductCommandResponse::SprintCreated(product_sprint(sprint))),
        ProductCommand::CreateModule {
            project_id,
            name,
            description,
            target_at_ms,
        } => state
            .store
            .create_module(actor_id, project_id, &name, &description, target_at_ms)
            .map(|module| ProductCommandResponse::ModuleCreated(product_module(module))),
        ProductCommand::CreatePage {
            project_id,
            title,
            content,
        } => state
            .store
            .create_page(actor_id, project_id, &title, &content)
            .map(|page| ProductCommandResponse::PageCreated(product_page(page))),
        ProductCommand::CreateIntakeItem {
            project_id,
            title,
            description,
            submitter_email,
        } => state
            .store
            .create_intake_item(
                actor_id,
                project_id,
                &title,
                &description,
                submitter_email.as_deref(),
            )
            .map(|item| ProductCommandResponse::IntakeItemCreated(product_intake(item))),
        ProductCommand::MoveTicket {
            ticket_id,
            state_group,
            expected_version,
        } => state
            .store
            .move_ticket(actor_id, ticket_id, &state_group, expected_version)
            .map(|ticket| ProductCommandResponse::TicketMoved(product_ticket(ticket))),
    };
    match result {
        Ok(response) => {
            if let Ok(workspace_id) = response_workspace_id(state, &response) {
                if let Ok(cursor) = state
                    .store
                    .latest_event_cursor(actor_id, Some(workspace_id))
                {
                    let _ = state.event_sender.send(EventSignal {
                        workspace_id,
                        cursor,
                    });
                }
            }
            ResponseEnvelope::success(request_id, response)
        }
        Err(error) => ResponseEnvelope::error(request_id, structured_control_plane_error(&error)),
    }
}

fn response_workspace_id(
    state: &ProductState,
    response: &ProductCommandResponse,
) -> Result<Uuid, ControlPlaneError> {
    match response {
        ProductCommandResponse::WorkspaceCreated(workspace) => Ok(workspace.id),
        ProductCommandResponse::ProjectCreated(project) => Ok(project.workspace_id),
        ProductCommandResponse::TicketCreated(ticket) => {
            state.store.workspace_for_project(ticket.project_id)
        }
        ProductCommandResponse::SprintCreated(sprint) => {
            state.store.workspace_for_project(sprint.project_id)
        }
        ProductCommandResponse::ModuleCreated(module) => {
            state.store.workspace_for_project(module.project_id)
        }
        ProductCommandResponse::PageCreated(page) => {
            state.store.workspace_for_project(page.project_id)
        }
        ProductCommandResponse::IntakeItemCreated(item) => {
            state.store.workspace_for_project(item.project_id)
        }
        ProductCommandResponse::TicketMoved(ticket) => {
            state.store.workspace_for_project(ticket.project_id)
        }
    }
}

fn authenticate(
    state: &ProductState,
    headers: &HeaderMap,
    require_csrf: bool,
) -> Result<AuthenticatedSession, ProductApiError> {
    let token = cookie_value(headers, SESSION_COOKIE)
        .ok_or_else(|| ProductApiError::unauthorized("missing_session", "Session is required"))?;
    let csrf = if require_csrf {
        Some(
            headers
                .get(CSRF_HEADER)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| {
                    ProductApiError::unauthorized("missing_csrf", "CSRF token is required")
                })?,
        )
    } else {
        None
    };
    state
        .store
        .validate_session(token, csrf)
        .map_err(Into::into)
}

fn validate_origin(state: &ProductState, headers: &HeaderMap) -> Result<(), ProductApiError> {
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ProductApiError::forbidden("missing_origin", "Origin is required"))?;
    if origin != state.origin.as_ref() {
        return Err(ProductApiError::forbidden(
            "invalid_origin",
            "Origin is not allowed",
        ));
    }
    Ok(())
}

async fn require_product_host(
    State(state): State<ProductState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, ProductApiError> {
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ProductApiError::forbidden("missing_host", "Host is required"))?;
    if host != state.authority.as_ref() {
        return Err(ProductApiError::forbidden(
            "invalid_host",
            "Host does not match the configured public origin",
        ));
    }
    Ok(next.run(request).await)
}

fn session_cookie(token: &str, expires_at_ms: i64, secure: bool) -> String {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let max_age = expires_at_ms.saturating_sub(now_ms) / 1_000;
    let secure_attribute = if secure { "; Secure" } else { "" };
    format!(
        "{SESSION_COOKIE}={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age={max_age}{secure_attribute}"
    )
}

fn cookie_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find_map(|cookie| cookie.strip_prefix(&format!("{name}=")))
}

fn worker_bearer_token(headers: &HeaderMap) -> Result<&str, ProductApiError> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty())
        .ok_or_else(|| {
            ProductApiError::unauthorized(
                "missing_worker_credential",
                "Worker bearer credential is required",
            )
        })
}

fn product_user(user: User) -> ProductUser {
    ProductUser {
        id: user.id,
        email: user.email,
        display_name: user.display_name,
        version: user.version,
    }
}

fn product_workspace(access: WorkspaceAccess) -> ProductWorkspace {
    ProductWorkspace {
        id: access.workspace.id,
        slug: access.workspace.slug,
        name: access.workspace.name,
        role: workspace_role(access.role),
        version: access.workspace.version,
    }
}

fn product_project(project: Project) -> ProductProject {
    ProductProject {
        id: project.id,
        workspace_id: project.workspace_id,
        identifier: project.identifier,
        name: project.name,
        description: project.description,
        repository_identity: project.repository_identity,
        version: project.version,
    }
}

fn product_workflow_state(state: WorkflowState) -> ProductWorkflowState {
    ProductWorkflowState {
        id: state.id,
        project_id: state.project_id,
        name: state.name,
        state_group: state.state_group,
        color: state.color,
        position: state.position,
        version: state.version,
    }
}

fn product_ticket(ticket: Ticket) -> ProductTicket {
    ProductTicket {
        id: ticket.id,
        project_id: ticket.project_id,
        sequence_number: ticket.sequence_number,
        title: ticket.title,
        description: ticket.description,
        state_id: ticket.state_id,
        priority: product_ticket_priority(ticket.priority),
        created_by_id: ticket.created_by_id,
        version: ticket.version,
    }
}

fn product_sprint(sprint: Sprint) -> ProductSprint {
    ProductSprint {
        id: sprint.id,
        project_id: sprint.project_id,
        name: sprint.name,
        description: sprint.description,
        starts_at_ms: sprint.starts_at_ms,
        ends_at_ms: sprint.ends_at_ms,
        status: sprint.status,
        version: sprint.version,
    }
}

fn product_module(module: ProjectModule) -> ProductModule {
    ProductModule {
        id: module.id,
        project_id: module.project_id,
        name: module.name,
        description: module.description,
        status: module.status,
        target_at_ms: module.target_at_ms,
        version: module.version,
    }
}

fn product_page(page: ProjectPage) -> ProductPage {
    ProductPage {
        id: page.id,
        project_id: page.project_id,
        title: page.title,
        content: page.content,
        created_by_id: page.created_by_id,
        version: page.version,
    }
}

fn product_intake(item: IntakeItem) -> ProductIntakeItem {
    ProductIntakeItem {
        id: item.id,
        project_id: item.project_id,
        title: item.title,
        description: item.description,
        submitter_email: item.submitter_email,
        status: item.status,
        ticket_id: item.ticket_id,
        version: item.version,
    }
}

fn product_worker(worker: WorkerDevice) -> ProductWorker {
    ProductWorker {
        id: worker.id,
        workspace_id: worker.workspace_id,
        name: worker.name,
        protocol_version: worker.protocol_version,
        capabilities: worker.capabilities,
        repository_identities: worker.repository_identities,
        status: worker.status,
        last_seen_at_ms: worker.last_seen_at_ms,
        version: worker.version,
    }
}

fn remote_assignment(assignment: WorkerAssignment) -> RemoteAssignment {
    RemoteAssignment {
        id: assignment.id,
        workspace_id: assignment.workspace_id,
        worker_id: assignment.worker_id,
        ticket_id: assignment.ticket_id,
        repository_identity: assignment.repository_identity,
        provider: assignment.provider,
        command_json: assignment.command_json,
        status: assignment.status,
        fencing_epoch: assignment.fencing_epoch,
        lease_expires_at_ms: assignment.lease_expires_at_ms,
        next_event_sequence: assignment.next_event_sequence,
    }
}

fn workspace_role(role: WorkspaceRole) -> ProductWorkspaceRole {
    match role {
        WorkspaceRole::Owner => ProductWorkspaceRole::Owner,
        WorkspaceRole::Admin => ProductWorkspaceRole::Admin,
        WorkspaceRole::Member => ProductWorkspaceRole::Member,
        WorkspaceRole::Guest => ProductWorkspaceRole::Guest,
    }
}

fn ticket_priority(priority: ProductTicketPriority) -> TicketPriority {
    match priority {
        ProductTicketPriority::None => TicketPriority::None,
        ProductTicketPriority::Urgent => TicketPriority::Urgent,
        ProductTicketPriority::High => TicketPriority::High,
        ProductTicketPriority::Medium => TicketPriority::Medium,
        ProductTicketPriority::Low => TicketPriority::Low,
    }
}

fn product_ticket_priority(priority: TicketPriority) -> ProductTicketPriority {
    match priority {
        TicketPriority::None => ProductTicketPriority::None,
        TicketPriority::Urgent => ProductTicketPriority::Urgent,
        TicketPriority::High => ProductTicketPriority::High,
        TicketPriority::Medium => ProductTicketPriority::Medium,
        TicketPriority::Low => ProductTicketPriority::Low,
    }
}

fn event_page(page: EventRecordsPage) -> ProductEventPage {
    ProductEventPage {
        events: page
            .events
            .into_iter()
            .map(|event| ProductEvent {
                cursor: event.cursor,
                workspace_id: event.workspace_id,
                aggregate_kind: event.aggregate_kind,
                aggregate_id: event.aggregate_id,
                aggregate_version: event.aggregate_version,
                event_kind: event.event_kind,
                actor_id: event.actor_id,
                body: event.body,
                created_at_ms: event.created_at_ms,
            })
            .collect(),
        next_cursor: page.next_cursor,
        has_more: page.has_more,
    }
}

async fn send_message(socket: &mut WebSocket, message: &ProductServerMessage) -> Result<(), ()> {
    let json = serde_json::to_string(message).map_err(|_| ())?;
    socket
        .send(Message::Text(json.into()))
        .await
        .map_err(|_| ())
}

async fn send_error(socket: &mut WebSocket, code: &str, message: &str) -> Result<(), ()> {
    send_message(
        socket,
        &ProductServerMessage::Error(StructuredError {
            code: code.to_owned(),
            message: message.to_owned(),
            retryable: false,
        }),
    )
    .await
}

fn structured_control_plane_error(error: &ControlPlaneError) -> StructuredError {
    let (code, message, retryable) = match error {
        ControlPlaneError::InvalidCredentials => {
            ("invalid_credentials", "Invalid credentials", false)
        }
        ControlPlaneError::InvalidSession => {
            ("invalid_session", "Session is invalid or expired", false)
        }
        ControlPlaneError::InvalidCsrfToken => ("invalid_csrf", "CSRF token is invalid", false),
        ControlPlaneError::InvalidLaunchToken => (
            "invalid_launch_token",
            "Launch token is invalid or expired",
            false,
        ),
        ControlPlaneError::InvalidWorkerCredential => (
            "invalid_worker_credential",
            "Worker credential is invalid or revoked",
            false,
        ),
        ControlPlaneError::StaleWorkerFence => (
            "stale_worker_fence",
            "Worker assignment fencing epoch is stale",
            false,
        ),
        ControlPlaneError::WorkerEventSequence { .. } => (
            "worker_event_sequence",
            "Worker event sequence is invalid",
            false,
        ),
        ControlPlaneError::Forbidden => ("forbidden", "Action is forbidden", false),
        ControlPlaneError::VersionConflict => (
            "version_conflict",
            "Record changed; refresh before retrying",
            true,
        ),
        ControlPlaneError::NotFound(_) => ("not_found", "Requested resource was not found", false),
        ControlPlaneError::QuotaExceeded(_) => {
            ("quota_exceeded", "Workspace quota was exceeded", false)
        }
        ControlPlaneError::InvalidInput(_) => ("invalid_input", "Request input is invalid", false),
        ControlPlaneError::Database(_) => {
            ("database_unavailable", "Database operation failed", true)
        }
        _ => (
            "control_plane_error",
            "Control-plane operation failed",
            false,
        ),
    };
    StructuredError {
        code: code.to_owned(),
        message: message.to_owned(),
        retryable,
    }
}

#[derive(Debug)]
struct ProductApiError {
    status: StatusCode,
    error: StructuredError,
}

impl ProductApiError {
    fn forbidden(code: &str, message: &str) -> Self {
        Self::new(StatusCode::FORBIDDEN, code, message)
    }

    fn unauthorized(code: &str, message: &str) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, code, message)
    }

    fn internal(message: &str) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_server_error",
            message,
        )
    }

    fn new(status: StatusCode, code: &str, message: &str) -> Self {
        Self {
            status,
            error: StructuredError {
                code: code.to_owned(),
                message: message.to_owned(),
                retryable: false,
            },
        }
    }
}

impl From<ControlPlaneError> for ProductApiError {
    fn from(error: ControlPlaneError) -> Self {
        let status = match error {
            ControlPlaneError::InvalidCredentials
            | ControlPlaneError::InvalidSession
            | ControlPlaneError::InvalidCsrfToken
            | ControlPlaneError::InvalidLaunchToken
            | ControlPlaneError::InvalidWorkerCredential => StatusCode::UNAUTHORIZED,
            ControlPlaneError::Forbidden => StatusCode::FORBIDDEN,
            ControlPlaneError::VersionConflict => StatusCode::CONFLICT,
            ControlPlaneError::NotFound(_) => StatusCode::NOT_FOUND,
            ControlPlaneError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            ControlPlaneError::StaleWorkerFence | ControlPlaneError::WorkerEventSequence { .. } => {
                StatusCode::CONFLICT
            }
            ControlPlaneError::QuotaExceeded(_) => StatusCode::PAYLOAD_TOO_LARGE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status,
            error: structured_control_plane_error(&error),
        }
    }
}

impl IntoResponse for ProductApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.error)).into_response()
    }
}

#[derive(Debug, Error)]
pub enum ProductServerError {
    #[error("invalid public origin: {0}")]
    InvalidOrigin(String),
    #[error("static assets directory could not be opened: {0}")]
    StaticAssets(PathBuf),
}
