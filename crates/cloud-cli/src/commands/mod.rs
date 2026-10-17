use cloud_core::{
    groups::group::GroupRole,
    logger::logger::{
        LogLevel::{Error, Info, Warn},
        log,
    },
};
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
            cli::Commands::Help {} => {
                help::print_help();
                Ok(true)
            }

            cli::Commands::Reload {} => reload::reload(http_client, &url).await,

            cli::Commands::Console { instance } => {
                match instance::console(&url, &instance).await {
                    Ok(_) => {}
                    Err(error) => log(Error, &format!("Error while connecting to console: {error}")),
                };
                Ok(true)
            }

            cli::Commands::Group { command } => match command {
                cli::GroupCommands::Create {
                    name,
                    proxy,
                    proxified,
                    standalone,
                } => {
                    if !proxy && !proxified && !standalone {
                        log(Warn, "Specify whether the group should be --proxy, --proxified or --standalone");
                        return Ok(true);
                    }

                    if proxy {
                        group::create(http_client, &url, name.to_string(), GroupRole::Proxy).await?;
                    }

                    if proxified {
                        group::create(http_client, &url, name.to_string(), GroupRole::ProxiedServer).await?;
                    }

                    if standalone {
                        group::create(http_client, &url, name.to_string(), GroupRole::StandaloneServer).await?;
                    }

                    Ok(true)
                }
                cli::GroupCommands::List => group::list(http_client, &url).await,
            },

            cli::Commands::Template { command } => match command {
                cli::TemplateCommands::Create { group, name, proxy, server: _ } => template::create(http_client, &url, group, name, proxy).await,
                cli::TemplateCommands::List => template::list(http_client, &url).await,
            },

            cli::Commands::Instance { command } => match command {
                cli::InstanceCommands::List => instance::list(http_client, &url).await,
                cli::InstanceCommands::Create { group, template } => instance::create(http_client, &url, group, template).await,
                cli::InstanceCommands::Start { instance } => instance::start(http_client, &url, &instance).await,
                cli::InstanceCommands::Stop { instance } => instance::stop(http_client, &url, &instance).await,
            },
            cli::Commands::Shutdown => {
                http_client.post(format!("{url}shutdown")).send().await?.error_for_status()?;
                return Ok(false);
            }

            cli::Commands::Health => {
                let response = http_client.get(url + "health").send().await?;
                log(Info, &format!("{}", response.text().await?));
                return Ok(true);
            }
        },

        None => {
            help::print_help();
            return Ok(true);
        }
    }
}
