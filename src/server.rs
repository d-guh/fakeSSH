use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use russh::keys::ssh_key;
use russh::server::{Auth, Handler, Msg, Server as RusshServer, Session};
use russh::{Channel, ChannelId, MethodKind, MethodSet, Pty};
use tokio::time::sleep;

use crate::commands::CommandContext;
use crate::shell::{ShellPerformer, run_command_line};

pub struct Server {
    id: usize,
    credentials: Arc<HashMap<String, String>>,
    ip_log_file: Arc<PathBuf>,
    peer_addr: Option<SocketAddr>,
    failed_password_attempts: usize,
    disconnect_logged: bool,
    performer: ShellPerformer,
    vte_parser: vte::Parser,
}

const MAX_PASSWORD_ATTEMPTS: usize = 3;
const FAILED_PASSWORD_DELAY_SECS: u64 = 4;
//const PASSWORD_ONLY_METHODS: &[MethodKind] = &[MethodKind::Password];
const ADVERTISED_AUTH_METHODS: &[MethodKind] = &[MethodKind::PublicKey, MethodKind::Password];

impl Server {
    pub fn new(
        credentials: Arc<HashMap<String, String>>,
        ctx: CommandContext,
        ip_log_file: Arc<PathBuf>,
    ) -> Self {
        Server {
            id: 0,
            credentials,
            ip_log_file,
            peer_addr: None,
            failed_password_attempts: 0,
            disconnect_logged: false,
            performer: ShellPerformer::new(ctx),
            vte_parser: vte::Parser::new(),
        }
    }

    fn log_ip_event(&self, event: &str, user: Option<&str>) {
        self.log_ip_event_with_details(event, user, &[]);
    }

