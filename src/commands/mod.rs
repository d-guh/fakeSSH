mod cd;
mod clear;
mod filesystem;
mod hostname;
mod id;
mod ls;
mod pwd;
mod uname;
mod w;
mod who;
mod whoami;

use std::collections::{BTreeMap, HashSet};

use filesystem::{DirEntry, DirItem, FakeFileSystem, FileEntry};

#[derive(Clone)]
pub struct CommandContext {
    pub username: String,
    pub hostname: String,
    pub login_time: String,
    pub fs: FakeFileSystem,
    pub cwd: Vec<String>,
}

pub struct Args {
    pub short_flags: HashSet<char>,
    pub long_flags: HashSet<String>,
    pub positional: Vec<String>,
}

impl Args {
    pub fn parse(parts: &[&str]) -> Self {
        let mut short_flags = HashSet::new();
        let mut long_flags = HashSet::new();
        let mut positional = Vec::new();
        let mut flags_done = false;

        for &part in parts {
            if flags_done {
                positional.push(part.to_string());
            } else if part == "--" {
                flags_done = true;
            } else if let Some(long) = part.strip_prefix("--") {
                long_flags.insert(long.to_string());
            } else if let Some(short) = part.strip_prefix('-') {
                for c in short.chars() {
                    short_flags.insert(c);
                }
            } else {
                positional.push(part.to_string());
            }
        }

        Args {
            short_flags,
            long_flags,
            positional,
        }
    }
}

pub trait Command {
    fn run(&self, args: &Args, ctx: &mut CommandContext) -> Vec<u8>;
}

pub fn execute(parts: &[&str], ctx: &mut CommandContext) -> Option<Vec<u8>> {
    let cmd: &dyn Command = match parts.first().copied() {
        Some("clear") => &clear::Clear,
        Some("ls") => &ls::Ls,
        Some("cd") => &cd::Cd,
        Some("pwd") => &pwd::Pwd,
        Some("whoami") => &whoami::Whoami,
        Some("who") => &who::Who,
        Some("w") => &w::W,
        Some("id") => &id::Id,
        Some("hostname") => &hostname::Hostname,
        Some("uname") => &uname::Uname,
        _ => return None,
    };

    let args = Args::parse(&parts[1..]);
    Some(cmd.run(&args, ctx))
}

impl CommandContext {
    pub fn new(hostname: String) -> Self {
        let fs = default_filesystem();
        let cwd = vec!["home".to_string(), "ubuntu".to_string()];
        CommandContext {
            username: "ubuntu".to_string(),
            hostname,
            login_time: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
            fs,
            cwd,
        }
    }

    pub fn pwd(&self) -> String {
        self.fs.display_path(&self.cwd)
    }

    pub fn prompt(&self) -> String {
        format!(
            "\x1b[01;32m{}\x1b[0m@\x1b[01;32m{}\x1b[0m:\x1b[01;34m{}\x1b[0m$ ",
            self.username,
            self.hostname,
            self.fs.prompt_path(&self.cwd)
        )
    }

    pub fn user_profile(&self) -> UserProfile {
        match self.username.as_str() {
            "root" => UserProfile {
                uid: 0,
                gid: 0,
                group_name: "root",
                groups: &[(0, "root")],
            },
            "admin" => UserProfile {
                uid: 1001,
                gid: 1001,
                group_name: "admin",
                groups: &[(1001, "admin"), (27, "sudo"), (100, "users")],
            },
            _ => UserProfile {
                uid: 1000,
                gid: 1000,
                group_name: "ubuntu",
                groups: &[(1000, "ubuntu"), (27, "sudo"), (100, "users")],
            },
        }
    }
}

pub struct UserProfile {
    pub uid: u32,
    pub gid: u32,
    pub group_name: &'static str,
    pub groups: &'static [(u32, &'static str)],
}

