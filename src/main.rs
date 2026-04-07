pub mod auth;
pub mod server;
pub mod shell;

use std::sync::Arc;

use russh::keys::ssh_key::LineEnding;
use russh::keys::ssh_key::rand_core::OsRng;
use russh::keys::{Algorithm, PrivateKey};
use russh::server::Server as _;
use tokio::net::TcpListener;

use auth::load_credentials;
use server::Server;

fn get_or_create_host_key(path: &str) -> Result<PrivateKey, Box<dyn std::error::Error>> {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let credentials = Arc::new(load_credentials("credentials.json")?);

    let config = russh::server::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
        auth_rejection_time: std::time::Duration::from_secs(3),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        keys: vec![get_or_create_host_key("host_key.pem")?],
        ..Default::default()
    };
    let config = Arc::new(config);
    let mut sh = Server::new(credentials);

    let listen = "0.0.0.0";
    let port = 2222;

    let socket = TcpListener::bind((listen, port)).await?;
    log::info!("Honeypot listening on {}:{}", listen, port);
    sh.run_on_socket(config, &socket).await?;
    Ok(())
}
