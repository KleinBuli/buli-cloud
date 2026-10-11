pub fn print_help() {
    const RESET: &str = "\x1b[0m";
    const CYAN: &str = "\x1b[96m";
    const _GREEN: &str = "\x1b[92m";
    const GRAY: &str = "\x1b[90m";
    const BOLD: &str = "\x1b[1m";

    println!();
    println!("{CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{RESET}");
    println!("{BOLD}  BULICLOUD {GRAY}/ Command Reference{RESET}");
    println!("{GRAY}  Lightweight Minecraft Cloud System{RESET}");
    println!();

    println!("{CYAN}{BOLD}  INSTANCE MANAGEMENT{RESET}");
    print_command("instance create <group> <template>", "Create a new instance");
    print_command("instance start <instance>", "Start an existing instance");
    print_command("instance stop <instance>", "Stop an instance");
    print_command("instance list", "List all instances");
    print_command("console <instance>", "Attach to live console");

    println!();
    println!("{CYAN}{BOLD}  CONFIGURATION{RESET}");
    print_command("template create <group> <name>", "Create a template");
    print_command("template list", "List all templates");
    print_command("group create <name>", "Create a group");
    print_command("group list", "List all groups");
    println!("{GRAY}    Template options: --proxy / --server{RESET}");

    println!();
    println!("{CYAN}{BOLD}  SYSTEM{RESET}");
    print_command("reload", "Reload configuration");
    print_command("health", "Check daemon status");
    print_command("shutdown", "Shutdown daemon");
    print_command("help", "Show command reference");

    println!();
    println!("{CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{RESET}");
    println!("{GRAY}  BuliCloud • Built with Rust{RESET}");
    println!();

    fn print_command(command: &str, description: &str) {
        const GREEN: &str = "\x1b[92m";
        const RESET: &str = "\x1b[0m";

        println!("    {GREEN}{command:<34}{RESET}{description}");
    }
}
