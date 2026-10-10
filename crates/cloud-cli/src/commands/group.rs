use cloud_core::logger::logger::{
    LogLevel::{self, Error, Info},
    log,
};
use reqwest::Client;

pub(super) async fn list(http_client: &Client, url: &str) -> Result<bool, reqwest::Error> {
    let response = http_client.get(format!("{}groups/", url)).send().await?;
    let status = response.status();
    let body = response.text().await?;

    if status.is_success() {
        log(Info, &body.to_string())
    } else {
        log(Error, &format!("Error while requesting groups: {} - {}", body, status))
    }

    return Ok(true);
}

pub(super) async fn create(http_client: &Client, url: &str, name: String) -> Result<bool, reqwest::Error> {
    let response = http_client.post(format!("{}groups/{}", url, name)).send().await?;
    let status = response.status();
    let body = response.text().await?;

    if status.is_success() {
        log(Info, &format!("Group {} created successfully", name));
    } else {
        log(LogLevel::Error, &format!("Error while creating group {}: {} - {}", name, body, status));
    }

    return Ok(true);
}
