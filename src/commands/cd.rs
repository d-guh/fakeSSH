use super::{Args, Command, CommandContext};

pub struct Cd;

impl Command for Cd {
    fn run(&self, args: &Args, ctx: &mut CommandContext) -> Vec<u8> {
        let target = args
            .positional
            .first()
            .map(String::as_str)
            .unwrap_or("/home/ubuntu");

        match ctx.fs.resolve_path(&ctx.cwd, target) {
            Ok(path) => {
                ctx.cwd = path;
                Vec::new()
            }
            Err(err) => err.into_bytes(),
        }
    }
}
