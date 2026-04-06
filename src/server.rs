use std::collections::HashMap;
use std::sync::Arc;

use russh::keys::ssh_key;
use russh::server::{Auth, Handler, Msg, Server as RusshServer, Session};
use russh::{Channel, ChannelId, MethodKind, MethodSet, Pty};

use crate::shell::ShellPerformer;

pub struct Server {
    id: usize,
    credentials: Arc<HashMap<String, String>>,
    performer: ShellPerformer,
    vte_parser: vte::Parser,
}

impl Server {
    pub fn new(credentials: Arc<HashMap<String, String>>) -> Self {
        Server {
            id: 0,
            credentials,
            performer: ShellPerformer::default(),
            vte_parser: vte::Parser::new(),
        }
    }
}

impl Clone for Server {
    fn clone(&self) -> Self {
        Server {
            id: self.id,
            credentials: Arc::clone(&self.credentials),
            performer: self.performer.clone(),
            // Each client session gets a fresh parser
            vte_parser: vte::Parser::new(),
        }
    }
}

impl RusshServer for Server {
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

impl Handler for Server {
    type Error = russh::Error;

    async fn auth_password(
        &mut self,
        user: &str,
        password: &str,
    ) -> Result<Auth, Self::Error> {
        match self.credentials.get(user) {
            Some(stored) if stored == password => {
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
        let banner = b"Welcome to Ubuntu 22.04.3 LTS\r\n\
                       Last login: Wed Jan 10 14:23:05 2024 from 192.168.1.1\r\n\
                       $ "
        .to_vec();
        session.data(channel, banner)?;
        Ok(())
    }

    // Parse data packet
    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.performer.output.clear();
        self.performer.disconnect = false;

        // Update parser state
        self.vte_parser.advance(&mut self.performer, data);

        if !self.performer.output.is_empty() {
            session.data(channel, self.performer.output.clone())?;
        }

        if self.performer.disconnect {
            return Err(russh::Error::Disconnect);
        }

        Ok(())
    }
}
