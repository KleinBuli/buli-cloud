use cloud_core::{
    groups::group::{Group, GroupRole},
    logger::logger::{
        LogLevel::{self, Error, Info},
        log,
    },
};
use reqwest::Client;

pub(super) async fn list(http_client: &Client, url: &str) -> Result<bool, reqwest::Error> {
    let response = http_client.get(format!("{}groups", url)).send().await?;
    let status = response.status();
    let body = response.text().await?;

    if status.is_success() {
        let groups: Vec<Group> = serde_json::from_str(&body).expect("Invalid JSON");

        for group in groups {
            let templates = group.template_names().join(", ");
            let line = format!("{} | Templates: [{}]", group.name(), templates);
            log(Info, &line);
        }
    } else {
        log(Error, &format!("Error while requesting groups: {} - {}", body, status))
    }

    return Ok(true);
}

pub(super) async fn create(http_client: &Client, url: &str, name: String, role: GroupRole) -> Result<bool, reqwest::Error> {
    let response = http_client.post(format!("{}groups/{}?role={}", url, name, role.to_string())).send().await?;
    let status = response.status();
    let body = response.text().await?;

    if status.is_success() {
        log(Info, &format!("Group {} created successfully", name));
    } else {
        log(LogLevel::Error, &format!("Error while creating group {}: {} - {}", name, body, status));
    }

    return Ok(true);
}
