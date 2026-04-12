use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use vte::Perform;

use crate::commands;
use crate::commands::CommandContext;

pub struct CommandRun {
    pub output: Vec<u8>,
    pub disconnect: bool,
    pub exit_status: u32,
}

#[derive(Clone)]
pub struct ShellPerformer {
    pub(crate) ctx: CommandContext,
    pub(crate) line_buf: String,
    pub(crate) output: Vec<u8>,
    pub(crate) disconnect: bool,
    pub(crate) cursor_pos: usize,
    pub(crate) history: Vec<String>,
    pub(crate) history_index: Option<usize>,
    pub(crate) history_stash: String,
    pub(crate) peer_ip: Option<String>,
    pub(crate) ip_log_file: Option<Arc<PathBuf>>,
}

impl ShellPerformer {
    pub fn new(ctx: CommandContext) -> Self {
        ShellPerformer {
            ctx,
            line_buf: String::new(),
            output: Vec::new(),
            disconnect: false,
            cursor_pos: 0,
            history: Vec::new(),
            history_index: None,
            history_stash: String::new(),
            peer_ip: None,
            ip_log_file: None,
        }
    }

    pub fn set_logging_context(&mut self, peer_ip: Option<String>, ip_log_file: Arc<PathBuf>) {
        self.peer_ip = peer_ip;
        self.ip_log_file = Some(ip_log_file);
    }

    fn process_command(&mut self) {
        let cmd = self.line_buf.trim().to_string();
        if !cmd.is_empty() {
            self.history.push(cmd.clone());
        }

        self.line_buf.clear();
        self.cursor_pos = 0;
        self.history_index = None;
        self.history_stash.clear();

        let result = run_command_line(&mut self.ctx, &cmd, true);
        if !cmd.is_empty() {
            self.log_command("interactive", &cmd, result.exit_status);
        }
        if result.output.is_empty() && cmd.is_empty() {
            self.output.extend_from_slice(self.ctx.prompt().as_bytes());
            return;
        }

        self.output.extend_from_slice(&result.output);
        if result.disconnect {
            self.disconnect = true;
            return;
        }
        self.output.extend_from_slice(self.ctx.prompt().as_bytes());
    }

    fn log_command(&self, mode: &str, command: &str, exit_status: u32) {
        let peer_ip = self.peer_ip.as_deref().unwrap_or("unknown");
        let command = escape_log_value(command);
        log::info!(
            "Command from {} as '{}' via {}: {:?} (exit {})",
            peer_ip,
            self.ctx.username,
            mode,
            command,
            exit_status
        );

        let Some(ip_log_file) = &self.ip_log_file else {
            return;
        };

        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let line = format!(
            "{timestamp} event=command ip={} user={} mode={} exit_status={} command=\"{}\"\n",
            peer_ip, self.ctx.username, mode, exit_status, command
        );

        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(ip_log_file.as_ref())
        {
            Ok(mut file) => {
                if let Err(err) = file.write_all(line.as_bytes()) {
                    log::error!("Failed writing command log entry: {err}");
                }
            }
            Err(err) => {
                log::error!("Failed opening IP log file {:?}: {err}", ip_log_file);
            }
        }
    }

    fn clear_screen(&mut self) {
        self.output.extend_from_slice(b"\x1b[2J\x1b[H");
        self.output.extend_from_slice(self.ctx.prompt().as_bytes());
    }

    fn set_line(&mut self, new_line: String) {
        self.output.extend_from_slice(b"\r");
        self.output.extend_from_slice(b"\x1b[2K");
        self.output.extend_from_slice(self.ctx.prompt().as_bytes());
        self.output.extend_from_slice(new_line.as_bytes());
        self.line_buf = new_line;
        self.cursor_pos = self.line_buf.chars().count();
    }

    fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }

        match self.history_index {
            None => {
                self.history_stash = self.line_buf.clone();
                self.history_index = Some(self.history.len() - 1);
            }
            Some(0) => {}
            Some(idx) => {
                self.history_index = Some(idx - 1);
            }
        }

        if let Some(idx) = self.history_index {
            self.set_line(self.history[idx].clone());
        }
    }

    fn history_down(&mut self) {
        let Some(idx) = self.history_index else {
            return;
        };

        if idx + 1 < self.history.len() {
            self.history_index = Some(idx + 1);
            self.set_line(self.history[idx + 1].clone());
        } else {
            self.history_index = None;
            self.set_line(self.history_stash.clone());
        }
    }

    fn complete_current_token(&mut self) {
        let cursor_byte = self
            .line_buf
            .char_indices()
            .nth(self.cursor_pos)
            .map(|(i, _)| i)
            .unwrap_or(self.line_buf.len());

        let left = &self.line_buf[..cursor_byte];
        let token_start = left
            .rfind(char::is_whitespace)
            .map(|idx| idx + 1)
            .unwrap_or(0);
        let token = &self.line_buf[token_start..cursor_byte];
        if token.is_empty() && token_start != 0 {
            return;
        }

        let is_first_token = token_start == 0 && !left[..token_start].contains(char::is_whitespace);
        let matches = if is_first_token {
            commands::shell_word_names()
                .into_iter()
                .filter(|name| name.starts_with(token))
                .map(|name| name.to_string())
                .collect::<Vec<_>>()
        } else {
            self.ctx.fs.complete_in_dir(&self.ctx.cwd, token)
        };

        if matches.is_empty() {
            return;
        }

        let replacement = longest_common_prefix(&matches);
        if replacement.len() > token.len() {
            let mut new_line = self.line_buf.clone();
            new_line.replace_range(token_start..cursor_byte, &replacement);
            self.set_line(new_line);
        } else if matches.len() == 1 {
            let mut completed = matches[0].clone();
            if is_first_token || !completed.ends_with('/') {
                completed.push(' ');
            }

            let mut new_line = self.line_buf.clone();
            new_line.replace_range(token_start..cursor_byte, &completed);
            self.set_line(new_line);
        }
    }
}

