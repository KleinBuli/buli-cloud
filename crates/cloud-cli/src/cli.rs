use clap::{Parser, Subcommand};

#[derive(Parser)]
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
        template: String,
    },
    Copy {
        instance: String,
    },
    Shutdown,
    Health,
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
}

// group create <name>
#[derive(Subcommand)]
pub enum GroupCommands {
    Create { name: String },
}
