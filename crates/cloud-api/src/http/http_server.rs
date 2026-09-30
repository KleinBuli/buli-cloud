use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use cloud_core::{CloudCore, instances::instance::Instance, templates::template::Template};

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

async fn templates(State(cloud_core): State<Arc<CloudCore>>) -> Json<Vec<Template>> {
    Json(cloud_core.template_manager().templates_list().await)
}
async fn create_template(State(cloud_core): State<Arc<CloudCore>>, Path(name): Path<String>) -> StatusCode {
    match cloud_core.template_manager().create_new_template(&name).await {
        Ok(_) => StatusCode::CREATED,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => StatusCode::CONFLICT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
async fn delete_template(State(cloud_core): State<Arc<CloudCore>>, Path(name): Path<String>) -> StatusCode {
    match cloud_core.template_manager().delete_template(&name).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => StatusCode::NOT_FOUND,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn create_instance_from_template(
    State(cloud_core): State<Arc<CloudCore>>,
    Path(template_name): Path<String>,
) -> Result<(StatusCode, Json<Instance>), StatusCode> {
    let template = cloud_core
        .template_manager()
        .get_template(&template_name)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let instance = cloud_core.instance_manager().create_instance_from_template(template).await;

    Ok((StatusCode::CREATED, Json(instance)))
}

async fn instances(State(cloud_core): State<Arc<CloudCore>>) -> Json<Vec<Instance>> {
    let instance_manager = cloud_core.instance_manager();
    Json(instance_manager.instances_list().await)
}

pub async fn start_http_server(cloud_core: Arc<CloudCore>) -> std::io::Result<()> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/paths", get(paths))
        .route("/templates", get(templates))
        .route("/templates/{name}", post(create_template))
        .route("/templates/{name}", delete(delete_template))
        .route("/instances", get(instances))
        .route("/instances/{template_name}", post(create_instance_from_template))
        .with_state(cloud_core);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;

    axum::serve(listener, app).await.map_err(std::io::Error::other)
}
