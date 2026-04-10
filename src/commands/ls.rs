use super::{Args, Command, CommandContext};

pub struct Ls;

impl Command for Ls {
    fn run(&self, args: &Args, ctx: &mut CommandContext) -> Vec<u8> {
        let long = args.short_flags.contains(&'l');
        let show_all = args.short_flags.contains(&'a');
        let target = args.positional.first().map(String::as_str);

        match ctx.fs.list_dir(&ctx.cwd, target, show_all) {
            Ok(entries) => {
                if long {
                    let total: u64 = entries.iter().map(|entry| entry.size).sum::<u64>() / 1024;
                    let mut lines = vec![format!("total {}", total.max(4))];
                    lines.extend(entries.into_iter().map(|entry| {
                        format!(
                            "{} 1 {:<7} {:<7} {:>4} {} {}",
                            entry.mode,
                            entry.owner,
                            entry.group,
                            entry.size,
                            entry.modified,
                            colorize_name(&entry.name, entry.is_dir),
                        )
                    }));
                    lines.join("\r\n").into_bytes()
                } else {
                    entries
                        .into_iter()
                        .map(|entry| colorize_name(&entry.name, entry.is_dir))
                        .collect::<Vec<_>>()
                        .join("  ")
                        .into_bytes()
                }
            }
            Err(err) => err.into_bytes(),
        }
    }
}

fn colorize_name(name: &str, is_dir: bool) -> String {
    if is_dir {
        format!("\x1b[01;34m{}\x1b[0m", name)
    } else {
        name.to_string()
    }
}
