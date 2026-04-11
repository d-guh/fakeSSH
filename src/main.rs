pub mod commands;
pub mod server;
pub mod shell;

use commands::CommandContext;
use russh::keys::ssh_key::LineEnding;
use russh::keys::ssh_key::rand_core::OsRng;
use russh::keys::{Algorithm, PrivateKey};
use russh::server::Server as _;
use serde::Deserialize;
use server::Server;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let cfg = Config::new("config.toml")?;

    let credentials = Arc::new(cfg.credentials);
    let ctx = CommandContext::new(cfg.server.hostname);
    let ip_log_file = Arc::new(PathBuf::from(&cfg.server.ip_log_file));

    let config = russh::server::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(
            cfg.server.inactivity_timeout_secs,
        )),
        auth_rejection_time: std::time::Duration::from_secs(cfg.server.auth_rejection_time_secs),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(
            cfg.server.auth_rejection_time_initial_secs,
        )),
        keys: vec![get_or_create_host_key(&cfg.server.host_key_file)?],
        ..Default::default()
    };
    let config = Arc::new(config);
    let mut sh = Server::new(credentials, ctx, ip_log_file);

    let socket = TcpListener::bind((&*cfg.server.listen, cfg.server.port)).await?;
    log::info!(
        "Honeypot listening on {}:{}",
        cfg.server.listen,
        cfg.server.port
    );
    sh.run_on_socket(config, &socket).await?;
    Ok(())
}

pub fn get_or_create_host_key(path: &str) -> Result<PrivateKey, Box<dyn std::error::Error>> {
    let p = std::path::Path::new(path);
    if p.exists() {
        let pem = std::fs::read_to_string(p)?;
        Ok(PrivateKey::from_openssh(pem.as_bytes())?)
    } else {
        let key = PrivateKey::random(&mut OsRng, Algorithm::Ed25519)?;
        let pem = key.to_openssh(LineEnding::LF)?;
        std::fs::write(p, pem.as_bytes())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o600))?;
        }
        log::info!("Generated new host key, saved to {path}");
        Ok(key)
    }
}

#[derive(Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub credentials: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub hostname: String,
    pub listen: String,
    pub port: u16,
    pub host_key_file: String,
    pub ip_log_file: String,
    pub inactivity_timeout_secs: u64,
    pub auth_rejection_time_secs: u64,
    pub auth_rejection_time_initial_secs: u64,
}

impl Config {
    pub fn new(cfg_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(cfg_path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }
}
