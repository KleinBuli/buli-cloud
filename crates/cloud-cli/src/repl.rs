use std::io::{Error, Write, stdin, stdout};

use clap::Parser;
use cloud_core::logger::logger::{LogLevel, log};
use reqwest::Client;

use crate::{cli::Cli, commands::parse_commands};

pub(crate) async fn run_cli_loop(http_client: &Client, url: &str) -> Result<(), Error> {
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
                match error.kind() {
                    clap::error::ErrorKind::InvalidSubcommand | clap::error::ErrorKind::UnknownArgument => {
                        log(LogLevel::Error, "Invalid command. Run \x1b[36mhelp\x1b[0m for available commands.");
                    }

                    _ => {
                        error.print().ok();
                    }
                }
                continue;
            }
        }
    }

    Ok(())
}
