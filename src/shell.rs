use std::io::Write;
use vte::Perform;

const LS_SHORT: &str = "\x1b[01;34mDesktop\x1b[0m  \x1b[01;34mDocuments\x1b[0m  \
    \x1b[01;34mDownloads\x1b[0m  \x1b[01;34mMusic\x1b[0m  \x1b[01;34mPictures\x1b[0m  \
    \x1b[01;34mPublic\x1b[0m  \x1b[01;34msnap\x1b[0m  \x1b[01;34mTemplates\x1b[0m  \
    \x1b[01;34mVideos\x1b[0m";

const LS_LONG: &str = "total 60\r\n\
    drwxr-x--- 11 ubuntu ubuntu 4096 Jan 10 14:23 \x1b[01;34m.\x1b[0m\r\n\
    drwxr-xr-x  3 root   root   4096 Jan  5 09:15 \x1b[01;34m..\x1b[0m\r\n\
    -rw-------  1 ubuntu ubuntu 1234 Jan 10 14:22 .bash_history\r\n\
    -rw-r--r--  1 ubuntu ubuntu  220 Jan  5 09:15 .bash_logout\r\n\
    -rw-r--r--  1 ubuntu ubuntu 3526 Jan  5 09:15 .bashrc\r\n\
    -rw-r--r--  1 ubuntu ubuntu  807 Jan  5 09:15 .profile\r\n\
    drwx------  2 ubuntu ubuntu 4096 Jan  5 09:15 \x1b[01;34m.ssh\x1b[0m\r\n\
    drwxr-xr-x  2 ubuntu ubuntu 4096 Jan  5 09:15 \x1b[01;34mDesktop\x1b[0m\r\n\
    drwxr-xr-x  2 ubuntu ubuntu 4096 Jan  5 09:15 \x1b[01;34mDocuments\x1b[0m\r\n\
    drwxr-xr-x  2 ubuntu ubuntu 4096 Jan  5 09:15 \x1b[01;34mDownloads\x1b[0m\r\n\
    drwxr-xr-x  2 ubuntu ubuntu 4096 Jan  5 09:15 \x1b[01;34mMusic\x1b[0m\r\n\
    drwxr-xr-x  2 ubuntu ubuntu 4096 Jan  5 09:15 \x1b[01;34mPictures\x1b[0m\r\n\
    drwxr-xr-x  2 ubuntu ubuntu 4096 Jan  5 09:15 \x1b[01;34mPublic\x1b[0m\r\n\
    drwxrwxr-x  3 ubuntu ubuntu 4096 Jan  8 10:15 \x1b[01;34msnap\x1b[0m\r\n\
    drwxr-xr-x  2 ubuntu ubuntu 4096 Jan  5 09:15 \x1b[01;34mTemplates\x1b[0m\r\n\
    drwxr-xr-x  2 ubuntu ubuntu 4096 Jan  5 09:15 \x1b[01;34mVideos\x1b[0m";

#[derive(Clone, Default)]
pub struct ShellPerformer {
    pub(crate) line_buf: String,
    pub(crate) output: Vec<u8>,
    pub(crate) disconnect: bool,
    pub(crate) cursor_pos: usize,
}

impl ShellPerformer {
    fn process_command(&mut self) {
        let cmd = self.line_buf.trim().to_string();
        self.line_buf.clear();
        self.cursor_pos = 0;

        let parts: Vec<&str> = cmd.split_whitespace().collect();

        match parts.first().copied() {
            None => {
                self.output.extend_from_slice(b"$ ");
            }
            Some("ls") => {
                let flags: String = parts[1..]
                    .iter()
                    .filter(|a| a.starts_with('-'))
                    .flat_map(|a| a.chars())
                    .collect();

                let listing = if flags.contains('l') {
                    LS_LONG
                } else {
                    LS_SHORT
                };
                self.output.extend_from_slice(listing.as_bytes());
            }
            Some("hostname") => {
                self.output.extend_from_slice(b"test-vm");
            }
            // Echo unknown commands back to user
            _ => {
                self.output.extend_from_slice(cmd.as_bytes());
            }
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
                if first_param == Some(3) {
                    if self.cursor_pos < self.line_buf.chars().count() {
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
            }
            _ => {}
        }
    }
}
