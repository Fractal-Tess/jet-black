use agents::MockProvider;
use config::{CliOverrides, StandaloneConfig};
use git::GitService;
use orchestration::{DEFAULT_APPROVAL_TTL, LocalOrchestrator};
use persistence::SqliteStore;
use server::StandaloneServer;
use std::{collections::HashMap, sync::Arc};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let environment = std::env::vars()
        .filter(|(key, _)| key.starts_with("JET_BLACK_"))
        .collect::<HashMap<_, _>>();
    let cli = CliOverrides::parse_from(std::env::args().skip(1))?;
    let config = StandaloneConfig::load(cli, &environment, None, None)?;
    config.prepare_directories()?;

    let store = SqliteStore::open(&config.sqlite_path)?;
    let git = GitService::new(
        config.repository_roots.clone(),
        config.data_dir.join("worktrees"),
    )?;
    let runtime = Arc::new(LocalOrchestrator::new(
        store,
        git,
        MockProvider::deterministic(),
        DEFAULT_APPROVAL_TTL,
    ));
    let listener = TcpListener::bind(config.bind).await?;
    let address = listener.local_addr()?;
    let recovery = runtime.recover()?;
    let server = StandaloneServer::new(address, config.public_bootstrap(), runtime)?;

    for action in &recovery.actions {
        println!(
            "recovery {} {}: {} ({})",
            action.aggregate_kind, action.aggregate_id, action.action, action.detail
        );
    }
    println!(
        "Jet Black standalone local core ready on {}; recovery: {} recoverable, {} interrupted, {} failed, {} terminal, {} actions",
        address,
        recovery.recoverable.len(),
        recovery.interrupted.len(),
        recovery.failed.len(),
        recovery.terminal.len(),
        recovery.actions.len()
    );
    println!(
        "Open http://{address}/#exchange={}",
        server.launch_token().expose()
    );
    server.serve(listener).await?;
    Ok(())
}
