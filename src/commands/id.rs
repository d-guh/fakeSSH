use super::{Args, Command, CommandContext};

pub struct Id;

impl Command for Id {
    fn run(&self, _args: &Args, ctx: &mut CommandContext) -> Vec<u8> {
        let profile = ctx.user_profile();
        let groups = profile
            .groups
            .iter()
            .map(|(gid, name)| format!("{gid}({name})"))
            .collect::<Vec<_>>()
            .join(",");

        format!(
            "uid={}({}) gid={}({}) groups={}",
            profile.uid, ctx.username, profile.gid, profile.group_name, groups
        )
        .into_bytes()
    }
}
