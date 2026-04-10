use super::{Args, Command, CommandContext};

pub struct Pwd;

impl Command for Pwd {
    fn run(&self, _args: &Args, ctx: &mut CommandContext) -> Vec<u8> {
        ctx.pwd().into_bytes()
    }
}
