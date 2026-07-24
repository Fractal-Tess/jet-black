use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use config::PublicBootstrap;
use control_plane::{ControlPlaneStore, QuotaLimits, TicketPriority};
use domain::{ProviderSelection, RunState};
use futures_util::{SinkExt, StreamExt};
use protocol::{
    CommandResult, Envelope, EventCursor, EventPage, LocalCommand, LocalCommandResponse,
    ProductClientMessage, ProductCommand, ProductCommandResponse, ProductServerMessage,
    RecoveryResponse, ResponseEnvelope, StructuredError,
};
use serde_json::Value;
use server::{ProductServer, ProductServerConfig, Runtime, StaticAssets};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tempfile::TempDir;
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};
use tower::ServiceExt;

const ADDRESS: &str = "127.0.0.1:43191";
const ORIGIN: &str = "http://127.0.0.1:43191";

struct TestServer {
    _directory: TempDir,
    store: ControlPlaneStore,
    router: Router,
}

struct ProductExecutionFixture;

impl Runtime for ProductExecutionFixture {
    fn dispatch(&self, command: LocalCommand) -> Result<LocalCommandResponse, StructuredError> {
        match command {
            LocalCommand::GetRecovery => Ok(LocalCommandResponse::Recovery(RecoveryResponse {
                actions: Vec::new(),
            })),
            _ => Err(StructuredError {
                code: "unsupported_fixture_command".to_owned(),
                message: "fixture command is unsupported".to_owned(),
                retryable: false,
            }),
        }
    }

    fn drive_run_to_approval(&self, _run_id: uuid::Uuid) -> Result<(), StructuredError> {
        Ok(())
    }

    fn run_state(&self, _run_id: uuid::Uuid) -> Result<RunState, StructuredError> {
        Ok(RunState::Completed)
    }

    fn events_after(
        &self,
        run_id: uuid::Uuid,
        after_sequence: u64,
        _limit: usize,
    ) -> Result<EventPage, StructuredError> {
        Ok(EventPage {
            events: Vec::new(),
            next_cursor: EventCursor {
                run_id,
                after_sequence,
            },
        })
    }
}

fn test_server() -> TestServer {
    let directory = tempfile::tempdir().expect("temporary directory");
    let store =
        ControlPlaneStore::open(directory.path().join("product.sqlite")).expect("control plane");
    store
        .seed_development("development password")
        .expect("development seed");
    let server = ProductServer::new(
        ProductServerConfig {
            address: ADDRESS.parse().expect("address"),
            public_origin: ORIGIN.to_owned(),
            profile: "server".to_owned(),
        },
        store.clone(),
    )
    .expect("product server");
    TestServer {
        _directory: directory,
        store,
        router: server.router(),
    }
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 128 * 1024)
        .await
        .expect("response body");
    serde_json::from_slice(&bytes).expect("JSON response")
}

async fn login(router: &Router) -> (String, String) {
    let response = router
        .clone()
        .oneshot(
            Request::post("/api/auth/password")
                .header(header::HOST, ADDRESS)
                .header(header::ORIGIN, ORIGIN)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"email":"dev@jet-black.local","password":"development password"}"#,
                ))
                .expect("login request"),
        )
        .await
        .expect("login response");
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .expect("session cookie")
        .to_str()
        .expect("cookie text")
        .split(';')
        .next()
        .expect("cookie value")
        .to_owned();
    let body = response_json(response).await;
    let csrf = body["csrf_token"].as_str().expect("csrf token").to_owned();
    (cookie, csrf)
}

