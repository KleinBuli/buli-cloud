use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, io::Error, path::PathBuf};

use crate::logger::logger::{LogLevel::Info, log};

#[derive(Serialize, Clone)]
pub enum ServerSoftware {
    Paper,
    Vanilla,
    Velocity,
}

impl fmt::Display for ServerSoftware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerSoftware::Paper => write!(f, "paper"),
            ServerSoftware::Vanilla => write!(f, "vanilla"),
            ServerSoftware::Velocity => write!(f, "velocity"),
        }
    }
}
pub struct ServerSoftwareManager {
    download_path: PathBuf,
}

#[derive(Deserialize)]
struct PaperBuild {
    channel: String,
    downloads: HashMap<String, PaperDownload>,
}

#[derive(Deserialize)]
struct PaperDownload {
    url: String,
}

impl ServerSoftwareManager {
    pub fn new(download_path: PathBuf) -> Self {
        Self { download_path }
    }

    pub fn download_path(&self) -> &PathBuf {
        &self.download_path
    }

    pub async fn ensure_paper_available(&self, minecraft_version: &str) -> Result<(), Error> {
        log(Info, "Checking cached PaperMC .jar...");

        let file_name = format!("paper-{}.jar", minecraft_version);
        let file_path = self.download_path.join(file_name);

        if file_path.exists() {
            log(Info, "PaperMC Jar already cached and ready-to-start.");
            return Ok(());
        }

        log(Info, "No fallback .jar found. Trying to download papermc...");
        let client = reqwest::Client::new();

        let builds = client
            .get(format!(
                "https://fill.papermc.io/v3/projects/paper/versions/{}/builds",
                minecraft_version
            ))
            .header("User-Agent", "BuliCloud/0.1 (https://github.com/KleinBuli/buli-cloud)")
            .send()
            .await
            .map_err(|e| Error::other(e))?
            .json::<Vec<PaperBuild>>()
            .await
            .map_err(|e| Error::other(e))?;

        let build = builds
            .iter()
            .find(|build| build.channel == "STABLE")
            .ok_or_else(|| Error::other("No stable Paper build found"))?;

        let url = &build
            .downloads
            .get("server:default")
            .ok_or_else(|| Error::other("No server download found"))?
            .url;

        let bytes = client
            .get(url)
            .send()
            .await
            .map_err(|e| Error::other(e))?
            .bytes()
            .await
            .map_err(|e| Error::other(e))?;

        tokio::fs::write(file_path, bytes).await?;
        log(Info, "Downloaded papermc successfully.");

        Ok(())
    }

    pub async fn download_velocity() {}
    pub async fn download_vanilla() {}
}
