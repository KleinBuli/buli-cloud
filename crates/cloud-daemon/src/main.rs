use std::sync::Arc;

use cloud_api::http::http_server::start_http_server;
use cloud_core::{
    CloudCore,
    logger::logger::{
        LogLevel::{self, Error},
        log,
    },
};

#[tokio::main]
async fn main() {
    log(LogLevel::Info, "Starting BuliCloud daemon...");

    let root_path = std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("data");

    let core = match CloudCore::new(root_path).await {
        Ok(core) => core,
        Err(error) => {
            log(Error, &format!("Error to create BuliCloud-Core: {error}"));
            return;
        }
    };

    let core = Arc::new(core);

    if let Err(error) = core.initialize().await {
        log(LogLevel::Error, &format!("Failed to initialize BuliCloud-Core: {error}"));
        return;
    }

    log(LogLevel::Info, "BuliCloud-Core initialized");
    log(LogLevel::Info, "BuliCloud daemon is running");

    tokio::spawn(async move {
        log(LogLevel::Info, "Starting HTTP-Server at Port 8080....");
        log(LogLevel::Info, "HTTP Server is running");

        if let Err(error) = start_http_server(Arc::clone(&core)).await {
            log(LogLevel::Error, &format!("failed starting http server: {}", error));
        }
    });

    if let Err(error) = tokio::signal::ctrl_c().await {
        log(LogLevel::Error, &format!("Failed to listen for shutdown signal: {error}"));
        return;
    }
    log(LogLevel::Info, "Shutdown signal received");
    log(LogLevel::Info, "Stopping BuliCloud daemon...");
}
