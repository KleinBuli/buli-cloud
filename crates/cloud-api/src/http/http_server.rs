use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use cloud_core::{
    CloudCore,
    groups::group::Group,
    instances::{instance::Instance, instance_runtime::InstanceInfo},
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
    let version = if software == ServerSoftware::Velocity {
        None
    } else {
        Some(
            options
                .version
                .unwrap_or_else(|| core.config_manager().config().fallback_minecraft_version().to_string()),
        )
    };
    core.create_template(&group, &name, software, version).await.map_err(error_status)?;
    Ok(StatusCode::CREATED)
}

async fn delete_template(
    State(core): State<Arc<CloudCore>>,
    Path((group, name)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
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
        .route("/paths", get(paths))
        .route("/groups", get(groups))
        .route("/groups/{name}", get(group).post(create_group).delete(delete_group))
        .route(
            "/groups/{name}/maintenance",
            post(enable_group_maintenance).delete(disable_group_maintenance),
        )
        .route("/templates", get(templates))
        .route(
            "/templates/{group_name}/{template_name}",
            post(create_template).delete(delete_template),
        )
        .route("/instances", get(instances))
        .route("/instances/new/{group_name}/{template_name}", post(create_instance_from_template))
        .route("/instances/{id}/remove", delete(delete_instance))
        .route("/instances/{id}/start", post(start_instance))
        .route("/instances/{id}/stop", post(stop_instance))
        .with_state(core)
}

pub async fn start_http_server(core: Arc<CloudCore>) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    axum::serve(listener, router(core)).await.map_err(Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn scoped_routes_create_prepare_and_delete_with_both_path_parameters() {
        let root = tempfile::tempdir().unwrap();
        let core = Arc::new(CloudCore::new(root.path()).await.unwrap());
        std::fs::create_dir_all(core.cache_path()).unwrap();
        std::fs::write(core.cache_path().join("paper-1.21.11.jar"), b"fixture").unwrap();
        core.initialize().await.unwrap();
        let app = router(core.clone());
        for (method, path, expected) in [
            ("POST", "/groups/lobby", StatusCode::CREATED),
            ("POST", "/templates/lobby/custom", StatusCode::CREATED),
            ("POST", "/instances/new/lobby/custom", StatusCode::CREATED),
            ("DELETE", "/templates/lobby/custom", StatusCode::CONFLICT),
            ("POST", "/groups/lobby/maintenance", StatusCode::OK),
            ("POST", "/instances/lobby-1/start", StatusCode::CONFLICT),
            ("POST", "/instances/new/lobby/custom", StatusCode::CONFLICT),
            ("DELETE", "/instances/lobby-1/remove", StatusCode::NO_CONTENT),
            ("DELETE", "/templates/lobby/custom", StatusCode::NO_CONTENT),
            ("DELETE", "/groups/lobby", StatusCode::NO_CONTENT),
            ("POST", "/templates/absent/custom", StatusCode::NOT_FOUND),
        ] {
            let response = app
                .clone()
                .oneshot(Request::builder().method(method).uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), expected, "{method} {path}");
        }
    }
}