#[tokio::test]
async fn password_auth_snapshot_commands_and_logout_are_enforced() {
    let server = test_server();
    let health = server
        .router
        .clone()
        .oneshot(
            Request::get("/api/health")
                .header(header::HOST, ADDRESS)
                .body(Body::empty())
                .expect("health request"),
        )
        .await
        .expect("health response");
    assert_eq!(health.status(), StatusCode::OK);

    let foreign_origin = server
        .router
        .clone()
        .oneshot(
            Request::post("/api/auth/password")
                .header(header::HOST, ADDRESS)
                .header(header::ORIGIN, "https://attacker.invalid")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"email":"dev@jet-black.local","password":"development password"}"#,
                ))
                .expect("login request"),
        )
        .await
        .expect("login response");
    assert_eq!(foreign_origin.status(), StatusCode::FORBIDDEN);

    let (cookie, csrf) = login(&server.router).await;
    let snapshot = server
        .router
        .clone()
        .oneshot(
            Request::get("/api/product/snapshot")
                .header(header::HOST, ADDRESS)
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("snapshot request"),
        )
        .await
        .expect("snapshot response");
    assert_eq!(snapshot.status(), StatusCode::OK);
    let snapshot = response_json(snapshot).await;
    assert_eq!(snapshot["user"]["email"], "dev@jet-black.local");
    assert_eq!(
        snapshot["workspaces"].as_array().expect("workspaces").len(),
        1
    );

    let command = Envelope::new(ProductCommand::CreateWorkspace {
        slug: "second-workspace".to_owned(),
        name: "Second workspace".to_owned(),
    });
    let missing_csrf = server
        .router
        .clone()
        .oneshot(
            Request::post("/api/product/commands")
                .header(header::HOST, ADDRESS)
                .header(header::ORIGIN, ORIGIN)
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&command).expect("serialize command"),
                ))
                .expect("command request"),
        )
        .await
        .expect("command response");
    assert_eq!(missing_csrf.status(), StatusCode::UNAUTHORIZED);

    let response = server
        .router
        .clone()
        .oneshot(
            Request::post("/api/product/commands")
                .header(header::HOST, ADDRESS)
                .header(header::ORIGIN, ORIGIN)
                .header(header::COOKIE, &cookie)
                .header("x-csrf-token", &csrf)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&command).expect("serialize command"),
                ))
                .expect("command request"),
        )
        .await
        .expect("command response");
    assert_eq!(response.status(), StatusCode::OK);
    let body: ResponseEnvelope<ProductCommandResponse> =
        serde_json::from_value(response_json(response).await).expect("command envelope");
    assert!(matches!(
        body.result,
        CommandResult::Ok(ProductCommandResponse::WorkspaceCreated(_))
    ));

    let logout = server
        .router
        .clone()
        .oneshot(
            Request::post("/api/auth/logout")
                .header(header::HOST, ADDRESS)
                .header(header::ORIGIN, ORIGIN)
                .header(header::COOKIE, &cookie)
                .header("x-csrf-token", &csrf)
                .body(Body::empty())
                .expect("logout request"),
        )
        .await
        .expect("logout response");
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);
    let rejected = server
        .router
        .clone()
        .oneshot(
            Request::get("/api/product/snapshot")
                .header(header::HOST, ADDRESS)
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("snapshot request"),
        )
        .await
        .expect("snapshot response");
    assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rejects_bad_versions_cross_workspace_access_and_oversized_bodies() {
    let server = test_server();
    let outsider = server
        .store
        .create_user("outsider@example.com", "Outsider", "outsider password")
        .expect("outsider");
    let issued = server
        .store
        .authenticate_password("outsider@example.com", "outsider password")
        .expect("outsider session");
    let seed = server
        .store
        .seed_development("development password")
        .expect("seed");
    let cross_workspace = Envelope::new(ProductCommand::CreateProject {
        workspace_id: seed.workspace.id,
        identifier: "NO".to_owned(),
        name: "Forbidden".to_owned(),
        description: String::new(),
        repository_identity: None,
    });
    let response = server
        .router
        .clone()
        .oneshot(
            Request::post("/api/product/commands")
                .header(header::HOST, ADDRESS)
                .header(header::ORIGIN, ORIGIN)
                .header(
                    header::COOKIE,
                    format!("jet_black_session={}", issued.token),
                )
                .header("x-csrf-token", &issued.csrf_token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&cross_workspace).expect("serialize"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    let body: ResponseEnvelope<ProductCommandResponse> =
        serde_json::from_value(response_json(response).await).expect("response envelope");
    assert!(matches!(
        body.result,
        CommandResult::Error(ref error) if error.code == "forbidden"
    ));
    assert_eq!(
        server
            .store
            .workspaces_for_user(outsider.id, 10)
            .expect("outsider workspaces"),
        []
    );

    let oversized = server
        .router
        .clone()
        .oneshot(
            Request::post("/api/auth/password")
                .header(header::HOST, ADDRESS)
                .header(header::ORIGIN, ORIGIN)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(vec![b'x'; 65 * 1024]))
                .expect("oversized request"),
        )
        .await
        .expect("oversized response");
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);

    let mut bad_version = Envelope::new(ProductCommand::CreateWorkspace {
        slug: "ignored".to_owned(),
        name: "Ignored".to_owned(),
    });
    bad_version.version = "0.1".to_owned();
    let seed_session = server
        .store
        .authenticate_password("dev@jet-black.local", "development password")
        .expect("seed session");
    let response = server
        .router
        .clone()
        .oneshot(
            Request::post("/api/product/commands")
                .header(header::HOST, ADDRESS)
                .header(header::ORIGIN, ORIGIN)
                .header(
                    header::COOKIE,
                    format!("jet_black_session={}", seed_session.token),
                )
                .header("x-csrf-token", seed_session.csrf_token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&bad_version).expect("serialize"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    let body: ResponseEnvelope<ProductCommandResponse> =
        serde_json::from_value(response_json(response).await).expect("response envelope");
    assert!(matches!(
        body.result,
        CommandResult::Error(ref error) if error.code == "unsupported_protocol_version"
    ));
}

async fn spawn_product_server(
    store: ControlPlaneStore,
) -> (SocketAddr, JoinHandle<Result<(), std::io::Error>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
    let address = listener.local_addr().expect("listener address");
    let server = ProductServer::new(
        ProductServerConfig {
            address,
            public_origin: format!("http://{address}"),
            profile: "server".to_owned(),
        },
        store,
    )
    .expect("product server");
    let task = tokio::spawn(server.serve(listener));
    (address, task)
}

async fn receive_server_message(
    socket: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> ProductServerMessage {
    let message = tokio::time::timeout(Duration::from_secs(2), socket.next())
        .await
        .expect("websocket response timeout")
        .expect("websocket open")
        .expect("websocket message");
    let Message::Text(text) = message else {
        panic!("expected text WebSocket message");
    };
    serde_json::from_str(&text).expect("server message")
}

#[tokio::test]
async fn websocket_authenticates_replays_commands_and_reports_malformed_messages() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let store =
        ControlPlaneStore::open(directory.path().join("product.sqlite")).expect("control plane");
    let seed = store
        .seed_development("development password")
        .expect("development seed");
    let workspace = store
        .create_workspace(seed.user.id, "realtime", "Realtime")
        .expect("workspace event");
    let issued = store
        .authenticate_password("dev@jet-black.local", "development password")
        .expect("session");
    let (address, server_task) = spawn_product_server(store).await;

    let mut request = format!("ws://{address}/api/ws")
        .into_client_request()
        .expect("websocket request");
    request.headers_mut().insert(
        header::ORIGIN,
        format!("http://{address}").parse().expect("origin"),
    );
    request.headers_mut().insert(
        header::COOKIE,
        format!("jet_black_session={}", issued.token)
            .parse()
            .expect("cookie"),
    );
    let (mut socket, response) = connect_async(request).await.expect("websocket connect");
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    assert!(matches!(
        receive_server_message(&mut socket).await,
        ProductServerMessage::Ready { .. }
    ));

    socket
        .send(Message::Text(
            serde_json::to_string(&ProductClientMessage::Subscribe {
                workspace_id: workspace.id,
                after_cursor: 0,
            })
            .expect("subscribe JSON")
            .into(),
        ))
        .await
        .expect("subscribe");
    let events = receive_server_message(&mut socket).await;
    assert!(matches!(
        events,
        ProductServerMessage::Events(ref page)
            if page.events.iter().any(|event| event.event_kind == "workspace.created")
    ));
    assert!(matches!(
        receive_server_message(&mut socket).await,
        ProductServerMessage::Subscribed { workspace_id, .. } if workspace_id == workspace.id
    ));

    socket
        .send(Message::Text(
            serde_json::to_string(&ProductClientMessage::Command(Envelope::new(
                ProductCommand::CreateProject {
                    workspace_id: workspace.id,
                    identifier: "RT".to_owned(),
                    name: "Realtime project".to_owned(),
                    description: String::new(),
                    repository_identity: None,
                },
            )))
            .expect("command JSON")
            .into(),
        ))
        .await
        .expect("command");
    assert!(matches!(
        receive_server_message(&mut socket).await,
        ProductServerMessage::CommandResult(ResponseEnvelope {
            result: CommandResult::Ok(ProductCommandResponse::ProjectCreated(_)),
            ..
        })
    ));
    assert!(matches!(
        receive_server_message(&mut socket).await,
        ProductServerMessage::Events(ref page)
            if page.events.iter().any(|event| event.event_kind == "project.created")
    ));

    socket
        .send(Message::Text("{not-json".into()))
        .await
        .expect("malformed message");
    assert!(matches!(
        receive_server_message(&mut socket).await,
        ProductServerMessage::Error(ref error) if error.code == "invalid_message"
    ));
    socket.close(None).await.expect("close websocket");
    server_task.abort();
}

#[tokio::test]
async fn static_assets_use_index_fallback_without_shadowing_api_routes() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_directory = tempfile::tempdir().expect("database directory");
    std::fs::write(
        directory.path().join("index.html"),
        "<main>Jet Black static client</main>",
    )
    .expect("write index");
    std::fs::write(
        directory.path().join("app.js"),
        "export const ready = true;",
    )
    .expect("write asset");
    let store = ControlPlaneStore::open(database_directory.path().join("product.sqlite"))
        .expect("control plane");
    let server = ProductServer::new(
        ProductServerConfig {
            address: ADDRESS.parse().expect("address"),
            public_origin: ORIGIN.to_owned(),
            profile: "server".to_owned(),
        },
        store,
    )
    .expect("server")
    .with_static_assets(StaticAssets::open(directory.path()).expect("static assets"));
    let router = server.router();

    let deep_link = router
        .clone()
        .oneshot(
            Request::get("/workspace/platform/tickets/42")
                .header(header::HOST, ADDRESS)
                .body(Body::empty())
                .expect("deep-link request"),
        )
        .await
        .expect("deep-link response");
    assert_eq!(deep_link.status(), StatusCode::OK);
    let body = to_bytes(deep_link.into_body(), 16 * 1024)
        .await
        .expect("deep-link body");
    assert_eq!(body, "<main>Jet Black static client</main>");

    let missing_api = router
        .oneshot(
            Request::get("/api/not-real")
                .header(header::HOST, ADDRESS)
                .body(Body::empty())
                .expect("API request"),
        )
        .await
        .expect("API response");
    assert_eq!(missing_api.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn product_session_authorizes_the_composed_execution_runtime() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let store =
        ControlPlaneStore::open(directory.path().join("product.sqlite")).expect("control plane");
    store
        .seed_development("development password")
        .expect("development seed");
    let server = ProductServer::new(
        ProductServerConfig {
            address: ADDRESS.parse().expect("address"),
            public_origin: ORIGIN.to_owned(),
            profile: "standalone".to_owned(),
        },
        store,
    )
    .expect("server")
    .with_execution(
        PublicBootstrap {
            profile: "standalone".to_owned(),
            version: "test".to_owned(),
            protocol_version: protocol::PROTOCOL_VERSION.to_owned(),
            enabled_features: vec!["local-execution".to_owned()],
            provider_availability: vec![domain::ProviderKind::Mock],
            default_provider: ProviderSelection::new(domain::ProviderKind::Mock, None)
                .expect("provider selection"),
        },
        Arc::new(ProductExecutionFixture),
    );
    let router = server.router();
    let (cookie, csrf) = login(&router).await;

    let bootstrap = router
        .clone()
        .oneshot(
            Request::get("/api/execution/bootstrap")
                .header(header::HOST, ADDRESS)
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("bootstrap request"),
        )
        .await
        .expect("bootstrap response");
    assert_eq!(bootstrap.status(), StatusCode::OK);

    let command = Envelope::new(LocalCommand::GetRecovery);
    let response = router
        .oneshot(
            Request::post("/api/execution/commands")
                .header(header::HOST, ADDRESS)
                .header(header::ORIGIN, ORIGIN)
                .header(header::COOKIE, cookie)
                .header("x-csrf-token", csrf)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&command).expect("serialize command"),
                ))
                .expect("command request"),
        )
        .await
        .expect("command response");
    let response: ResponseEnvelope<LocalCommandResponse> =
        serde_json::from_value(response_json(response).await).expect("response envelope");
    assert!(matches!(
        response.result,
        CommandResult::Ok(LocalCommandResponse::Recovery(RecoveryResponse { actions }))
            if actions.is_empty()
    ));
}

