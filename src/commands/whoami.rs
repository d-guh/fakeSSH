use super::{Args, Command, CommandContext};

pub struct Whoami;

impl Command for Whoami {
    fn run(&self, _args: &Args, ctx: &mut CommandContext) -> Vec<u8> {
        ctx.username.as_bytes().to_vec()
    }
}
