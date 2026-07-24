use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use control_plane::ControlPlaneStore;
use futures_util::{SinkExt, StreamExt};
use protocol::{
    CommandResult, Envelope, ProductClientMessage, ProductCommand, ProductCommandResponse,
    ProductServerMessage, ResponseEnvelope,
};
use serde_json::Value;
use server::{ProductServer, ProductServerConfig, StaticAssets};
use std::{net::SocketAddr, time::Duration};
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
