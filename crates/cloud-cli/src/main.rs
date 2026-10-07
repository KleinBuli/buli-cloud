use std::io::Error;

use tokio::process::Command;

use crate::repl::run_cli_loop;

pub mod cli;
mod commands;
mod repl;
pub mod setup;
mod startup;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli_path = std::env::current_exe()?;
    startup::ensure_config(&cli_path).await?;

    let daemon_path = cli_path.parent().unwrap().join("cloud-daemon.exe");

    let mut child = Command::new(daemon_path).spawn()?;

    let http_client = reqwest::Client::new();
    let url = "http://127.0.0.1:8080/";

    run_cli_loop(&http_client, url).await?;
    child.wait().await?;

    Ok(())
}
