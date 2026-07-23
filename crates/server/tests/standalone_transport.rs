use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use config::{ProviderKind, PublicBootstrap};
use domain::{Id, RunState, limits::MAX_COMMAND_BODY_BYTES};
use futures_util::StreamExt;
use protocol::{
    CommandResult, Envelope, EventCursor, EventPage, LocalCommand, LocalCommandResponse,
    OrderedRunEvent, RecoveryResponse, ResponseEnvelope, SemanticEventKind, StructuredError,
};
use serde::Deserialize;
use server::{Runtime, StandaloneServer};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;
use tower::ServiceExt;

const AUTHORITY: &str = "127.0.0.1:43170";
const ORIGIN: &str = "http://127.0.0.1:43170";

#[derive(Default)]
struct FixtureRuntime {
    event_requests: Mutex<Vec<(u64, usize)>>,
}

impl Runtime for FixtureRuntime {
    fn dispatch(&self, command: LocalCommand) -> Result<LocalCommandResponse, StructuredError> {
        match command {
            LocalCommand::GetRecovery => Ok(LocalCommandResponse::Recovery(RecoveryResponse {
                actions: Vec::new(),
            })),
            _ => Err(StructuredError {
                code: "unsupported".to_owned(),
                message: "unsupported fixture command".to_owned(),
                retryable: false,
            }),
        }
    }

    fn run_state(&self, _run_id: Id) -> Result<RunState, StructuredError> {
        Ok(RunState::Running)
    }

    fn events_after(
        &self,
        run_id: Id,
        after_sequence: u64,
        limit: usize,
    ) -> Result<EventPage, StructuredError> {
        self.event_requests
            .lock()
            .unwrap()
            .push((after_sequence, limit));
        let sequence = after_sequence + 1;
        Ok(EventPage {
            events: vec![OrderedRunEvent {
                run_id,
                sequence,
                event: SemanticEventKind::Text {
                    text: "fixture event".to_owned(),
                },
            }],
            next_cursor: EventCursor {
                run_id,
                after_sequence: sequence,
            },
        })
    }
}

fn fixture_server(runtime: Arc<FixtureRuntime>) -> StandaloneServer {
    StandaloneServer::new(
        AUTHORITY.parse::<SocketAddr>().unwrap(),
        PublicBootstrap {
            profile: "standalone".to_owned(),
            version: "test".to_owned(),
            protocol_version: protocol::PROTOCOL_VERSION.to_owned(),
            enabled_features: vec!["local_execution".to_owned()],
            provider_availability: vec![ProviderKind::Mock],
        },
        runtime,
    )
    .unwrap()
}

#[derive(Deserialize)]
struct ExchangeResponse {
    csrf_token: String,
}

async fn exchange(server: &StandaloneServer) -> (Router, String, String) {
    let router = server.router();
    let request = Request::post("/api/session/exchange")
        .header(header::HOST, AUTHORITY)
        .header(header::ORIGIN, ORIGIN)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            r#"{{"token":"{}"}}"#,
            server.launch_token().expose()
        )))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Strict"));
    let session_cookie = cookie.split(';').next().unwrap().to_owned();
    let body = to_bytes(response.into_body(), MAX_COMMAND_BODY_BYTES)
        .await
        .unwrap();
    let exchange: ExchangeResponse = serde_json::from_slice(&body).unwrap();
    (router, session_cookie, exchange.csrf_token)
}

#[tokio::test]
async fn binds_an_ephemeral_loopback_listener_before_server_creation() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    assert!(address.ip().is_loopback());
    assert_ne!(address.port(), 0);
    let server = StandaloneServer::new(
        address,
        PublicBootstrap {
            profile: "standalone".to_owned(),
            version: "test".to_owned(),
            protocol_version: protocol::PROTOCOL_VERSION.to_owned(),
            enabled_features: vec!["local_execution".to_owned()],
            provider_availability: vec![ProviderKind::Mock],
        },
        Arc::new(FixtureRuntime::default()),
    );
    assert!(server.is_ok());
}

