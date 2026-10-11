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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn run_instance_command(args: &[&str], path: &str, status: &str, body: &str) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let expected = format!("POST {path} HTTP/1.1");
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            // Reject any unintended second request, including an automatic start after create.
            drop(listener);
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                let count = socket.read(&mut buffer).await.unwrap();
                assert!(count > 0);
                request.extend_from_slice(&buffer[..count]);
            }
            assert_eq!(String::from_utf8(request).unwrap().lines().next().unwrap(), expected);
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        let cli = Cli::try_parse_from(args).unwrap();
        let client = Client::builder().no_proxy().timeout(std::time::Duration::from_secs(3)).build().unwrap();
        assert!(parse_commands(&client, url, cli).await.unwrap());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn create_only_creates() {
        run_instance_command(
            &["bulicloud", "instance", "create", "lobby", "default"],
            "/instances/new/lobby/default",
            "201 Created",
            r#"["lobby-1",{"id":"lobby-1","group_name":"lobby","template_name":"default","instance_mode":"Dynamic"}]"#,
        )
        .await;
    }

    #[tokio::test]
    async fn start_and_stop_use_existing_instance() {
        for action in ["start", "stop"] {
            run_instance_command(
                &["bulicloud", "instance", action, "lobby-1"],
                &format!("/instances/lobby-1/{action}"),
                "200 OK",
                "",
            )
            .await;
        }
    }

    #[tokio::test]
    async fn api_errors_keep_the_cli_running() {
        for action in ["start", "stop"] {
            run_instance_command(
                &["bulicloud", "instance", action, "missing"],
                &format!("/instances/missing/{action}"),
                "404 Not Found",
                "Instance not found",
            )
            .await;
        }
        run_instance_command(
            &["bulicloud", "instance", "create", "missing", "default"],
            "/instances/new/missing/default",
            "404 Not Found",
            "Template not found",
        )
        .await;
    }

    #[test]
    fn instance_commands_require_arguments() {
        for args in [
            vec!["bulicloud", "instance"],
            vec!["bulicloud", "instance", "create", "lobby"],
            vec!["bulicloud", "instance", "start"],
            vec!["bulicloud", "instance", "stop"],
        ] {
            assert!(Cli::try_parse_from(args).is_err());
        }
        assert!(Cli::try_parse_from(["bulicloud", "instance", "list"]).is_ok());
    }
}
