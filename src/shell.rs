use std::io::Write;
use vte::Perform;

use crate::commands;
use crate::commands::CommandContext;

#[derive(Clone)]
pub struct ShellPerformer {
    pub(crate) ctx: CommandContext,
    pub(crate) line_buf: String,
    pub(crate) output: Vec<u8>,
    pub(crate) disconnect: bool,
    pub(crate) cursor_pos: usize,
}

impl ShellPerformer {
    pub fn new(ctx: CommandContext) -> Self {
        ShellPerformer {
            ctx,
            line_buf: String::new(),
            output: Vec::new(),
            disconnect: false,
            cursor_pos: 0,
        }
    }

    fn process_command(&mut self) {
        let cmd = self.line_buf.trim().to_string();
        self.line_buf.clear();
        self.cursor_pos = 0;

        let parts: Vec<&str> = cmd.split_whitespace().collect();

        if parts.is_empty() {
            self.output.extend_from_slice(b"$ ");
            return;
        }

        if let Some(result) = commands::execute(&parts, &self.ctx) {
            self.output.extend_from_slice(&result);
        } else {
            self.output.extend_from_slice(cmd.as_bytes());
        }
        self.output.extend_from_slice(b"\r\n$ ");
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
                self.output.extend_from_slice(b"^C\r\n$ ");
            }
            // Ctrl-D
            4 => {
                self.disconnect = true;
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
                }
            }
            _ => {}
        }
    }
}
