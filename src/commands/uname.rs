use super::{Args, Command, CommandContext};

pub struct Uname;

impl Command for Uname {
    fn run(&self, args: &Args, ctx: &mut CommandContext) -> Vec<u8> {
        let mut kernel_name = false;
        let mut node_name = false;
        let mut kernel_release = false;
        let mut kernel_version = false;
        let mut machine = false;
        let mut processor = false;
        let mut hardware_platform = false;
        let mut operating_system = false;

        if args.short_flags.contains(&'a') || args.long_flags.contains("all") {
            kernel_name = true;
            node_name = true;
            kernel_release = true;
            kernel_version = true;
            machine = true;
            processor = true;
            hardware_platform = true;
            operating_system = true;
        }
        if args.short_flags.contains(&'s') || args.long_flags.contains("kernel-name") {
            kernel_name = true;
        }
        if args.short_flags.contains(&'n') || args.long_flags.contains("nodename") {
            node_name = true;
        }
        if args.short_flags.contains(&'r') || args.long_flags.contains("kernel-release") {
            kernel_release = true;
        }
        if args.short_flags.contains(&'v') || args.long_flags.contains("kernel-version") {
            kernel_version = true;
        }
        if args.short_flags.contains(&'m') || args.long_flags.contains("machine") {
            machine = true;
        }
        if args.short_flags.contains(&'p') || args.long_flags.contains("processor") {
            processor = true;
        }
        if args.short_flags.contains(&'i') || args.long_flags.contains("hardware-platform") {
            hardware_platform = true;
        }
        if args.short_flags.contains(&'o') || args.long_flags.contains("operating-system") {
            operating_system = true;
        }

        if !kernel_name
            && !node_name
            && !kernel_release
            && !kernel_version
            && !machine
            && !processor
            && !hardware_platform
            && !operating_system
        {
            kernel_name = true;
        }

        let mut parts = Vec::new();
        if kernel_name {
            parts.push("Linux");
        }
        if node_name {
            parts.push(&ctx.hostname);
        }
        if kernel_release {
            parts.push("6.17.0-20-generic");
        }
        if kernel_version {
            parts.push("#20~24.04.1-Ubuntu SMP PREEMPT_DYNAMIC Thu Mar 19 01:28:37 UTC 2");
        }
        if machine {
            parts.push("x86_64");
        }
        if processor {
            parts.push("x86_64");
        }
        if hardware_platform {
            parts.push("x86_64");
        }
        if operating_system {
            parts.push("GNU/Linux");
        }

        parts.join(" ").into_bytes()
    }
}
