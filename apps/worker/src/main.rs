use protocol::{
    PROTOCOL_VERSION, RemoteAssignment, WorkerClaimResponse, WorkerEventAck, WorkerEventRequest,
    WorkerHeartbeatRequest,
};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::{env, path::PathBuf, process::Stdio, time::Duration};
use tokio::{
    io::AsyncWriteExt,
    process::Command,
    time::{interval, sleep},
};

const POLL_INTERVAL: Duration = Duration::from_secs(2);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Debug)]
struct WorkerConfig {
    control_plane: String,
    token: String,
    handler: PathBuf,
    capabilities: Vec<String>,
    repository_identities: Vec<String>,
}

#[derive(Debug, Serialize)]
struct HandlerInput<'a> {
    assignment: &'a RemoteAssignment,
}

#[derive(Debug, Deserialize)]
struct HandlerOutput {
    #[serde(default)]
    events: Vec<HandlerEvent>,
    terminal_status: String,
}

#[derive(Debug, Deserialize)]
struct HandlerEvent {
    event_kind: String,
    #[serde(default = "empty_json_object")]
    body: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = WorkerConfig::load()?;
    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;
    let mut heartbeat_timer = interval(HEARTBEAT_INTERVAL);
    loop {
        tokio::select! {
            _ = heartbeat_timer.tick() => {
                heartbeat(&client, &config).await?;
            }
            _ = sleep(POLL_INTERVAL) => {
                if let Some(assignment) = claim(&client, &config).await? {
                    execute_assignment(&client, &config, assignment).await?;
                }
            }
        }
    }
}

impl WorkerConfig {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let control_plane = required_env("JET_BLACK_CONTROL_PLANE")?
            .trim_end_matches('/')
            .to_owned();
        let parsed = reqwest::Url::parse(&control_plane)?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err("JET_BLACK_CONTROL_PLANE must be an HTTP(S) origin".into());
        }
        let handler = PathBuf::from(required_env("JET_BLACK_WORKER_HANDLER")?);
        if !handler.is_absolute() {
            return Err("JET_BLACK_WORKER_HANDLER must be an absolute executable path".into());
        }
        Ok(Self {
            control_plane,
            token: required_env("JET_BLACK_WORKER_TOKEN")?,
            handler,
            capabilities: csv_env("JET_BLACK_WORKER_CAPABILITIES"),
            repository_identities: csv_env("JET_BLACK_WORKER_REPOSITORIES"),
        })
    }
}

async fn heartbeat(
    client: &Client,
    config: &WorkerConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    send_json::<_, serde_json::Value>(
        client,
        config,
        "/api/worker/heartbeat",
        &WorkerHeartbeatRequest {
            protocol_version: PROTOCOL_VERSION.to_owned(),
            capabilities: config.capabilities.clone(),
            repository_identities: config.repository_identities.clone(),
            draining: false,
        },
    )
    .await?;
    Ok(())
}

async fn claim(
    client: &Client,
    config: &WorkerConfig,
) -> Result<Option<RemoteAssignment>, Box<dyn std::error::Error>> {
    let response = client
        .post(format!("{}/api/worker/claim", config.control_plane))
        .bearer_auth(&config.token)
        .send()
        .await?;
    let response = require_success(response).await?;
    Ok(response.json::<WorkerClaimResponse>().await?.assignment)
}

async fn execute_assignment(
    client: &Client,
    config: &WorkerConfig,
    assignment: RemoteAssignment,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::new(&config.handler)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let input = serde_json::to_vec(&HandlerInput {
        assignment: &assignment,
    })?;
    child
        .stdin
        .take()
        .ok_or("worker handler stdin is unavailable")?
        .write_all(&input)
        .await?;
    let output = child.wait_with_output().await?;
    let handler = if output.status.success() {
        serde_json::from_slice::<HandlerOutput>(&output.stdout)?
    } else {
        HandlerOutput {
            events: vec![HandlerEvent {
                event_kind: "run.handler_failed".to_owned(),
                body: format!("{{\"exit_code\":{:?}}}", output.status.code()),
            }],
            terminal_status: "failed".to_owned(),
        }
    };
    if !matches!(
        handler.terminal_status.as_str(),
        "completed" | "failed" | "cancelled"
    ) {
        return Err("worker handler returned an invalid terminal status".into());
    }
    let mut sequence = assignment.next_event_sequence;
    for event in handler.events {
        post_event(
            client,
            config,
            &assignment,
            sequence,
            event.event_kind,
            event.body,
            None,
        )
        .await?;
        sequence += 1;
    }
    post_event(
        client,
        config,
        &assignment,
        sequence,
        format!("run.{}", handler.terminal_status),
        empty_json_object(),
        Some(handler.terminal_status),
    )
    .await?;
    Ok(())
}

async fn post_event(
    client: &Client,
    config: &WorkerConfig,
    assignment: &RemoteAssignment,
    sequence: u64,
    event_kind: String,
    body: String,
    terminal_status: Option<String>,
) -> Result<WorkerEventAck, Box<dyn std::error::Error>> {
    send_json(
        client,
        config,
        "/api/worker/events",
        &WorkerEventRequest {
            assignment_id: assignment.id,
            fencing_epoch: assignment.fencing_epoch,
            sequence,
            event_kind,
            body,
            terminal_status,
        },
    )
    .await
}

async fn send_json<I: Serialize, O: for<'de> Deserialize<'de>>(
    client: &Client,
    config: &WorkerConfig,
    path: &str,
    input: &I,
) -> Result<O, Box<dyn std::error::Error>> {
    let response = client
        .post(format!("{}{}", config.control_plane, path))
        .bearer_auth(&config.token)
        .json(input)
        .send()
        .await?;
    Ok(require_success(response).await?.json::<O>().await?)
}

async fn require_success(
    response: reqwest::Response,
) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let message = response.text().await.unwrap_or_default();
    if status == StatusCode::UNAUTHORIZED {
        return Err(format!("worker credential was rejected: {message}").into());
    }
    Err(format!("control plane returned {status}: {message}").into())
}

fn required_env(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    env::var(name)
        .map_err(|_| format!("{name} is required").into())
        .and_then(|value| {
            if value.trim().is_empty() {
                Err(format!("{name} cannot be empty").into())
            } else {
                Ok(value)
            }
        })
}

fn csv_env(name: &str) -> Vec<String> {
    env::var(name)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

fn empty_json_object() -> String {
    "{}".to_owned()
}
