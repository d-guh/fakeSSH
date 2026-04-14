pub mod commands;
pub mod server;
pub mod shell;

use commands::CommandContext;
use russh::MethodKind;
use russh::MethodSet;
use russh::Preferred;
use russh::keys::ssh_key::HashAlg;
use russh::keys::ssh_key::LineEnding;
use russh::keys::ssh_key::rand_core::OsRng;
use russh::keys::{Algorithm, PrivateKey};
use russh::server::Server as _;
use serde::Deserialize;
use server::{MAX_TOTAL_AUTH_ATTEMPTS, Server};
use std::borrow::Cow;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let cfg = Config::new("config.toml")?;

    let credentials = Arc::new(cfg.credentials);
    let ctx = CommandContext::new(cfg.server.hostname.clone());
    let ip_log_file = Arc::new(PathBuf::from(&cfg.server.ip_log_file));

    let config = russh::server::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(
            cfg.server.inactivity_timeout_secs,
        )),
        nodelay: true,
        methods: MethodSet::from(&[MethodKind::Password][..]),
        max_auth_attempts: MAX_TOTAL_AUTH_ATTEMPTS,
        auth_rejection_time: std::time::Duration::from_secs(cfg.server.auth_rejection_time_secs),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(
            cfg.server.auth_rejection_time_initial_secs,
        )),
        preferred: preferred_algorithms(),
        keys: load_host_keys(&cfg.server)?,
        ..Default::default()
    };
    let config = Arc::new(config);
    let mut sh = Server::new(credentials, ctx, ip_log_file);

    let socket = TcpListener::bind((&*cfg.server.listen, cfg.server.port))
        .await
        .map_err(|err| bind_error(&cfg.server.listen, cfg.server.port, err))?;
    log::info!(
        "Honeypot listening on {}:{}",
        cfg.server.listen,
        cfg.server.port
    );
    sh.run_on_socket(config, &socket).await?;
    Ok(())
}

fn bind_error(listen: &str, port: u16, err: io::Error) -> io::Error {
    let message = match err.kind() {
        io::ErrorKind::PermissionDenied => format!(
            "Failed to bind {}:{}: permission denied. Try a port above 1024 or run with elevated privileges. OS error: {}",
            listen, port, err
        ),
        io::ErrorKind::AddrInUse => format!(
            "Failed to bind {}:{}: address already in use. Another process is already listening on that port. OS error: {}",
            listen, port, err
        ),
        _ => format!("Failed to bind {}:{}: {}", listen, port, err),
    };

    io::Error::new(err.kind(), message)
}

pub fn get_or_create_host_key(
    path: &str,
    algorithm: Algorithm,
) -> Result<PrivateKey, Box<dyn std::error::Error>> {
    let p = std::path::Path::new(path);
    if p.exists() {
        let pem = std::fs::read_to_string(p)?;
        Ok(PrivateKey::from_openssh(pem.as_bytes())?)
    } else {
        let key = PrivateKey::random(&mut OsRng, algorithm)?;
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

fn load_host_keys(server: &ServerConfig) -> Result<Vec<PrivateKey>, Box<dyn std::error::Error>> {
    let configured_paths = if server.host_key_files.is_empty() {
        vec![server.primary_host_key_file()?.to_string()]
    } else {
        server.host_key_files.clone()
    };

    let mut keys = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    for path in configured_paths {
        if seen_paths.insert(path.clone()) {
            keys.push(get_or_create_host_key(
                &path,
                infer_host_key_algorithm(&path),
            )?);
        }
    }

    let has_rsa_key = keys.iter().any(|key| key.algorithm().is_rsa());
    if !has_rsa_key {
        let primary_host_key = server.primary_host_key_file()?;
        let rsa_path = server
            .rsa_host_key_file
            .clone()
            .unwrap_or_else(|| derive_rsa_host_key_path(primary_host_key));

        if seen_paths.insert(rsa_path.clone()) {
            keys.push(get_or_create_host_key(
                &rsa_path,
                Algorithm::Rsa {
                    hash: Some(HashAlg::Sha512),
                },
            )?);
        }
    }

    Ok(keys)
}

fn infer_host_key_algorithm(path: &str) -> Algorithm {
    let lower = path.to_ascii_lowercase();
    if lower.contains("rsa") {
        Algorithm::Rsa {
            hash: Some(HashAlg::Sha512),
        }
    } else {
        Algorithm::Ed25519
    }
}

fn derive_rsa_host_key_path(primary_path: &str) -> String {
    let path = std::path::Path::new(primary_path);
    let parent = path.parent();
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("host_key");
    let extension = path.extension().and_then(|ext| ext.to_str());

    let file_name = match extension {
        Some(ext) if !ext.is_empty() => format!("{stem}_rsa.{ext}"),
        _ => format!("{stem}_rsa"),
    };

    parent
        .map(|parent| parent.join(&file_name))
        .unwrap_or_else(|| PathBuf::from(file_name))
        .to_string_lossy()
        .into_owned()
}

fn preferred_algorithms() -> Preferred {
    let mut preferred = Preferred::default();
    preferred.key = Cow::Owned(vec![
        Algorithm::Ed25519,
        Algorithm::Rsa {
            hash: Some(HashAlg::Sha512),
        },
        Algorithm::Rsa {
            hash: Some(HashAlg::Sha256),
        },
        Algorithm::Rsa { hash: None },
    ]);
    preferred
}

#[derive(Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub credentials: HashMap<String, String>,
}

#[derive(Deserialize, Clone)]
pub struct ServerConfig {
    pub hostname: String,
    pub listen: String,
    pub port: u16,
    pub host_key_file: Option<String>,
    #[serde(default)]
    pub host_key_files: Vec<String>,
    pub rsa_host_key_file: Option<String>,
    pub ip_log_file: String,
    pub inactivity_timeout_secs: u64,
    pub auth_rejection_time_secs: u64,
    pub auth_rejection_time_initial_secs: u64,
}

impl ServerConfig {
    fn primary_host_key_file(&self) -> Result<&str, Box<dyn std::error::Error>> {
        self.host_key_file
            .as_deref()
            .or_else(|| self.host_key_files.first().map(String::as_str))
            .ok_or_else(|| {
                "server.host_key_file or server.host_key_files must be configured".into()
            })
    }
}

impl Config {
    pub fn new(cfg_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(cfg_path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }
}
