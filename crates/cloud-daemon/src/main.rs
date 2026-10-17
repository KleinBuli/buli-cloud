use std::{io::Write, sync::Arc};

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
    print!("\x1B[2J\x1B[1;1H");
    let _ = std::io::stdout().flush();
    println!(
        r#"
  ____        _ _  ____ _                 _
 | __ ) _   _| (_)/ ___| | ___  _   _  __| |
 |  _ \| | | | | | |   | |/ _ \| | | |/ _` |
 | |_) | |_| | | | |___| | (_) | |_| | (_| |
 |____/ \__,_|_|_|\____|_|\___/ \__,_|\__,_|

                BuliCloud Daemon
──────────────────────────────────────────────────
"#
    );
    log(LogLevel::Info, "Starting BuliCloud daemon...");

    let root_path = std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("data");

    let mut core = match CloudCore::new(root_path).await {
        Ok(core) => core,
        Err(error) => {
            log(Error, &format!("Error to create BuliCloud-Core: {error}"));
            return;
        }
    };

    if let Err(error) = core.initialize().await {
        log(LogLevel::Error, &format!("Failed to initialize BuliCloud-Core: {error}"));
        return;
    }

    let core = Arc::new(core);

    log(LogLevel::Info, "BuliCloud-Core initialized");
    log(LogLevel::Info, "BuliCloud daemon is running");

    core.spawn_instance_event_listener().await;
    log(LogLevel::Info, "Instance Eventlistener is running");

    let server_core = Arc::clone(&core);
    let mut server_task = tokio::spawn(async move {
        log(LogLevel::Info, "Starting HTTP-Server at Port 8080....");
        log(LogLevel::Info, "HTTP Server is running");

        if let Err(error) = start_http_server(server_core).await {
            log(LogLevel::Error, &format!("failed starting http server: {}", error));
        }
    });

    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            if let Err(error) = result { log(LogLevel::Error, &format!("Shutdown signal failed: {error}")); }
        }
        _ = core.wait_for_shutdown() => {}
        _ = &mut server_task => {}
    }
    core.request_shutdown();
    log(LogLevel::Info, "Shutdown signal received");
    log(LogLevel::Info, "Stopping BuliCloud daemon...");
    if let Err(error) = core.shutdown().await {
        log(LogLevel::Error, &format!("Could not stop all instances: {error}"));
    }
    if !server_task.is_finished() && tokio::time::timeout(std::time::Duration::from_secs(2), &mut server_task).await.is_err() {
        server_task.abort();
    }
}
