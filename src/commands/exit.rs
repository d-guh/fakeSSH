use super::{Args, Command, CommandContext};

pub struct Exit;

impl Command for Exit {
    fn run(&self, _args: &Args, ctx: &mut CommandContext) -> Vec<u8> {
        ctx.should_exit = true;
        Vec::new()
    }
}