#[tokio::test]
async fn bootstrap_rejects_foreign_hosts_and_exposes_no_launch_secret() {
    let server = fixture_server(Arc::new(FixtureRuntime::default()));
    let router = server.router();
    let rejected = router
        .clone()
        .oneshot(
            Request::get("/api/bootstrap")
                .header(header::HOST, "attacker.invalid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::FORBIDDEN);

    let response = router
        .oneshot(
            Request::get("/api/bootstrap")
                .header(header::HOST, AUTHORITY)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), MAX_COMMAND_BODY_BYTES)
        .await
        .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(!text.contains(server.launch_token().expose()));
    assert!(!text.contains('/'));
}

#[tokio::test]
async fn exchange_is_same_origin_single_use_and_body_limited() {
    let server = fixture_server(Arc::new(FixtureRuntime::default()));
    let missing_origin = server
        .router()
        .oneshot(
            Request::post("/api/session/exchange")
                .header(header::HOST, AUTHORITY)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"token":"{}"}}"#,
                    server.launch_token().expose()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_origin.status(), StatusCode::FORBIDDEN);

    let (router, _, _) = exchange(&server).await;
    let reused = router
        .clone()
        .oneshot(
            Request::post("/api/session/exchange")
                .header(header::HOST, AUTHORITY)
                .header(header::ORIGIN, ORIGIN)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"token":"{}"}}"#,
                    server.launch_token().expose()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reused.status(), StatusCode::UNAUTHORIZED);

    let oversized = router
        .oneshot(
            Request::post("/api/session/exchange")
                .header(header::HOST, AUTHORITY)
                .header(header::ORIGIN, ORIGIN)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("x".repeat(MAX_COMMAND_BODY_BYTES + 1)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn commands_require_session_origin_and_csrf() {
    let server = fixture_server(Arc::new(FixtureRuntime::default()));
    let (router, cookie, csrf) = exchange(&server).await;
    let envelope = Envelope::new(LocalCommand::GetRecovery);
    let body = serde_json::to_vec(&envelope).unwrap();

    let missing_csrf = router
        .clone()
        .oneshot(
            Request::post("/api/commands")
                .header(header::HOST, AUTHORITY)
                .header(header::ORIGIN, ORIGIN)
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_csrf.status(), StatusCode::UNAUTHORIZED);

    let response = router
        .oneshot(
            Request::post("/api/commands")
                .header(header::HOST, AUTHORITY)
                .header(header::ORIGIN, ORIGIN)
                .header(header::COOKIE, cookie)
                .header("x-csrf-token", csrf)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), MAX_COMMAND_BODY_BYTES)
        .await
        .unwrap();
    let response: ResponseEnvelope<LocalCommandResponse> = serde_json::from_slice(&body).unwrap();
    assert!(matches!(
        response.result,
        CommandResult::Ok(LocalCommandResponse::Recovery(_))
    ));
}

#[tokio::test]
async fn authenticated_recovery_and_sse_replay_use_bounded_cursors() {
    let runtime = Arc::new(FixtureRuntime::default());
    let server = fixture_server(Arc::clone(&runtime));
    let (router, cookie, _) = exchange(&server).await;

    let recovery = router
        .clone()
        .oneshot(
            Request::get("/api/recovery")
                .header(header::HOST, AUTHORITY)
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(recovery.status(), StatusCode::OK);

    let run_id = Id::new_v4();
    let response = router
        .oneshot(
            Request::get(format!("/api/runs/{run_id}/events?after_sequence=7"))
                .header(header::HOST, AUTHORITY)
                .header(header::COOKIE, cookie)
                .header("last-event-id", "9")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/event-stream"
    );
    let mut stream = response.into_body().into_data_stream();
    let chunk = tokio::time::timeout(std::time::Duration::from_secs(1), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let text = String::from_utf8(chunk.to_vec()).unwrap();
    assert!(text.contains("id: 10"));
    assert_eq!(*runtime.event_requests.lock().unwrap(), vec![(9, 100)]);
}
