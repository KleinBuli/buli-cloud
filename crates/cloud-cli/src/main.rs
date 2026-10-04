use std::io::{Error, Write, stdin, stdout};

use clap::Parser;
use cloud_core::{
    config::config_manager::ConfigManager,
    logger::logger::{
        LogLevel::{self},
        log,
    },
};
use tokio::process::Command;

use crate::{cli::Cli, setup::run_setup};

pub mod cli;
pub mod setup;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli_path = std::env::current_exe()?;
    let data_path = cli_path.parent().unwrap().parent().unwrap().join("data");

    let config_path = data_path.join("config/config.toml");
    if !config_path.exists() {
        let config = run_setup()?;
        let config_manager = ConfigManager::from(config_path.to_path_buf(), config).await?;
        config_manager.save_config().await?;
    }

    let daemon_path = cli_path.parent().unwrap().join("cloud-daemon.exe");

    let mut child = Command::new(daemon_path).spawn()?;
    run_cli_loop().await?;
    child.wait().await?;

    Ok(())
}

async fn run_cli_loop() -> Result<(), Error> {
    loop {
        print!("BuliCloud > ");
        stdout().flush()?;
        let mut input = String::new();
        stdin().read_line(&mut input)?;

        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        let mut args = vec!["bulicloud"];
        args.extend(input.split_whitespace());
        match Cli::try_parse_from(args) {
            Ok(cli) => {
                if !parse_commands(cli).await {
                    break;
                }
            }
            Err(error) => {
                log(LogLevel::Error, &format!("{error}"));
                continue;
            }
        }
    }

    Ok(())
}

async fn parse_commands(cli: Cli) -> bool {
    match cli.command {
        Some(command) => match command {
            cli::Commands::Template { command } => match command {
                cli::TemplateCommands::Create { name } => {
                    println!("Create template: {}", name);
                    return true;
                }
            },

            cli::Commands::Start { template } => {
                println!("Start template: {}", template);
                return true;
            }

            cli::Commands::Stop { template } => {
                println!("Stop template: {}", template);
                return true;
            }

            cli::Commands::Copy { instance } => {
                println!("Copy instance: {}", instance);
                return true;
            }

            cli::Commands::Shutdown => {
                println!("Shutdown");
                return false;
            }

            cli::Commands::Health => {
                println!("Health");
                return true;
            }
        },

        None => {
            print_help();
            return true;
        }
    }
}

fn print_help() {
    println!(
        r#"
BuliCloud Commands

  template create <name>      Create a new template
  start <template_name>       Start an instance from a template
  stop <instance_name>        Stop a running instance
  copy <instance_name>        Copy an instance
  shutdown                    Stop the BuliCloud daemon
  health                      Check daemon health
"#
    );
}