pub fn run_command_line(
    ctx: &mut CommandContext,
    line: &str,
    append_trailing_newline: bool,
) -> CommandRun {
    let cmd = line.trim();
    let raw_parts: Vec<&str> = cmd.split_whitespace().collect();

    if raw_parts.is_empty() {
        return CommandRun {
            output: Vec::new(),
            disconnect: false,
            exit_status: 0,
        };
    }

    let expanded_parts = commands::expand_alias(&raw_parts);
    let parts = expanded_parts
        .iter()
        .map(|part| part.as_str())
        .collect::<Vec<_>>();

    let (mut output, exit_status) = if let Some(result) = commands::execute(&parts, ctx) {
        (result, 0)
    } else {
        (
            format!("-bash: {}: command not found", raw_parts[0]).into_bytes(),
            127,
        )
    };

    if append_trailing_newline && !output.is_empty() {
        output.extend_from_slice(b"\r\n");
    }

    CommandRun {
        output,
        disconnect: ctx.should_exit,
        exit_status,
    }
}

impl Perform for ShellPerformer {
    fn print(&mut self, c: char) {
        // Get byte index for insertion point
        let byte_idx = self
            .line_buf
            .char_indices()
            .nth(self.cursor_pos)
            .map(|(i, _)| i)
            .unwrap_or(self.line_buf.len());

        self.line_buf.insert(byte_idx, c);
        self.cursor_pos += 1;
        self.history_index = None;

        // Echo the inserted char
        let mut buf = [0u8; 4];
        self.output
            .extend_from_slice(c.encode_utf8(&mut buf).as_bytes());

        // Reprint the tail after the insertion point
        let tail: String = self.line_buf[byte_idx + c.len_utf8()..].to_string();
        if !tail.is_empty() {
            self.output.extend_from_slice(tail.as_bytes());
            let n = tail.chars().count();
            write!(self.output, "\x1b[{}D", n).unwrap();
        }
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            // Ctrl-C
            3 => {
                self.line_buf.clear();
                self.cursor_pos = 0;
                self.output.extend_from_slice(b"^C\r\n");
                self.output.extend_from_slice(self.ctx.prompt().as_bytes());
            }
            // Ctrl-D
            4 => {
                self.disconnect = true;
            }
            // Ctrl-L
            12 => {
                self.line_buf.clear();
                self.cursor_pos = 0;
                self.clear_screen();
            }
            // Backspace (^H) — 0x7F is mapped to 0x08 before advancing the parser
            8 => {
                if self.cursor_pos == 0 {
                    return;
                }
                self.cursor_pos -= 1;
                let byte_idx = self
                    .line_buf
                    .char_indices()
                    .nth(self.cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(self.line_buf.len());
                self.line_buf.remove(byte_idx);

                // Move cursor left one
                self.output.extend_from_slice(b"\x08");
                // Reprint the tail
                let tail: String = self.line_buf[byte_idx..].to_string();
                self.output.extend_from_slice(tail.as_bytes());
                // Erase the last character
                self.output.extend_from_slice(b" ");
                // Move cursor back by tail length + 1 (for the erased char)
                let n = tail.chars().count() + 1;
                write!(self.output, "\x1b[{}D", n).unwrap();
                self.history_index = None;
            }
            // Tab
            9 => {
                self.complete_current_token();
            }
            // Return
            13 => {
                self.output.extend_from_slice(b"\r\n");
                self.process_command();
            }
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        match action {
            // Up arrow
            'A' => {
                self.history_up();
            }
            // Down arrow
            'B' => {
                self.history_down();
            }
            // Left arrow
            'D' => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.output.extend_from_slice(b"\x1b[D");
                }
            }
            // Right arrow
            'C' => {
                if self.cursor_pos < self.line_buf.chars().count() {
                    self.cursor_pos += 1;
                    self.output.extend_from_slice(b"\x1b[C");
                }
            }
            // Forward delete (\x1b[3~)
            '~' => {
                let first_param = params.iter().next().and_then(|p| p.first().copied());
                if first_param == Some(3) && self.cursor_pos < self.line_buf.chars().count() {
                    let byte_idx = self
                        .line_buf
                        .char_indices()
                        .nth(self.cursor_pos)
                        .map(|(i, _)| i)
                        .unwrap_or(self.line_buf.len());
                    self.line_buf.remove(byte_idx);

                    // Reprint the tail
                    let tail: String = self.line_buf[byte_idx..].to_string();
                    self.output.extend_from_slice(tail.as_bytes());
                    // Erase the last character
                    self.output.extend_from_slice(b" ");
                    // Move cursor back by tail length + 1
                    let n = tail.chars().count() + 1;
                    write!(self.output, "\x1b[{}D", n).unwrap();
                    self.history_index = None;
                }
            }
            _ => {}
        }
    }
}

fn longest_common_prefix(values: &[String]) -> String {
    let Some(first) = values.first() else {
        return String::new();
    };

    let mut prefix = first.clone();
    for value in &values[1..] {
        let shared_len = prefix
            .chars()
            .zip(value.chars())
            .take_while(|(a, b)| a == b)
            .count();
        prefix = prefix.chars().take(shared_len).collect();
        if prefix.is_empty() {
            break;
        }
    }

    prefix
}

fn escape_log_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}
