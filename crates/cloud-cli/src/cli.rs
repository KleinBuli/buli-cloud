use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bulicloud", disable_help_subcommand = true, disable_help_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Console {
        instance: String,
    },

    Instance {
        #[command(subcommand)]
        command: InstanceCommands,
    },
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
pub enum InstanceCommands {
    Create { group: String, template: String },
    Start { instance: String },
    Stop { instance: String },
    List,
}

#[derive(Subcommand)]
pub enum GroupCommands {
    Create {
        name: String,
        #[arg(long, conflicts_with_all = ["proxified", "standalone"])]
        proxy: bool,

        #[arg(long, conflicts_with_all = ["proxy", "standalone"])]
        proxified: bool,

        #[arg(long, conflicts_with_all = ["proxified", "proxy"])]
        standalone: bool,
    },
    List,
}
