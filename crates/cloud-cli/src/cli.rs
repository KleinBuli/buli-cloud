use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bulicloud", disable_help_subcommand = true, disable_help_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Start {
        group: String,
        template: String,
    },
    Stop {
        instance: String,
    },
    Console {
        instance: String,
    },

    Instances,
    Reload,
    Shutdown,
    Health,
    Help,

    Template {
        #[command(subcommand)]
        command: TemplateCommands,
    },

    Group {
        #[command(subcommand)]
        command: GroupCommands,
    },
}

#[derive(Subcommand)]
pub enum TemplateCommands {
    Create {
        group: String,
        name: String,

        #[arg(long, conflicts_with = "server")]
        proxy: bool,

        #[arg(long, conflicts_with = "proxy")]
        server: bool,
    },

    List,
}

#[derive(Subcommand)]
pub enum GroupCommands {
    Create { name: String },
    List,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_targets_instance_and_copy_is_unavailable() {
        assert!(matches!(Cli::try_parse_from(["bulicloud", "stop", "lobby-1"]).unwrap().command, Some(Commands::Stop { instance }) if instance == "lobby-1"));
        assert!(Cli::try_parse_from(["bulicloud", "copy", "lobby-1"]).is_err());
    }
}
