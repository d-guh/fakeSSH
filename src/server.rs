use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use russh::keys::ssh_key;
use russh::server::{Auth, Handler, Msg, Server as RusshServer, Session};
use russh::{Channel, ChannelId, MethodKind, MethodSet, Pty};

use crate::commands::CommandContext;
use crate::shell::ShellPerformer;

pub struct Server {
    id: usize,
    credentials: Arc<HashMap<String, String>>,
    peer_addr: Option<SocketAddr>,
    performer: ShellPerformer,
    vte_parser: vte::Parser,
}

impl Server {
    pub fn new(credentials: Arc<HashMap<String, String>>, ctx: CommandContext) -> Self {
        Server {
            id: 0,
            credentials,
            peer_addr: None,
            performer: ShellPerformer::new(ctx),
            vte_parser: vte::Parser::new(),
        }
    }
}

impl Clone for Server {
    fn clone(&self) -> Self {
        Server {
            id: self.id,
            credentials: Arc::clone(&self.credentials),
            peer_addr: self.peer_addr,
            performer: self.performer.clone(),
            vte_parser: vte::Parser::new(),
        }
    }
}

impl RusshServer for Server {
    type Handler = Self;

    fn new_client(&mut self, peer: Option<std::net::SocketAddr>) -> Self {
        let mut s = self.clone();
        s.peer_addr = peer;
        self.id += 1;
        s
    }

    fn handle_session_error(&mut self, error: <Self::Handler as russh::server::Handler>::Error) {
        log::error!("Session error: {error:#?}");
    }
}

impl Handler for Server {
    type Error = russh::Error;

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        match self.credentials.get(user) {
            Some(stored) if stored == password => {
                self.performer.ctx.username = user.to_string();
                log::info!("Accepted login for user '{user}'");
                Ok(Auth::Accept)
            }
            _ => {
                log::info!("Rejected login for user '{user}'");
                Ok(Auth::Reject {
                    proceed_with_methods: None,
                    partial_success: false,
                })
            }
        }
    }

    // For right now we reject public key auth, push to password auth
    async fn auth_publickey(
        &mut self,
        _user: &str,
        _key: &ssh_key::PublicKey,
    ) -> Result<Auth, Self::Error> {
        Ok(Auth::Reject {
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
        let timestamp = chrono::Local::now().format("%a %b %e %H:%M:%S %Y");
        let peer_ip = self
            .peer_addr
            .map(|a| a.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let banner = format!(
            "Welcome to Ubuntu 22.04.3 LTS\r\n\
             Last login: {timestamp} from {peer_ip}\r\n\
             {}",
            self.performer.ctx.prompt()
        );
        session.data(channel, banner.into_bytes())?;
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.performer.output.clear();
        self.performer.disconnect = false;

        // VTE is expecting 0x08 BS for backspace, however SSH emits 0x7F DEL for backspace, this will remap all 0x7F to 0x08
        let mapped: Vec<u8> = data
            .iter()
            .map(|&b| if b == 0x7f { 0x08 } else { b })
            .collect();
        self.vte_parser.advance(&mut self.performer, &mapped);

        if !self.performer.output.is_empty() {
            session.data(channel, self.performer.output.clone())?;
        }

        if self.performer.disconnect {
            return Err(russh::Error::Disconnect);
        }

        Ok(())
    }
}
