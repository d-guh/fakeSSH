mod hostname;
mod ls;

use std::collections::HashSet;

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

        Args { short_flags, long_flags, positional }
    }
}

pub trait Command {
    fn run(&self, args: &Args) -> Vec<u8>;
}

pub fn execute(parts: &[&str]) -> Option<Vec<u8>> {
    let cmd: &dyn Command = match parts.first().copied() {
        Some("ls") => &ls::Ls,
        Some("hostname") => &hostname::Hostname,
        _ => return None,
    };

    let args = Args::parse(&parts[1..]);
    Some(cmd.run(&args))
}
