use agents::{
    ClaudeCodeProviderFactory, CodexProviderFactory, LocalProviderRegistry, MockProvider,
    OpenCodeProviderFactory, ProviderResolver,
};
use config::{ProviderKind, PublicBootstrap};
use control_plane::ControlPlaneStore;
use domain::ProviderSelection;
use git::GitService;
use orchestration::{DEFAULT_APPROVAL_TTL, LocalOrchestrator};
use persistence::{ArtifactPolicy, LocalArtifactStore, SqliteStore};
use review::ReviewOptions;
use server::{ProductServer, ProductServerConfig, StaticAssets};
use std::{
    collections::HashMap,
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::net::TcpListener;

#[derive(Debug)]
struct ApplicationConfig {
    bind: SocketAddr,
    data_dir: PathBuf,
    profile: String,
    public_origin: Option<String>,
    static_assets: PathBuf,
    development_seed: bool,
    development_password: String,
    provider: ProviderKind,
    provider_model: Option<String>,
    repository_roots: Vec<PathBuf>,
}

impl ApplicationConfig {
    fn load(profile_override: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let profile = profile_override.map(str::to_owned).unwrap_or_else(|| {
            env::var("JET_BLACK_PROFILE").unwrap_or_else(|_| "standalone".to_owned())
        });
        if !matches!(profile.as_str(), "standalone" | "server" | "desktop") {
            return Err(format!("unsupported Jet Black profile: {profile}").into());
        }
        let default_bind = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4317);
        let bind = match env::var("JET_BLACK_BIND") {
            Ok(value) => value.parse()?,
            Err(env::VarError::NotPresent) => default_bind,
            Err(error) => return Err(error.into()),
        };
        let data_dir = env::var_os("JET_BLACK_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(default_data_dir);
        let static_assets = env::var_os("JET_BLACK_STATIC_ASSETS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/web/build-client")
            });
        let public_origin = env::var("JET_BLACK_PUBLIC_ORIGIN").ok();
        let development_seed = env::var("JET_BLACK_DEV_SEED")
            .map(|value| matches!(value.as_str(), "1" | "true"))
            .unwrap_or(false);
        let development_password = env::var("JET_BLACK_DEV_PASSWORD")
            .unwrap_or_else(|_| "jet-black-development".to_owned());
        let provider = env::var("JET_BLACK_PROVIDER")
            .unwrap_or_else(|_| "mock".to_owned())
            .parse()
            .map_err(|_| "JET_BLACK_PROVIDER is invalid")?;
        let provider_model = env::var("JET_BLACK_PROVIDER_MODEL").ok();
        let repository_roots: Vec<PathBuf> = env::var("JET_BLACK_REPOSITORY_ROOTS")
            .map(|value| {
                value
                    .split(':')
                    .filter(|path| !path.is_empty())
                    .map(PathBuf::from)
                    .collect()
            })
            .unwrap_or_default();
        for root in &repository_roots {
            if !root.is_absolute() {
                return Err(format!("repository root must be absolute: {}", root.display()).into());
            }
        }
        Ok(Self {
            bind,
            data_dir,
            profile,
            public_origin,
            static_assets,
            development_seed,
            development_password,
            provider,
            provider_model,
            repository_roots,
        })
    }
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    run_with_profile(None).await
}

pub async fn run_desktop() -> Result<(), Box<dyn std::error::Error>> {
    run_with_profile(Some("desktop")).await
}

async fn run_with_profile(
    profile_override: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = ApplicationConfig::load(profile_override)?;
    std::fs::create_dir_all(&config.data_dir)?;
    let data_dir = std::fs::canonicalize(&config.data_dir)?;
    let database_path = data_dir.join("jet-black.sqlite3");
    let execution_store = SqliteStore::open(&database_path)?;
    let store = ControlPlaneStore::open(&database_path)?;
    let local_user_id = if config.profile == "desktop" {
        Some(store.ensure_local_user()?.id)
    } else {
        None
    };
    if config.development_seed {
        store.seed_development(&config.development_password)?;
    }

    let environment = env::vars().collect::<HashMap<_, _>>();
    let search_path = environment.get("PATH").cloned().unwrap_or_default();
    let provider_state = data_dir.join("providers");
    let default_provider = ProviderSelection::new(config.provider, config.provider_model.clone())?;
    let claude_code = ClaudeCodeProviderFactory::discover(
        &search_path,
        environment.get("ANTHROPIC_API_KEY").cloned(),
        &provider_state.join(ProviderKind::ClaudeCode.name()),
        Duration::from_secs(900),
    )
    .ok();
    let codex = CodexProviderFactory::discover(
        &search_path,
        environment.get("OPENAI_API_KEY").cloned(),
        &provider_state.join(ProviderKind::Codex.name()),
        Duration::from_secs(900),
    )
    .ok();
    let opencode = OpenCodeProviderFactory::discover(
        &search_path,
        &environment,
        &provider_state.join(ProviderKind::OpenCode.name()),
        Duration::from_secs(900),
    )
    .ok();
    let providers = LocalProviderRegistry::new(
        default_provider.clone(),
        Some(MockProvider::deterministic()),
        claude_code,
        codex,
        opencode,
    )?;
    let provider_availability = providers.available_kinds();
    let git = GitService::new(config.repository_roots.clone(), data_dir.join("worktrees"))?;
    let artifact_store = LocalArtifactStore::new(
        execution_store.clone(),
        data_dir.join("artifacts"),
        ArtifactPolicy::default(),
    )?;
    let runtime = Arc::new(
        LocalOrchestrator::new(execution_store, git, providers, DEFAULT_APPROVAL_TTL)
            .with_approved_repositories(config.repository_roots.clone())?
            .with_artifact_store(artifact_store)
            .with_review_options(ReviewOptions::default(), &search_path)?,
    );
    let recovery = runtime.recover()?;
    let execution_bootstrap = PublicBootstrap {
        profile: config.profile.clone(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        protocol_version: protocol::PROTOCOL_VERSION.to_owned(),
        enabled_features: vec![
            "product-control-plane".to_owned(),
            "local-execution".to_owned(),
            "local-review".to_owned(),
        ],
        provider_availability,
        default_provider,
    };

    let listener = TcpListener::bind(config.bind).await?;
    let address = listener.local_addr()?;
    let public_origin = config
        .public_origin
        .unwrap_or_else(|| format!("http://{address}"));
    let static_assets = StaticAssets::open(&config.static_assets)?;
    let server = ProductServer::new(
        ProductServerConfig {
            address,
            public_origin: public_origin.clone(),
            profile: config.profile.clone(),
            local_user_id,
        },
        store,
    )?
    .with_execution(execution_bootstrap, runtime)
    .with_static_assets(static_assets);

    println!(
        "Jet Black {} control plane ready at {}; data: {}",
        config.profile,
        public_origin,
        data_dir.display()
    );
    println!(
        "Execution recovery: {} recoverable, {} interrupted, {} failed, {} actions",
        recovery.recoverable.len(),
        recovery.interrupted.len(),
        recovery.failed.len(),
        recovery.actions.len()
    );
    if config.development_seed {
        println!("Development user: dev@jet-black.local");
    }
    server.serve(listener).await?;
    Ok(())
}

fn default_data_dir() -> PathBuf {
    env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
        })
        .unwrap_or_else(|| PathBuf::from("."))
        .join("jet-black")
}
