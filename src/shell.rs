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
}

impl ShellPerformer {
    fn process_command(&mut self) {
        let cmd = self.line_buf.trim().to_string();
        self.line_buf.clear();

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

                let listing = if flags.contains('l') { LS_LONG } else { LS_SHORT };
                self.output.extend_from_slice(listing.as_bytes());
                self.output.extend_from_slice(b"\r\n$ ");
            }
            _ => {
                self.output.extend_from_slice(cmd.as_bytes());
                self.output.extend_from_slice(b"\r\n$ ");
            }
        }
    }
}

impl Perform for ShellPerformer {
    fn print(&mut self, c: char) {
        self.line_buf.push(c);
        let mut buf = [0u8; 4];
        self.output
            .extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
    }
    
    ///Handle Control Characters
    fn execute(&mut self, byte: u8) {
        match byte {
            // Ctrl-C
            3 => {
                self.line_buf.clear();
                self.output.extend_from_slice(b"^C\r\n$ ");
            }
            // Ctrl-D
            4 => {
                self.disconnect = true;
            }
            // Backspace and Delete
            8 | 127 => {
                if self.line_buf.pop().is_some() {
                    self.output.extend_from_slice(b"\x08 \x08");
                }
            }
            // Return
            13 => {
                self.output.extend_from_slice(b"\r\n");
                self.process_command();
            }
            _ => {}
        }
    }
}
