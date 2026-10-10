use reqwest::Client;

use crate::cli::{self, Cli};

mod group;
mod help;
mod instance;
mod reload;
mod template;

pub(crate) async fn parse_commands(http_client: &Client, url: String, cli: Cli) -> Result<bool, reqwest::Error> {
    match cli.command {
        Some(command) => match command {
            cli::Commands::Instances => instance::list(http_client, &url).await,

            cli::Commands::Help {} => {
                help::print_help();
                Ok(true)
            }

            cli::Commands::Reload {} => reload::reload(http_client, &url).await,

            cli::Commands::Console { instance } => {
                match instance::console(&url, &instance).await {
                    Ok(_) => {}
                    Err(error) => {
                        println!("Error while connecting to console: {error}")
                    }
                };
                Ok(true)
            }

            cli::Commands::Group { command } => match command {
                cli::GroupCommands::Create { name } => group::create(http_client, &url, name).await,
                cli::GroupCommands::List => group::list(http_client, &url).await,
            },

            cli::Commands::Template { command } => match command {
                cli::TemplateCommands::Create { group, name, proxy, server: _ } => template::create(http_client, &url, group, name, proxy).await,
                cli::TemplateCommands::List => template::list(http_client, &url).await,
            },

            cli::Commands::Start { group, template } => instance::start(http_client, &url, group, template).await,

            cli::Commands::Stop { template } => {
                println!("Stop template: {}", template);
                return Ok(true);
            }

            cli::Commands::Copy { instance } => {
                println!("Copy instance: {}", instance);
                return Ok(true);
            }

            cli::Commands::Shutdown => {
                println!("Shutdown");
                return Ok(false);
            }

            cli::Commands::Health => {
                let response = http_client.get(url + "health").send().await?;
                println!("{}", response.text().await?);
                return Ok(true);
            }
        },

        None => {
            print_help();
            return Ok(true);
        }
    }
}

fn print_help() {
    println!(
        r#"
BuliCloud Commands

  group create <name>              Create a new group            
  template create <group> <name>   Create a new template
  start <group> <template_name>    Start an instance from a template
  stop <instance_name>             Stop a running instance
  copy <instance_name>             Copy an instance
  shutdown                         Stop the BuliCloud daemon
  health                           Check daemon health
"#
    );
}
