use cloud_api::http::http_server::start_http_server;
use cloud_core::{
    CloudCore,
    logger::logger::{
        LogLevel::{self},
        log,
    },
};

#[tokio::main]
async fn main() {
    log(LogLevel::Info, "Starting BuliCloud daemon...");

    let core = CloudCore::new("./bulicloud");

    if let Err(error) = core.initialize() {
        log(LogLevel::Error, &format!("Failed to initialize BuliCloud-Core: {error}"));
        return;
    }

    log(LogLevel::Info, "BuliCloud-Core initialized");
    log(LogLevel::Info, "BuliCloud daemon is running");

    tokio::spawn(async {
        if let Err(error) = start_http_server().await {
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
