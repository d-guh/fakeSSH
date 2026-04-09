use super::{Args, Command, CommandContext};

pub struct Hostname;

impl Command for Hostname {
    fn run(&self, _args: &Args, ctx: &CommandContext) -> Vec<u8> {
        ctx.hostname.as_bytes().to_vec()
    }
}
