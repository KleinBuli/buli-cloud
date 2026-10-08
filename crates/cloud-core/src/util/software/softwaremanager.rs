use serde::{Deserialize, Serialize};
use sha256::digest;
use std::{
    collections::HashMap,
    fmt,
    fs::File,
    io::{Error, Read},
    path::PathBuf,
};
use tokio::fs;
use zip::ZipArchive;

use crate::logger::logger::{LogLevel::Info, log};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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

#[derive(serde::Deserialize)]
struct PaperProject {
    versions: std::collections::HashMap<String, Vec<String>>,
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

    pub async fn ensure_mojang_mapping(&self, minecraft_version: &str) -> Result<(), Error> {
        if !self.paper_exists(minecraft_version) {
            self.ensure_paper_available(minecraft_version).await?;
        }

        log(Info, "Checking cached mojang-mapping...");
        let file_name = format!("mojang_{}.jar", minecraft_version);
        let file_path = self.download_path.join("cache").join(file_name);

        if file_path.exists() {
            log(Info, "Mojang .jar already cached and ready-to-start.");
            return Ok(());
        }

        log(
            Info,
            &format!("No mojang .jar found for {minecraft_version}. Trying to download mojang_{minecraft_version}.jar..."),
        );

        self.handle_mojang_download(minecraft_version).await?;

        Ok(())
    }

    pub async fn ensure_paper_available(&self, minecraft_version: &str) -> Result<(), Error> {
        log(Info, "Checking cached PaperMC .jar...");

        let file_name = format!("paper-{}.jar", minecraft_version);

        if self.paper_exists(minecraft_version) {
            log(Info, "PaperMC Jar already cached and ready-to-start.");
            return Ok(());
        }

        let file_path = self.download_path.join(file_name);
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

    pub async fn ensure_velocity_available(&self) -> Result<(), Error> {
        log(Info, "Checking cached Velocity .jar...");

        let file_path = self.download_path.join("velocity.jar");

        if file_path.try_exists()? {
            log(Info, "Velocity jar already cached.");
            return Ok(());
        }

        let client = reqwest::Client::builder()
            .user_agent("BuliCloud/0.1 (https://github.com/KleinBuli/buli-cloud)")
            .build()
            .map_err(Error::other)?;

        let project = client
            .get("https://fill.papermc.io/v3/projects/velocity")
            .send()
            .await
            .map_err(Error::other)?
            .error_for_status()
            .map_err(Error::other)?
            .json::<PaperProject>()
            .await
            .map_err(Error::other)?;

        let mut versions: Vec<String> = project
            .versions
            .into_values()
            .flatten()
            .filter(|version| !version.contains('-'))
            .collect();

        fn version_key(version: &str) -> Result<Vec<u32>, Error> {
            version.split('.').map(|part| part.parse::<u32>().map_err(Error::other)).collect()
        }

        let mut sorted_versions = versions
            .drain(..)
            .map(|version| {
                let key = version_key(&version)?;
                Ok((key, version))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        sorted_versions.sort_by(|a, b| b.0.cmp(&a.0));

        for (_, version) in sorted_versions {
            let downloaded = self
                .handle_paper_project_download("velocity", &version, &client, &file_path)
                .await?;

            if downloaded {
                log(Info, &format!("Downloaded Velocity {version} successfully."));
                return Ok(());
            }
        }

        Err(Error::other("No stable Velocity build found"))
    }

    pub async fn ensure_vanilla_available() {}

    async fn handle_paper_project_download(
        &self,
        server_software: &str,
        version: &str,
        client: &reqwest::Client,
        destination: &std::path::Path,
    ) -> Result<bool, Error> {
        let builds = client
            .get(format!(
                "https://fill.papermc.io/v3/projects/{server_software}/versions/{version}/builds"
            ))
            .send()
            .await
            .map_err(Error::other)?
            .error_for_status()
            .map_err(Error::other)?
            .json::<Vec<PaperBuild>>()
            .await
            .map_err(Error::other)?;

        let Some(build) = builds.iter().find(|build| build.channel == "STABLE") else {
            return Ok(false);
        };

        let download = build
            .downloads
            .get("server:default")
            .ok_or_else(|| Error::other("No server download found"))?;

        let bytes = client
            .get(&download.url)
            .send()
            .await
            .map_err(Error::other)?
            .error_for_status()
            .map_err(Error::other)?
            .bytes()
            .await
            .map_err(Error::other)?;

        tokio::fs::create_dir_all(&self.download_path).await?;
        tokio::fs::write(destination, bytes).await?;

        Ok(true)
    }

    async fn handle_mojang_download(&self, minecraft_version: &str) -> Result<(), Error> {
        let paper_path = self.download_path.join(&format!("paper-{minecraft_version}.jar"));
        let paper_jar = File::open(paper_path)?;
        let mut archive = ZipArchive::new(paper_jar)?;

        let mut entry = archive.by_name("META-INF/download-context")?;
        let mut content = String::new();
        entry.read_to_string(&mut content)?;

        let mut parts = content.trim().split("\t");
        let checksum = parts.next().ok_or("Prüfsumme fehlt").map_err(std::io::Error::other)?;
        let url = parts.next().ok_or("URL fehlt").map_err(std::io::Error::other)?;

        fs::create_dir_all(self.download_path.join("cache")).await?;
        let file_name = parts.next().ok_or("Dateiname fehlt").map_err(std::io::Error::other)?;
        let file_path = self.download_path.join("cache").join(file_name);

        match tokio::fs::read(&file_path).await {
            Ok(bytes) => {
                if digest(&bytes) == checksum {
                    return Ok(());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }

        let client = reqwest::Client::new();
        let bytes = client
            .get(url)
            .send()
            .await
            .map_err(Error::other)?
            .error_for_status()
            .map_err(Error::other)?
            .bytes()
            .await
            .map_err(Error::other)?;

        let downloaded_sha256 = digest(bytes.as_ref());
        if downloaded_sha256 != checksum {
            return Err(Error::new(std::io::ErrorKind::InvalidData, "SHA-256 checksum mismatch"));
        }

        tokio::fs::write(file_path, bytes).await?;
        log(Info, "Downloaded mojang mappings successfully.");

        Ok(())
    }

    fn paper_exists(&self, minecraft_version: &str) -> bool {
        let file_path = self.download_path.join(minecraft_version);
        return file_path.exists();
    }
}
