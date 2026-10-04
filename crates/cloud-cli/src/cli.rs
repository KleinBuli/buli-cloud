use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Start {
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
}

#[derive(Subcommand)]
pub enum TemplateCommands {
    Create { name: String },
}
