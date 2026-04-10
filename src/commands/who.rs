use super::{Args, Command, CommandContext};

pub struct Who;

impl Command for Who {
    fn run(&self, _args: &Args, ctx: &mut CommandContext) -> Vec<u8> {
        format!("{} pts/0 {}", ctx.username, ctx.login_time).into_bytes()
    }
}
