use std::sync::Arc;

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use cloud_core::CloudCore;

async fn health() -> &'static str {
    "BuliCloud is running!"
}

async fn paths(State(cloud_core): State<Arc<CloudCore>>) -> String {
    format!(
        "templates: {}\nrunning: {}\nstatic: {}\nconfig: {}",
        cloud_core.templates_path().display(),
        cloud_core.running_path().display(),
        cloud_core.static_servers_path().display(),
        cloud_core.config_path().display(),
    )
}

async fn templates(State(cloud_core): State<Arc<CloudCore>>) -> Result<Json<Vec<String>>, StatusCode> {
    match cloud_core.template_manager().templates() {
        Ok(templates) => Ok(Json(templates)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn start_http_server(cloud_core: Arc<CloudCore>) -> std::io::Result<()> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/paths", get(paths))
        .route("/templates", get(templates))
        .with_state(cloud_core);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;

    axum::serve(listener, app).await.map_err(std::io::Error::other)
}
