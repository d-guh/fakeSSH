mod cd;
mod clear;
mod exit;
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
    pub should_exit: bool,
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
        Some("exit") | Some("logout") => &exit::Exit,
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

pub fn command_names() -> &'static [&'static str] {
    &[
        "clear", "ls", "cd", "pwd", "whoami", "who", "w", "id", "hostname", "uname", "exit",
        "logout",
    ]
}

impl CommandContext {
    pub fn new(hostname: String) -> Self {
        let fs = default_filesystem();
        let cwd = vec!["home".to_string(), "ubuntu".to_string()];
        CommandContext {
            username: "ubuntu".to_string(),
            hostname,
            login_time: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
            should_exit: false,
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

    let root =
        DirEntry::new("drwxr-xr-x", "root", "root", 4096, "Jan  5 09:15").with_children(
            BTreeMap::from([
                (
                    "bin".to_string(),
                    DirItem::Dir(
                        DirEntry::new("drwxr-xr-x", "root", "root", 4096, "Jan  5 09:15")
                            .with_children(BTreeMap::from([
                                (
                                    "bash".to_string(),
                                    DirItem::File(FileEntry::new(
                                        "-rwxr-xr-x",
                                        "root",
                                        "root",
                                        1265648,
                                        "Jan  5 09:15",
                                    )),
                                ),
                                (
                                    "ls".to_string(),
                                    DirItem::File(FileEntry::new(
                                        "-rwxr-xr-x",
                                        "root",
                                        "root",
                                        151344,
                                        "Jan  5 09:15",
                                    )),
                                ),
                            ])),
                    ),
                ),
                (
                    "boot".to_string(),
                    DirItem::Dir(
                        DirEntry::new("drwxr-xr-x", "root", "root", 4096, "Jan  5 09:15")
                            .with_children(BTreeMap::from([
                                (
                                    "grub".to_string(),
                                    DirItem::Dir(DirEntry::new(
                                        "drwxr-xr-x",
                                        "root",
                                        "root",
                                        4096,
                                        "Jan  5 09:15",
                                    )),
                                ),
                                (
                                    "vmlinuz-6.8.0-generic".to_string(),
                                    DirItem::File(FileEntry::new(
                                        "-rw-r--r--",
                                        "root",
                                        "root",
                                        14897152,
                                        "Jan  5 09:15",
                                    )),
                                ),
                            ])),
                    ),
                ),
                (
                    "dev".to_string(),
                    DirItem::Dir(
                        DirEntry::new("drwxr-xr-x", "root", "root", 4096, "Jan 10 14:21")
                            .with_children(BTreeMap::from([
                                (
                                    "null".to_string(),
                                    DirItem::File(FileEntry::new(
                                        "crw-rw-rw-",
                                        "root",
                                        "root",
                                        0,
                                        "Jan 10 14:21",
                                    )),
                                ),
                                (
                                    "pts".to_string(),
                                    DirItem::Dir(DirEntry::new(
                                        "drwxr-xr-x",
                                        "root",
                                        "root",
                                        4096,
                                        "Jan 10 14:21",
                                    )),
                                ),
                                (
                                    "shm".to_string(),
                                    DirItem::Dir(DirEntry::new(
                                        "drwxrwxrwt",
                                        "root",
                                        "root",
                                        4096,
                                        "Jan 10 14:21",
                                    )),
                                ),
                            ])),
                    ),
                ),
                (
                    "etc".to_string(),
                    DirItem::Dir(
                        DirEntry::new("drwxr-xr-x", "root", "root", 4096, "Jan  5 09:15")
                            .with_children(BTreeMap::from([
                                (
                                    "fstab".to_string(),
                                    DirItem::File(FileEntry::new(
                                        "-rw-r--r--",
                                        "root",
                                        "root",
                                        642,
                                        "Jan  5 09:15",
                                    )),
                                ),
                                (
                                    "hostname".to_string(),
                                    DirItem::File(FileEntry::new(
                                        "-rw-r--r--",
                                        "root",
                                        "root",
                                        8,
                                        "Jan  5 09:15",
                                    )),
                                ),
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
                    "lib".to_string(),
                    DirItem::Dir(DirEntry::new(
                        "drwxr-xr-x",
                        "root",
                        "root",
                        4096,
                        "Jan  5 09:15",
                    )),
                ),
                (
                    "lib64".to_string(),
                    DirItem::Dir(DirEntry::new(
                        "drwxr-xr-x",
                        "root",
                        "root",
                        4096,
                        "Jan  5 09:15",
                    )),
                ),
                (
                    "media".to_string(),
                    DirItem::Dir(DirEntry::new(
                        "drwxr-xr-x",
                        "root",
                        "root",
                        4096,
                        "Jan  5 09:15",
                    )),
                ),
                (
                    "mnt".to_string(),
                    DirItem::Dir(DirEntry::new(
                        "drwxr-xr-x",
                        "root",
                        "root",
                        4096,
                        "Jan  5 09:15",
                    )),
                ),
                (
                    "opt".to_string(),
                    DirItem::Dir(
                        DirEntry::new("drwxr-xr-x", "root", "root", 4096, "Jan  5 09:15")
                            .with_children(BTreeMap::from([(
                                "containerd".to_string(),
                                DirItem::Dir(DirEntry::new(
                                    "drwxr-xr-x",
                                    "root",
                                    "root",
                                    4096,
                                    "Jan  8 10:15",
                                )),
                            )])),
                    ),
                ),
                (
                    "proc".to_string(),
                    DirItem::Dir(
                        DirEntry::new("dr-xr-xr-x", "root", "root", 0, "Jan 10 14:23")
                            .with_children(BTreeMap::from([
                                (
                                    "cpuinfo".to_string(),
                                    DirItem::File(FileEntry::new(
                                        "-r--r--r--",
                                        "root",
                                        "root",
                                        0,
                                        "Jan 10 14:23",
                                    )),
                                ),
                                (
                                    "meminfo".to_string(),
                                    DirItem::File(FileEntry::new(
                                        "-r--r--r--",
                                        "root",
                                        "root",
                                        0,
                                        "Jan 10 14:23",
                                    )),
                                ),
                                (
                                    "sys".to_string(),
                                    DirItem::Dir(DirEntry::new(
                                        "dr-xr-xr-x",
                                        "root",
                                        "root",
                                        0,
                                        "Jan 10 14:23",
                                    )),
                                ),
                            ])),
                    ),
                ),
                (
                    "root".to_string(),
                    DirItem::Dir(
                        DirEntry::new("drwx------", "root", "root", 4096, "Jan 10 14:18")
                            .with_children(BTreeMap::from([
                                (
                                    ".bash_history".to_string(),
                                    DirItem::File(FileEntry::new(
                                        "-rw-------",
                                        "root",
                                        "root",
                                        2048,
                                        "Jan 10 14:18",
                                    )),
                                ),
                                (
                                    ".ssh".to_string(),
                                    DirItem::Dir(DirEntry::new(
                                        "drwx------",
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
                    "run".to_string(),
                    DirItem::Dir(
                        DirEntry::new("drwxr-xr-x", "root", "root", 4096, "Jan 10 14:21")
                            .with_children(BTreeMap::from([(
                                "systemd".to_string(),
                                DirItem::Dir(DirEntry::new(
                                    "drwxr-xr-x",
                                    "root",
                                    "root",
                                    4096,
                                    "Jan 10 14:21",
                                )),
                            )])),
                    ),
                ),
                (
                    "sbin".to_string(),
                    DirItem::Dir(
                        DirEntry::new("drwxr-xr-x", "root", "root", 4096, "Jan  5 09:15")
                            .with_children(BTreeMap::from([(
                                "init".to_string(),
                                DirItem::File(FileEntry::new(
                                    "-rwxr-xr-x",
                                    "root",
                                    "root",
                                    92544,
                                    "Jan  5 09:15",
                                )),
                            )])),
                    ),
                ),
                (
                    "srv".to_string(),
                    DirItem::Dir(DirEntry::new(
                        "drwxr-xr-x",
                        "root",
                        "root",
                        4096,
                        "Jan  5 09:15",
                    )),
                ),
                (
                    "sys".to_string(),
                    DirItem::Dir(
                        DirEntry::new("dr-xr-xr-x", "root", "root", 0, "Jan 10 14:23")
                            .with_children(BTreeMap::from([(
                                "kernel".to_string(),
                                DirItem::Dir(DirEntry::new(
                                    "dr-xr-xr-x",
                                    "root",
                                    "root",
                                    0,
                                    "Jan 10 14:23",
                                )),
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
                (
                    "usr".to_string(),
                    DirItem::Dir(
                        DirEntry::new("drwxr-xr-x", "root", "root", 4096, "Jan  5 09:15")
                            .with_children(BTreeMap::from([
                                (
                                    "bin".to_string(),
                                    DirItem::Dir(DirEntry::new(
                                        "drwxr-xr-x",
                                        "root",
                                        "root",
                                        4096,
                                        "Jan  5 09:15",
                                    )),
                                ),
                                (
                                    "lib".to_string(),
                                    DirItem::Dir(DirEntry::new(
                                        "drwxr-xr-x",
                                        "root",
                                        "root",
                                        4096,
                                        "Jan  5 09:15",
                                    )),
                                ),
                                (
                                    "local".to_string(),
                                    DirItem::Dir(DirEntry::new(
                                        "drwxr-xr-x",
                                        "root",
                                        "root",
                                        4096,
                                        "Jan  5 09:15",
                                    )),
                                ),
                                (
                                    "sbin".to_string(),
                                    DirItem::Dir(DirEntry::new(
                                        "drwxr-xr-x",
                                        "root",
                                        "root",
                                        4096,
                                        "Jan  5 09:15",
                                    )),
                                ),
                                (
                                    "share".to_string(),
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
                            .with_children(BTreeMap::from([
                                (
                                    "cache".to_string(),
                                    DirItem::Dir(DirEntry::new(
                                        "drwxr-xr-x",
                                        "root",
                                        "root",
                                        4096,
                                        "Jan 10 14:22",
                                    )),
                                ),
                                (
                                    "lib".to_string(),
                                    DirItem::Dir(DirEntry::new(
                                        "drwxr-xr-x",
                                        "root",
                                        "root",
                                        4096,
                                        "Jan 10 14:22",
                                    )),
                                ),
                                (
                                    "log".to_string(),
                                    DirItem::Dir(
                                        DirEntry::new(
                                            "drwxr-xr-x",
                                            "syslog",
                                            "adm",
                                            4096,
                                            "Jan 10 14:23",
                                        )
                                        .with_children(
                                            BTreeMap::from([(
                                                "auth.log".to_string(),
                                                DirItem::File(FileEntry::new(
                                                    "-rw-r-----",
                                                    "syslog",
                                                    "adm",
                                                    24576,
                                                    "Jan 10 14:23",
                                                )),
                                            )]),
                                        ),
                                    ),
                                ),
                                (
                                    "tmp".to_string(),
                                    DirItem::Dir(DirEntry::new(
                                        "drwxrwxrwt",
                                        "root",
                                        "root",
                                        4096,
                                        "Jan 10 14:22",
                                    )),
                                ),
                            ])),
                    ),
                ),
            ]),
        );

    FakeFileSystem::new(root)
}
