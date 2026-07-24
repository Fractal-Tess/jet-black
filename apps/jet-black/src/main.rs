use control_plane::ControlPlaneStore;
use server::{ProductServer, ProductServerConfig, StaticAssets};
use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
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
}

impl ApplicationConfig {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let profile = env::var("JET_BLACK_PROFILE").unwrap_or_else(|_| "standalone".to_owned());
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
        Ok(Self {
            bind,
            data_dir,
            profile,
            public_origin,
            static_assets,
            development_seed,
            development_password,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ApplicationConfig::load()?;
    std::fs::create_dir_all(&config.data_dir)?;
    let store = ControlPlaneStore::open(config.data_dir.join("jet-black.sqlite3"))?;
    if config.development_seed {
        store.seed_development(&config.development_password)?;
    }

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
        },
        store,
    )?
    .with_static_assets(static_assets);

    println!(
        "Jet Black {} control plane ready at {}; data: {}",
        config.profile,
        public_origin,
        config.data_dir.display()
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