fn default_filesystem() -> FakeFileSystem {
    let home_dirs = [
        ("Desktop", "Jan  5 09:15"),
        ("Documents", "Jan  5 09:15"),
        ("Downloads", "Jan  5 09:15"),
        ("Music", "Jan  5 09:15"),
        ("Pictures", "Jan  5 09:15"),
        ("Public", "Jan  5 09:15"),
        ("Templates", "Jan  5 09:15"),
        ("Videos", "Jan  5 09:15"),
    ]
    .into_iter()
    .map(|(name, modified)| {
        (
            name.to_string(),
            DirItem::Dir(DirEntry::new(
                "drwxr-xr-x",
                "ubuntu",
                "ubuntu",
                4096,
                modified,
            )),
        )
    });

    let mut ubuntu_children = BTreeMap::from([
        (
            ".bash_history".to_string(),
            DirItem::File(FileEntry::new(
                "-rw-------",
                "ubuntu",
                "ubuntu",
                1234,
                "Jan 10 14:22",
            )),
        ),
        (
            ".bash_logout".to_string(),
            DirItem::File(FileEntry::new(
                "-rw-r--r--",
                "ubuntu",
                "ubuntu",
                220,
                "Jan  5 09:15",
            )),
        ),
        (
            ".bashrc".to_string(),
            DirItem::File(FileEntry::new(
                "-rw-r--r--",
                "ubuntu",
                "ubuntu",
                3526,
                "Jan  5 09:15",
            )),
        ),
        (
            ".profile".to_string(),
            DirItem::File(FileEntry::new(
                "-rw-r--r--",
                "ubuntu",
                "ubuntu",
                807,
                "Jan  5 09:15",
            )),
        ),
        (
            ".ssh".to_string(),
            DirItem::Dir(
                DirEntry::new("drwx------", "ubuntu", "ubuntu", 4096, "Jan  5 09:15")
                    .with_children(BTreeMap::from([
                        (
                            "authorized_keys".to_string(),
                            DirItem::File(FileEntry::new(
                                "-rw-------",
                                "ubuntu",
                                "ubuntu",
                                398,
                                "Jan  5 09:15",
                            )),
                        ),
                        (
                            "known_hosts".to_string(),
                            DirItem::File(FileEntry::new(
                                "-rw-------",
                                "ubuntu",
                                "ubuntu",
                                812,
                                "Jan 10 14:21",
                            )),
                        ),
                    ])),
            ),
        ),
        (
            "snap".to_string(),
            DirItem::Dir(
                DirEntry::new("drwxrwxr-x", "ubuntu", "ubuntu", 4096, "Jan  8 10:15")
                    .with_children(BTreeMap::from([(
                        "lxd".to_string(),
                        DirItem::Dir(DirEntry::new(
                            "drwxr-xr-x",
                            "ubuntu",
                            "ubuntu",
                            4096,
                            "Jan  8 10:15",
                        )),
                    )])),
            ),
        ),
    ]);
    ubuntu_children.extend(home_dirs);

    let root = DirEntry::new("drwxr-xr-x", "root", "root", 4096, "Jan  5 09:15").with_children(
        BTreeMap::from([
            (
                "home".to_string(),
                DirItem::Dir(
                    DirEntry::new("drwxr-xr-x", "root", "root", 4096, "Jan  5 09:15")
                        .with_children(BTreeMap::from([(
                            "ubuntu".to_string(),
                            DirItem::Dir(
                                DirEntry::new(
                                    "drwxr-x---",
                                    "ubuntu",
                                    "ubuntu",
                                    4096,
                                    "Jan 10 14:23",
                                )
                                .with_children(ubuntu_children),
                            ),
                        )])),
                ),
            ),
            (
                "etc".to_string(),
                DirItem::Dir(
                    DirEntry::new("drwxr-xr-x", "root", "root", 4096, "Jan  5 09:15")
                        .with_children(BTreeMap::from([
                            (
                                "passwd".to_string(),
                                DirItem::File(FileEntry::new(
                                    "-rw-r--r--",
                                    "root",
                                    "root",
                                    1842,
                                    "Jan  5 09:15",
                                )),
                            ),
                            (
                                "ssh".to_string(),
                                DirItem::Dir(DirEntry::new(
                                    "drwxr-xr-x",
                                    "root",
                                    "root",
                                    4096,
                                    "Jan  5 09:15",
                                )),
                            ),
                        ])),
                ),
            ),
            (
                "var".to_string(),
                DirItem::Dir(
                    DirEntry::new("drwxr-xr-x", "root", "root", 4096, "Jan  5 09:15")
                        .with_children(BTreeMap::from([(
                            "log".to_string(),
                            DirItem::Dir(
                                DirEntry::new("drwxr-xr-x", "syslog", "adm", 4096, "Jan 10 14:23")
                                    .with_children(BTreeMap::from([(
                                        "auth.log".to_string(),
                                        DirItem::File(FileEntry::new(
                                            "-rw-r-----",
                                            "syslog",
                                            "adm",
                                            24576,
                                            "Jan 10 14:23",
                                        )),
                                    )])),
                            ),
                        )])),
                ),
            ),
            (
                "tmp".to_string(),
                DirItem::Dir(DirEntry::new(
                    "drwxrwxrwt",
                    "root",
                    "root",
                    4096,
                    "Jan 10 14:20",
                )),
            ),
        ]),
    );

    FakeFileSystem::new(root)
}
