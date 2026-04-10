use super::{Args, Command, CommandContext};

pub struct W;

impl Command for W {
    fn run(&self, _args: &Args, ctx: &mut CommandContext) -> Vec<u8> {
        let now = chrono::Local::now().format("%H:%M:%S");
        let idle = "0.00s";
        let jcpu = "0.01s";
        let pcpu = "0.00s";
        let what = "-bash";

        format!(
            " {now} up 3 days,  4:12,  1 user,  load average: 0.03, 0.07, 0.05\r\nUSER     TTY      FROM             LOGIN@   IDLE   JCPU   PCPU WHAT\r\n{:<8} pts/0    -                {}   {:<6} {:<6} {:<6} {}",
            ctx.username,
            login_at_short(&ctx.login_time),
            idle,
            jcpu,
            pcpu,
            what,
        )
        .into_bytes()
    }
}

fn login_at_short(login_time: &str) -> &str {
    login_time.split_whitespace().nth(1).unwrap_or(login_time)
}
