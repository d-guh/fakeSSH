use super::{Args, Command, CommandContext};

const LS_SHORT: &str = "
    \x1b[01;34mDesktop\x1b[0m  \x1b[01;34mDocuments\x1b[0m  \
    \x1b[01;34mDownloads\x1b[0m  \x1b[01;34mMusic\x1b[0m  \x1b[01;34mPictures\x1b[0m  \
    \x1b[01;34mPublic\x1b[0m  \x1b[01;34msnap\x1b[0m  \x1b[01;34mTemplates\x1b[0m  \
    \x1b[01;34mVideos\x1b[0m";

const LS_LONG: &str = "
    total 60\r\n\
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

pub struct Ls;

impl Command for Ls {
    fn run(&self, args: &Args, _ctx: &CommandContext) -> Vec<u8> {
        let listing = if args.short_flags.contains(&'l') {
            LS_LONG
        } else {
            LS_SHORT
        };
        listing.as_bytes().to_vec()
    }
}
