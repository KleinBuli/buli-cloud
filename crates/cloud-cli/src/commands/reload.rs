use cloud_core::logger::logger::{
    LogLevel::{self, Info},
    log,
};
use reqwest::{Client, Error};

pub async fn reload(http_client: &Client, url: &str) -> Result<bool, Error> {
    let response = http_client.post(format!("{}reload", url)).send().await?;
    let status = response.status();
    let body = response.text().await?;

    if status.is_success() {
        log(Info, "Reloaded successfully.");
    } else {
        log(LogLevel::Error, &format!("Error while reloading: {} - {}", status, body));
    }

    Ok(true)
}
