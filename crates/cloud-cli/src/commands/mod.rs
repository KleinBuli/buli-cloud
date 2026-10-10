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

            cli::Commands::Stop { instance } => {
                http_client.post(format!("{url}instances/{instance}/stop")).send().await?.error_for_status()?;
                Ok(true)
            }

            cli::Commands::Shutdown => {
                http_client.post(format!("{url}shutdown")).send().await?.error_for_status()?;
                return Ok(false);
            }

            cli::Commands::Health => {
                let response = http_client.get(url + "health").send().await?;
                println!("{}", response.text().await?);
                return Ok(true);
            }
        },

        None => {
            help::print_help();
            return Ok(true);
        }
    }
}
