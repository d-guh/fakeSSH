use super::{Args, Command};

pub struct Hostname;

impl Command for Hostname {
    fn run(&self, _args: &Args) -> Vec<u8> {
        b"test-vm".to_vec()
    }
}
