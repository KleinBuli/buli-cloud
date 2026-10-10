use axum::{
    Json, Router,
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::Response,
    routing::{delete, get, post},
};
use cloud_core::{
    CloudCore,
    groups::group::Group,
    instances::{instance::Instance, instance_manager::InstanceManager, instance_runtime::InstanceInfo},
    logger::logger::{LogLevel, log},
    util::software::softwaremanager::ServerSoftware,
};
use serde::Deserialize;
use std::{
    io::{Error, ErrorKind},
    sync::Arc,
};

fn error_status(error: Error) -> StatusCode {
    let status = match error.kind() {
        ErrorKind::NotFound => StatusCode::NOT_FOUND,
        ErrorKind::AlreadyExists | ErrorKind::ResourceBusy => StatusCode::CONFLICT,
        ErrorKind::InvalidInput => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    log(LogLevel::Error, &error.to_string());
    status
}

async fn health() -> &'static str {
    "BuliCloud is running!"
}

async fn paths(State(core): State<Arc<CloudCore>>) -> String {
    format!(
        "templates: {}\nrunning: {}\nstatic: {}\nconfig: {}",
        core.templates_path().display(),
        core.running_path().display(),
        core.static_servers_path().display(),
        core.config_path().display()
    )
}

async fn reload(State(core): State<Arc<CloudCore>>) -> StatusCode {
    match core.reload().await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn templates(State(core): State<Arc<CloudCore>>) -> Json<Vec<String>> {
    let mut result = Vec::new();
    for group in core.group_manager().groups().await {
        for name in group.template_names() {
            result.push(format!("{}/{name}", group.name()));
        }
    }
    Json(result)
}

#[derive(Deserialize)]
struct TemplateOptions {
    software: Option<ServerSoftware>,
    version: Option<String>,
}

async fn create_template(
    State(core): State<Arc<CloudCore>>,
    Path((group, name)): Path<(String, String)>,
    Query(options): Query<TemplateOptions>,
) -> Result<StatusCode, StatusCode> {
    let software = options.software.unwrap_or(ServerSoftware::Paper);

    let version = match software {
        ServerSoftware::Paper | ServerSoftware::Vanilla => Some(
            options
                .version
                .unwrap_or_else(|| core.config_manager().config().fallback_minecraft_version().to_string()),
        ),

        ServerSoftware::Velocity | ServerSoftware::Custom => None,
    };

    if software == ServerSoftware::Custom {
        return Err(StatusCode::BAD_REQUEST);
    }

    core.create_template(&group, &name, software, version, None).await.map_err(error_status)?;

    Ok(StatusCode::CREATED)
}

async fn delete_template(State(core): State<Arc<CloudCore>>, Path((group, name)): Path<(String, String)>) -> Result<StatusCode, StatusCode> {
    core.delete_template(&group, &name).await.map_err(error_status)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_instance_from_template(
    State(core): State<Arc<CloudCore>>,
    Path((group, name)): Path<(String, String)>,
) -> Result<(StatusCode, Json<(String, Instance)>), StatusCode> {
    let instance = core.create_instance_from_template(&group, &name).await.map_err(error_status)?;
    Ok((StatusCode::CREATED, Json((instance.id().to_string(), instance))))
}

async fn delete_instance(State(core): State<Arc<CloudCore>>, Path(id): Path<String>) -> Result<StatusCode, StatusCode> {
    core.remove_instance(&id).await.map_err(error_status)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn instances(State(core): State<Arc<CloudCore>>) -> Json<Vec<InstanceInfo>> {
    Json(core.instance_manager().instances_list().await)
}

async fn start_instance(State(core): State<Arc<CloudCore>>, Path(id): Path<String>) -> Result<StatusCode, StatusCode> {
    core.start_instance(&id).await.map_err(error_status)?;
    Ok(StatusCode::OK)
}

async fn stop_instance(State(core): State<Arc<CloudCore>>, Path(id): Path<String>) -> Result<StatusCode, StatusCode> {
    core.stop_instance(&id).await.map_err(error_status)?;
    Ok(StatusCode::OK)
}

async fn instance_console(State(core): State<Arc<CloudCore>>, Path(id): Path<String>, web_socket: WebSocketUpgrade) -> Response {
    web_socket.on_upgrade(move |socket| InstanceManager::handle_instance_console(socket, core, id))
}

async fn groups(State(core): State<Arc<CloudCore>>) -> Json<Vec<Group>> {
    Json(core.group_manager().groups().await)
}

async fn group(State(core): State<Arc<CloudCore>>, Path(name): Path<String>) -> Result<Json<Group>, StatusCode> {
    core.group_manager().get_group(&name).await.map(Json).ok_or(StatusCode::NOT_FOUND)
}

async fn create_group(State(core): State<Arc<CloudCore>>, Path(name): Path<String>) -> Result<StatusCode, StatusCode> {
    core.create_group(&name).await.map_err(error_status)?;
    Ok(StatusCode::CREATED)
}

async fn delete_group(State(core): State<Arc<CloudCore>>, Path(name): Path<String>) -> Result<StatusCode, StatusCode> {
    core.remove_group(&name).await.map_err(error_status)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn enable_group_maintenance(State(core): State<Arc<CloudCore>>, Path(name): Path<String>) -> Result<StatusCode, StatusCode> {
    core.set_group_maintenance(&name, true).await.map_err(error_status)?;
    Ok(StatusCode::OK)
}

async fn disable_group_maintenance(State(core): State<Arc<CloudCore>>, Path(name): Path<String>) -> Result<StatusCode, StatusCode> {
    core.set_group_maintenance(&name, false).await.map_err(error_status)?;
    Ok(StatusCode::OK)
}

pub fn router(core: Arc<CloudCore>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/reload", post(reload))
        .route("/paths", get(paths))
        .route("/groups", get(groups))
        .route("/groups/{name}", get(group).post(create_group).delete(delete_group))
        .route("/groups/{name}/maintenance", post(enable_group_maintenance).delete(disable_group_maintenance))
        .route("/templates", get(templates))
        .route("/templates/{group_name}/{template_name}", post(create_template).delete(delete_template))
        .route("/instances", get(instances))
        .route("/instances/new/{group_name}/{template_name}", post(create_instance_from_template))
        .route("/instances/{id}/remove", delete(delete_instance))
        .route("/instances/{id}/start", post(start_instance))
        .route("/instances/{id}/stop", post(stop_instance))
        .route("/instances/{id}/console", get(instance_console))
        .with_state(core)
}

pub async fn start_http_server(core: Arc<CloudCore>) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    axum::serve(listener, router(core)).await.map_err(Error::other)
}
