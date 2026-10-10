use cloud_core::logger::logger::{LogLevel, LogLevel::Info, log};
use reqwest::{Client, Error};

pub(super) async fn list(http_client: &Client, url: &str) -> Result<bool, Error> {
    let response = http_client.post(format!("{}templates/", url)).send().await?;
    let status = response.status();
    let body = response.text().await?;

    if status.is_success() {
        log(Info, &body.to_string())
    } else {
        log(LogLevel::Error, &format!("Error while requesting groups: {} - {}", body, status))
    }

    Ok(true)
}

pub(super) async fn create(http_client: &Client, url: &str, group: String, name: String, proxy: bool) -> Result<bool, reqwest::Error> {
    let response = http_client
        .post(format!(
            "{}templates/{}/{}?software={}",
            url,
            group,
            name,
            if proxy { "Velocity" } else { "Paper" }
        ))
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;

    if status.is_success() {
        log(Info, &format!("Created Template: {}", name));
    } else {
        log(LogLevel::Error, &format!("Error while creating template {}: {} - {}", name, body, status));
    }
    return Ok(true);
}