#[tokio::test]
async fn remote_worker_enrollment_claim_and_outbox_use_separate_device_auth() {
    let server = test_server();
    let seed = server
        .store
        .seed_development("development password")
        .expect("development seed");
    let ticket = server
        .store
        .create_ticket(
            seed.user.id,
            seed.project.id,
            "Remote execution",
            "",
            TicketPriority::High,
            Some("remote-transport"),
            QuotaLimits::default(),
        )
        .expect("ticket");
    let (cookie, csrf) = login(&server.router).await;
    let enroll = server
        .router
        .clone()
        .oneshot(
            Request::post("/api/workers/enroll")
                .header(header::HOST, ADDRESS)
                .header(header::ORIGIN, ORIGIN)
                .header(header::COOKIE, &cookie)
                .header("x-csrf-token", &csrf)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "workspace_id": seed.workspace.id,
                        "name": "Remote laptop",
                        "protocol_version": protocol::PROTOCOL_VERSION,
                        "capabilities": ["mock", "review"],
                        "repository_identities": ["github:example/jet-black"]
                    })
                    .to_string(),
                ))
                .expect("enroll request"),
        )
        .await
        .expect("enroll response");
    assert_eq!(enroll.status(), StatusCode::OK);
    let enrolled = response_json(enroll).await;
    let token = enrolled["token"].as_str().expect("worker token");
    let worker_id = enrolled["worker"]["id"]
        .as_str()
        .expect("worker id")
        .to_owned();

    let heartbeat = server
        .router
        .clone()
        .oneshot(
            Request::post("/api/worker/heartbeat")
                .header(header::HOST, ADDRESS)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "protocol_version": protocol::PROTOCOL_VERSION,
                        "capabilities": ["mock", "review"],
                        "repository_identities": ["github:example/jet-black"],
                        "draining": false
                    })
                    .to_string(),
                ))
                .expect("heartbeat request"),
        )
        .await
        .expect("heartbeat response");
    assert_eq!(heartbeat.status(), StatusCode::OK);

    let assignment = server
        .router
        .clone()
        .oneshot(
            Request::post("/api/workers/assignments")
                .header(header::HOST, ADDRESS)
                .header(header::ORIGIN, ORIGIN)
                .header(header::COOKIE, &cookie)
                .header("x-csrf-token", &csrf)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "ticket_id": ticket.id,
                        "worker_id": worker_id,
                        "repository_identity": "github:example/jet-black",
                        "provider": "mock",
                        "command_json": "{\"title\":\"Remote execution\"}"
                    })
                    .to_string(),
                ))
                .expect("assignment request"),
        )
        .await
        .expect("assignment response");
    assert_eq!(assignment.status(), StatusCode::OK);
    let assigned = response_json(assignment).await;

    let claim = server
        .router
        .clone()
        .oneshot(
            Request::post("/api/worker/claim")
                .header(header::HOST, ADDRESS)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("claim request"),
        )
        .await
        .expect("claim response");
    assert_eq!(claim.status(), StatusCode::OK);
    let claimed = response_json(claim).await;
    assert_eq!(claimed["assignment"]["id"], assigned["id"]);

    let event = server
        .router
        .oneshot(
            Request::post("/api/worker/events")
                .header(header::HOST, ADDRESS)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "assignment_id": assigned["id"],
                        "fencing_epoch": assigned["fencing_epoch"],
                        "sequence": 1,
                        "event_kind": "run.accepted",
                        "body": "{\"ok\":true}",
                        "terminal_status": null
                    })
                    .to_string(),
                ))
                .expect("event request"),
        )
        .await
        .expect("event response");
    assert_eq!(event.status(), StatusCode::OK);
    assert_eq!(response_json(event).await["sequence"], 1);
}
