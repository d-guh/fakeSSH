use std::collections::HashMap;
use std::sync::Arc;

use russh::keys::ssh_key::rand_core::OsRng;
use russh::keys::*;
use russh::server::{Msg, Server as _, Session};
use russh::*;
use serde::Deserialize;
use tokio::net::TcpListener;

#[derive(Deserialize)]
struct CredentialsFile {
    users: HashMap<String, String>,
}

fn load_credentials(path: &str) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    let creds: CredentialsFile = serde_json::from_str(&contents)?;
    Ok(creds.users)
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
        keys: vec![PrivateKey::random(&mut OsRng, Algorithm::Ed25519)?],
        ..Default::default()
    };
    let config = Arc::new(config);
    let mut sh = Server {
        id: 0,
        credentials,
        line_buf: Vec::new(),
    };

    let socket = TcpListener::bind(("0.0.0.0", 2222)).await?;
    log::info!("Honeypot listening on 0.0.0.0:2222");
    sh.run_on_socket(config, &socket).await?;
    Ok(())
}

#[derive(Clone)]
struct Server {
    id: usize,
    credentials: Arc<HashMap<String, String>>,
    /// Per-session buffer for the current input line.
    line_buf: Vec<u8>,
}

impl server::Server for Server {
    type Handler = Self;

    fn new_client(&mut self, _peer: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }

    fn handle_session_error(&mut self, error: <Self::Handler as russh::server::Handler>::Error) {
        log::error!("Session error: {error:#?}");
    }
}

impl server::Handler for Server {
    type Error = russh::Error;

    async fn auth_password(
        &mut self,
        user: &str,
        password: &str,
    ) -> Result<server::Auth, Self::Error> {
        match self.credentials.get(user) {
            Some(stored) if stored == password => {
                log::info!("Accepted login for user '{user}'");
                Ok(server::Auth::Accept)
            }
            _ => {
                log::info!("Rejected login for user '{user}'");
                Ok(server::Auth::Reject {
                    proceed_with_methods: None,
                    partial_success: false,
                })
            }
        }
    }

    async fn auth_publickey(
        &mut self,
        _user: &str,
        _key: &ssh_key::PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        // Redirect clients to password auth.
        Ok(server::Auth::Reject {
            proceed_with_methods: Some(MethodSet::from(&[MethodKind::Password][..])),
            partial_success: false,
        })
    }

    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    async fn pty_request(
        &mut self,
        _channel: ChannelId,
        _term: &str,
        _col_width: u32,
        _row_height: u32,
        _pix_width: u32,
        _pix_height: u32,
        _modes: &[(Pty, u32)],
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let banner = b"Welcome to Ubuntu 22.04.3 LTS\r\n\
                       Last login: Wed Jan 10 14:23:05 2024 from 192.168.1.1\r\n\
                       $ "
        .to_vec();
        session.data(channel, banner)?;
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        for &byte in data {
            match byte {
                3 => {
                    // Ctrl+C: clear line, show new prompt.
                    self.line_buf.clear();
                    session.data(channel, b"^C\r\n$ ".to_vec())?;
                }
                4 => {
                    // Ctrl+D: disconnect.
                    return Err(russh::Error::Disconnect);
                }
                13 | 10 => {
                    // Enter: echo the buffered line back, then show prompt.
                    let cmd = String::from_utf8_lossy(&self.line_buf).to_string();
                    self.line_buf.clear();
                    let response = format!("\r\n{cmd}\r\n$ ");
                    session.data(channel, response.into_bytes())?;
                }
                127 | 8 => {
                    // Backspace / DEL: erase last character.
                    if !self.line_buf.is_empty() {
                        self.line_buf.pop();
                        session.data(channel, b"\x08 \x08".to_vec())?;
                    }
                }
                byte => {
                    // Printable character: buffer and echo.
                    self.line_buf.push(byte);
                    session.data(channel, vec![byte])?;
                }
            }
        }
        Ok(())
    }
}
