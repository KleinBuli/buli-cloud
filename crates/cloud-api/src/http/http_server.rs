use axum::{
    Json, Router,
    extract::{Path, Query, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use cloud_core::{
    CloudCore,
    groups::group::{Group, GroupRole},
    instances::{instance::Instance, instance_manager::InstanceManager, instance_runtime::InstanceInfo},
    logger::logger::{LogLevel, log},
    messaging::{
        cloudmessage::{CloudMessage, CloudPacket, handle_bridge_socket},
        cloudplayer::CloudPlayer,
    },
    templates::template::Template,
    util::software::softwaremanager::ServerSoftware,
};
use serde::Deserialize;
use std::{
    io::{Error, ErrorKind},
    sync::Arc,
};
use subtle::ConstantTimeEq;

#[derive(Deserialize)]
struct GroupRoleQuery {
    role: GroupRole,
}

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

async fn shutdown(State(core): State<Arc<CloudCore>>) -> StatusCode {
    core.request_shutdown();
    StatusCode::ACCEPTED
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

async fn get_template(State(core): State<Arc<CloudCore>>, Path((group, name)): Path<(String, String)>) -> Result<Json<Template>, StatusCode> {
    let template = core.template_manager().get_template(&group, &name).await;

    match template {
        Some(template) => Ok(Json(template)),
        None => Err(StatusCode::NOT_FOUND),
    }
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

async fn get_instance(State(core): State<Arc<CloudCore>>, Path(id): Path<String>) -> Result<Json<InstanceInfo>, StatusCode> {
    let instance_info = core.instance_manager().get_instance(&id).await;

    match instance_info {
        Some(instance_info) => Ok(Json(instance_info)),
        None => Err(StatusCode::NOT_FOUND),
    }
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

async fn create_group(State(core): State<Arc<CloudCore>>, Path(name): Path<String>, Query(query): Query<GroupRoleQuery>) -> Result<StatusCode, StatusCode> {
    core.create_group(&name, query.role).await.map_err(error_status)?;
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

fn authorize_bridge(core: &CloudCore, headers: &HeaderMap) -> Result<(), StatusCode> {
    let expected_token = core.bridge_secret();

    let authorized = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|token| bool::from(token.as_bytes().ct_eq(expected_token.as_bytes())));

    if expected_token.is_empty() || !authorized {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(())
}

async fn players(State(core): State<Arc<CloudCore>>, headers: HeaderMap) -> Result<Json<Vec<CloudPlayer>>, StatusCode> {
    authorize_bridge(&core, &headers)?;
    Ok(Json(core.bridge_messaging().players().all().await))
}

async fn get_player(State(core): State<Arc<CloudCore>>, headers: HeaderMap, Path(uuid): Path<String>) -> Result<Json<CloudPlayer>, StatusCode> {
    authorize_bridge(&core, &headers)?;
    core.bridge_messaging().players().get(&uuid).await.map(Json).ok_or(StatusCode::NOT_FOUND)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TransferPlayer {
    server: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DisconnectPlayer {
    reason: String,
}

async fn transfer_player(
    State(core): State<Arc<CloudCore>>,
    headers: HeaderMap,
    Path(uuid): Path<String>,
    Json(request): Json<TransferPlayer>,
) -> Result<StatusCode, StatusCode> {
    authorize_bridge(&core, &headers)?;
    let destination = core.instance_manager().get_instance(&request.server).await.ok_or(StatusCode::NOT_FOUND)?;
    if destination.backend_address().is_none() {
        return Err(StatusCode::CONFLICT);
    }
    send_player_command(
        &core,
        &uuid,
        CloudPacket::TransferPlayer,
        serde_json::json!({"uuid": uuid, "server": request.server}),
    )
    .await
}

async fn disconnect_player(
    State(core): State<Arc<CloudCore>>,
    headers: HeaderMap,
    Path(uuid): Path<String>,
    Json(request): Json<DisconnectPlayer>,
) -> Result<StatusCode, StatusCode> {
    authorize_bridge(&core, &headers)?;
    if request.reason.len() > 4096 {
        return Err(StatusCode::BAD_REQUEST);
    }
    send_player_command(
        &core,
        &uuid,
        CloudPacket::DisconnectPlayer,
        serde_json::json!({"uuid": uuid, "reason": request.reason}),
    )
    .await
}

async fn send_player_command(core: &CloudCore, uuid: &str, packet: CloudPacket, payload: serde_json::Value) -> Result<StatusCode, StatusCode> {
    let player = core.bridge_messaging().players().get(uuid).await.ok_or(StatusCode::NOT_FOUND)?;
    let id = format!(
        "player-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .as_nanos()
    );
    core.bridge_messaging()
        .send(&player.proxy_name, CloudMessage { id, packet, payload })
        .await
        .map_err(|error| match error.kind() {
            ErrorKind::NotConnected | ErrorKind::WouldBlock => StatusCode::SERVICE_UNAVAILABLE,
            ErrorKind::InvalidInput => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;
    Ok(StatusCode::ACCEPTED)
}

async fn bridge_handler(State(core): State<Arc<CloudCore>>, ws: WebSocketUpgrade, headers: HeaderMap) -> Response {
    if let Err(status) = authorize_bridge(&core, &headers) {
        return status.into_response();
    }
    let Some(instance) = headers.get("X-Instance-Id").and_then(|value| value.to_str().ok()).map(str::to_owned) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Some(info) = core.instance_manager().get_instance(&instance).await else {
        return StatusCode::FORBIDDEN.into_response();
    };
    if !matches!(
        info.status(),
        cloud_core::instances::instance_runtime::InstanceStatus::Starting | cloud_core::instances::instance_runtime::InstanceStatus::Running
    ) {
        return StatusCode::FORBIDDEN.into_response();
    }
    ws.max_message_size(64 * 1024)
        .max_frame_size(64 * 1024)
        .on_upgrade(move |socket| handle_bridge_socket(socket, core, instance))
}

pub fn router(core: Arc<CloudCore>) -> Router {
    Router::new()
        .route("/bridge", get(bridge_handler))
        .route("/players", get(players))
        .route("/players/{uuid}", get(get_player))
        .route("/players/{uuid}/transfer", post(transfer_player))
        .route("/players/{uuid}/disconnect", post(disconnect_player))
        .route("/health", get(health))
        .route("/reload", post(reload))
        .route("/shutdown", post(shutdown))
        .route("/paths", get(paths))
        .route("/groups", get(groups))
        .route("/groups/{name}", get(group).post(create_group).delete(delete_group))
        .route("/groups/{name}/maintenance", post(enable_group_maintenance).delete(disable_group_maintenance))
        .route("/templates", get(templates))
        .route(
            "/templates/{group_name}/{template_name}",
            get(get_template).post(create_template).delete(delete_template),
        )
        .route("/instances", get(instances))
        .route("/instances/new/{group_name}/{template_name}", post(create_instance_from_template))
        .route("/instances/{id}", get(get_instance))
        .route("/instances/{id}/remove", delete(delete_instance))
        .route("/instances/{id}/start", post(start_instance))
        .route("/instances/{id}/stop", post(stop_instance))
        .route("/instances/{id}/console", get(instance_console))
        .with_state(core)
}

pub async fn start_http_server(core: Arc<CloudCore>) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    let shutdown = Arc::clone(&core);
    axum::serve(listener, router(core))
        .with_graceful_shutdown(async move { shutdown.wait_for_shutdown().await })
        .await
        .map_err(Error::other)
}

#[cfg(test)]
mod player_tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn player_routes_require_authentication_and_return_json_or_not_found() {
        let directory = tempfile::tempdir().unwrap();
        let mut core = CloudCore::new(directory.path()).await.unwrap();
        core.set_bridge_secret("test-secret".into()).await;
        let app = router(Arc::new(core));
        for authorization in [None, Some("Bearer incorrect")] {
            let mut request = Request::builder().uri("/players");
            if let Some(value) = authorization {
                request = request.header(AUTHORIZATION, value);
            }
            assert_eq!(
                app.clone().oneshot(request.body(Body::empty()).unwrap()).await.unwrap().status(),
                StatusCode::UNAUTHORIZED
            );
        }
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/players")
                    .header(AUTHORIZATION, "Bearer test-secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(&to_bytes(response.into_body(), 1024).await.unwrap()[..], b"[]");
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/players/c5c8f17d-cd99-4d5d-933f-b732fe6bd194")
                    .header(AUTHORIZATION, "Bearer test-secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
