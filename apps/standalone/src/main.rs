use agents::{ClaudeCodeProvider, CodexProvider, LocalProvider, MockProvider, OpenCodeProvider};
use config::{CliOverrides, ProviderKind, StandaloneConfig};
use git::GitService;
use orchestration::{DEFAULT_APPROVAL_TTL, LocalOrchestrator};
use persistence::{ArtifactPolicy, LocalArtifactStore, SqliteStore};
use server::{StandaloneServer, StaticAssets};
use std::{collections::HashMap, sync::Arc};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let environment = std::env::vars().collect::<HashMap<_, _>>();
    let cli = CliOverrides::parse_from(std::env::args().skip(1))?;
    let config = StandaloneConfig::load(cli, &environment, None, None)?;
    let static_assets = StaticAssets::open(&config.static_assets_dir)?;
    config.prepare_directories()?;

    let search_path = environment.get("PATH").cloned().unwrap_or_default();
    let provider_state = config.data_dir.join("providers");
    let provider = match config.provider {
        ProviderKind::Mock => LocalProvider::Mock(MockProvider::deterministic()),
        ProviderKind::ClaudeCode => LocalProvider::ClaudeCode(ClaudeCodeProvider::discover(
            &search_path,
            environment.get("ANTHROPIC_API_KEY").cloned(),
            &provider_state.join(ProviderKind::ClaudeCode.name()),
            config.provider_model.clone(),
            config.run_timeout,
        )?),
        ProviderKind::Codex => LocalProvider::Codex(CodexProvider::discover(
            &search_path,
            environment.get("OPENAI_API_KEY").cloned(),
            &provider_state.join(ProviderKind::Codex.name()),
            config.provider_model.clone(),
            config.run_timeout,
        )?),
        ProviderKind::OpenCode => LocalProvider::OpenCode(OpenCodeProvider::discover(
            &search_path,
            &environment,
            &provider_state.join(ProviderKind::OpenCode.name()),
            config.provider_model.clone(),
            config.run_timeout,
        )?),
    };
    let provider_availability = vec![config.provider];

    let store = SqliteStore::open(&config.sqlite_path)?;
    let git = GitService::new(
        config.repository_roots.clone(),
        config.data_dir.join("worktrees"),
    )?;
    let artifact_store = LocalArtifactStore::new(
        store.clone(),
        config.log_dir.join("artifacts"),
        ArtifactPolicy::default(),
    )?;
    let runtime = Arc::new(
        LocalOrchestrator::new(store, git, provider, DEFAULT_APPROVAL_TTL)
            .with_artifact_store(artifact_store),
    );
    let listener = TcpListener::bind(config.bind).await?;
    let address = listener.local_addr()?;
    let recovery = runtime.recover()?;
    let server = StandaloneServer::new(
        address,
        config.public_bootstrap(provider_availability),
        runtime,
    )?
    .with_static_assets(static_assets);

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
