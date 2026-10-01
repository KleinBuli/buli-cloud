use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::{self, FileType},
    io::{Error, ErrorKind},
    path::PathBuf,
};

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

    pub async fn download_paper(&self, minecraft_version: &str) -> Result<(), Error> {
        let file_name = format!("paper-{}.jar", minecraft_version);

        let exists = fs::read_dir(&self.download_path)?
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy() == file_name);

        if exists {
            return Ok(());
        }

        let client = reqwest::Client::new();

        let builds = client
            .get(format!(
                "https://fill.papermc.io/v3/projects/paper/versions/{}/builds",
                minecraft_version
            ))
            .header("User-Agent", "BuliCloud/0.1")
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

        tokio::fs::write(&format!("paper-{}.jar", minecraft_version), bytes).await?;

        Ok(())
    }

    pub async fn download_velocity() {}
    pub async fn download_vanilla() {}
}
