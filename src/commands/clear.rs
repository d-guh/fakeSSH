use super::{Args, Command, CommandContext};

const CLEAR_SCREEN: &str = "\x1b[2J\x1b[H";

pub struct Clear;

impl Command for Clear {
    fn run(&self, _args: &Args, _ctx: &mut CommandContext) -> Vec<u8> {
        CLEAR_SCREEN.as_bytes().to_vec()
    }
}
