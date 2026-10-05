use std::io::{Error, Write, stdin, stdout};

use clap::Parser;
use cloud_core::{
    config::config_manager::ConfigManager,
    instances::instance::Instance,
    logger::logger::{
        LogLevel::{self, Info},
        log,
    },
};
use reqwest::Client;
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

    let http_client = reqwest::Client::new();
    let url = "http://127.0.0.1:8080/";

    run_cli_loop(&http_client, url).await?;
    child.wait().await?;

    Ok(())
}

async fn run_cli_loop(http_client: &Client, url: &str) -> Result<(), Error> {
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
            Ok(cli) => match parse_commands(http_client, url.to_string(), cli).await {
                Ok(bo) => {
                    if !bo {
                        break;
                    }
                }
                Err(error) => {
                    log(LogLevel::Error, &format!("{error}"));
                    continue;
                }
            },
            Err(error) => {
                log(LogLevel::Error, &format!("{error}"));
                continue;
            }
        }
    }

    Ok(())
}

async fn parse_commands(http_client: &Client, url: String, cli: Cli) -> Result<bool, reqwest::Error> {
    match cli.command {
        Some(command) => match command {
            cli::Commands::Group { command } => match command {
                cli::GroupCommands::Create { name } => {
                    let response = http_client.post(format!("{}groups/{}", url, name)).send().await?;
                    let status = response.status();
                    let body = response.text().await?;

                    if status.is_success() {
                        log(Info, &format!("Group {} created successfully", name));
                    } else {
                        log(
                            LogLevel::Error,
                            &format!("Error while creating group {}: {} - {}", name, body, status),
                        );
                    }

                    return Ok(true);
                }
            },

            cli::Commands::Template { command } => match command {
                cli::TemplateCommands::Create {
                    group,
                    name,
                    proxy,
                    server: _,
                } => {
                    let response = http_client
                        .post(format!(
                            "{}templates/{}/{}?software={}",
                            url,
                            group,
                            name,
                            if proxy { "Velocity" } else { "Paper" }
                        ))
                        .send()
                        .await?;
                    let status = response.status();
                    let body = response.text().await?;

                    if status.is_success() {
                        log(Info, &format!("Created Template: {}", name));
                    } else {
                        log(
                            LogLevel::Error,
                            &format!("Error while creating template {}: {} - {}", name, body, status),
                        );
                    }
                    return Ok(true);
                }
            },

            cli::Commands::Start { group, template } => {
                let response = http_client
                    .post(format!("{}instances/new/{}/{}", url, group, template))
                    .send()
                    .await?;

                let status = response.status();

                if !status.is_success() {
                    let body = response.text().await?;
                    log(LogLevel::Error, &format!("Error while creating instance: {} - {}", status, body));
                    return Ok(true);
                }

                let (id, _) = response.json::<(String, Instance)>().await?;

                log(Info, &format!("Created Instance: {}", id));

                let response = http_client.post(format!("{}instances/{}/start", url, id)).send().await?;

                let status = response.status();

                if status.is_success() {
                    log(Info, &format!("Started Instance: {}", id));
                } else {
                    let body = response.text().await?;
                    log(
                        LogLevel::Error,
                        &format!("Error while starting instance {}: {} - {}", id, status, body),
                    );
                }

                Ok(true)
            }

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
