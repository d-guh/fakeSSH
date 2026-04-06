pub mod auth;
pub mod server;
pub mod shell;

use std::sync::Arc;

use russh::keys::ssh_key::rand_core::OsRng;
use russh::keys::{Algorithm, PrivateKey};
use russh::server::Server as _;
use tokio::net::TcpListener;

use auth::load_credentials;
use server::Server;

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
        keys: vec![PrivateKey::random(&mut OsRng, Algorithm::Ed25519)?],
        ..Default::default()
    };
    let config = Arc::new(config);
    let mut sh = Server::new(credentials);

    let socket = TcpListener::bind(("0.0.0.0", 2222)).await?;
    log::info!("Honeypot listening on 0.0.0.0:2222");
    sh.run_on_socket(config, &socket).await?;
    Ok(())
}