    fn log_ip_event_with_details(
        &self,
        event: &str,
        user: Option<&str>,
        details: &[(&str, String)],
    ) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let ip = self
            .peer_addr
            .map(|addr| addr.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let user = user.unwrap_or("-");
        let detail_suffix = if details.is_empty() {
            String::new()
        } else {
            details
                .iter()
                .map(|(key, value)| format!(r#" {}="{}""#, key, escape_log_value(value)))
                .collect::<String>()
        };
        let line = format!("{timestamp} event={event} ip={ip} user={user}{detail_suffix}\n");

        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.ip_log_file.as_ref())
        {
            Ok(mut file) => {
                if let Err(err) = file.write_all(line.as_bytes()) {
                    log::error!("Failed writing IP log entry: {err}");
                }
            }
            Err(err) => {
                log::error!("Failed opening IP log file {:?}: {err}", self.ip_log_file);
            }
        }
    }

    fn peer_ip(&self) -> String {
        self.peer_addr
            .map(|addr| addr.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn log_command_event(&self, mode: &str, command: &str, exit_status: u32) {
        self.log_ip_event_with_details(
            "command",
            Some(&self.performer.ctx.username),
            &[
                ("mode", mode.to_string()),
                ("exit_status", exit_status.to_string()),
                ("command", command.to_string()),
            ],
        );
        log::info!(
            "Command from {} as '{}' via {}: {:?} (exit {})",
            self.peer_ip(),
            self.performer.ctx.username,
            mode,
            command,
            exit_status
        );
    }

    fn log_disconnect_event(&mut self, event: &str, reason: &str, mode: &str) {
        if self.disconnect_logged {
            return;
        }

        self.disconnect_logged = true;
        self.log_ip_event_with_details(
            event,
            Some(&self.performer.ctx.username),
            &[("mode", mode.to_string()), ("reason", reason.to_string())],
        );
        log::info!(
            "Session ended for {} as '{}' via {} ({})",
            self.peer_ip(),
            self.performer.ctx.username,
            mode,
            reason
        );
    }
}

impl Clone for Server {
    fn clone(&self) -> Self {
        Server {
            id: self.id,
            credentials: Arc::clone(&self.credentials),
            ip_log_file: Arc::clone(&self.ip_log_file),
            peer_addr: self.peer_addr,
            failed_password_attempts: self.failed_password_attempts,
            disconnect_logged: self.disconnect_logged,
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
        s.failed_password_attempts = 0;
        s.disconnect_logged = false;
        s.performer.set_logging_context(
            s.peer_addr.map(|addr| addr.ip().to_string()),
            Arc::clone(&s.ip_log_file),
        );
        s.log_ip_event("connect", None);
        log::info!("Connection opened from {}", s.peer_ip());
        self.id += 1;
        s
    }

    fn handle_session_error(&mut self, error: <Self::Handler as russh::server::Handler>::Error) {
        match error {
            russh::Error::IO(err) if err.kind() == ErrorKind::UnexpectedEof => {
                if self.peer_addr.is_some() && !self.disconnect_logged {
                    self.log_disconnect_event("disconnect", "client_closed", "session");
                }
            }
            russh::Error::Disconnect => {
                if self.peer_addr.is_some() && !self.disconnect_logged {
                    self.log_disconnect_event("disconnect", "session_disconnect", "session");
                }
            }
            other => {
                log::warn!("Session error from {}: {other:#?}", self.peer_ip());
            }
        }
    }
}

impl Handler for Server {
    type Error = russh::Error;

    async fn auth_none(&mut self, user: &str) -> Result<Auth, Self::Error> {
        log::debug!(
            "Auth method probe from {} for user '{}' -> password required",
            self.peer_ip(),
            user
        );
        Ok(Auth::Reject {
            proceed_with_methods: Some(MethodSet::from(ADVERTISED_AUTH_METHODS)),
            partial_success: false,
        })
    }

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        match self.credentials.get(user) {
            Some(stored) if stored == password => {
                self.failed_password_attempts = 0;
                self.performer.ctx.username = user.to_string();
                self.performer.ctx.login_time =
                    chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
                self.log_ip_event("auth_success", Some(user));
                log::info!("Accepted login for user '{}' from {}", user, self.peer_ip());
                Ok(Auth::Accept)
            }
            _ => {
                self.failed_password_attempts += 1;
                let attempts_left =
                    MAX_PASSWORD_ATTEMPTS.saturating_sub(self.failed_password_attempts);
                self.log_ip_event("auth_failure", Some(user));
                log::info!(
                    "Rejected login for user '{}' from {} (attempt {}/{}, {} remaining)",
                    user,
                    self.peer_ip(),
                    self.failed_password_attempts,
                    MAX_PASSWORD_ATTEMPTS,
                    attempts_left
                );
                sleep(Duration::from_secs(FAILED_PASSWORD_DELAY_SECS)).await;

                Ok(Auth::Reject {
                    proceed_with_methods: Some(MethodSet::from(ADVERTISED_AUTH_METHODS)),
                    partial_success: false,
                })
            }
        }
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        _key: &ssh_key::PublicKey,
    ) -> Result<Auth, Self::Error> {
        log::debug!(
            "Rejected public key auth from {} for user '{}'",
            self.peer_ip(),
            user
        );
        Ok(Auth::Reject {
            proceed_with_methods: Some(MethodSet::from(ADVERTISED_AUTH_METHODS)),
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
        session.channel_success(channel)?;
        self.log_ip_event("shell_open", Some(&self.performer.ctx.username));
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

    async fn exec_request(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel)?;

        let command = String::from_utf8_lossy(data).trim().to_string();
        self.log_ip_event("exec_open", Some(&self.performer.ctx.username));

        let result = run_command_line(&mut self.performer.ctx, &command, true);
        self.log_command_event("exec", &command, result.exit_status);
        if !result.output.is_empty() {
            session.data(channel, result.output)?;
        }

        self.log_disconnect_event("disconnect", "exec_complete", "exec");
        session.exit_status_request(channel, result.exit_status)?;
        session.eof(channel)?;
        session.close(channel)?;
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
            let reason = if self.performer.ctx.should_exit {
                "shell_exit"
            } else {
                "ctrl_d"
            };
            self.log_disconnect_event("disconnect", reason, "interactive");
            return Err(russh::Error::Disconnect);
        }

        Ok(())
    }
}

fn escape_log_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}
