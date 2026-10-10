use std::{
    fs::{self},
    io::Error,
    path::PathBuf,
    sync::RwLock,
};

use crate::{
    config::cloud_config::Config,
    logger::logger::{
        LogLevel::{Info, Warn},
        log,
    },
};

pub struct ConfigManager {
    config_path: PathBuf,
    config: RwLock<Config>,
}

impl ConfigManager {
    pub async fn new(config_path: PathBuf) -> Result<Self, Error> {
        if !fs::exists(&config_path)? {
            let default_config = Config::default();
            let config_manager = Self {
                config_path,
                config: RwLock::new(default_config),
            };

            log(Warn, "config.toml not found. Creating default configuration.");
            config_manager.save_config().await?;
            return Ok(config_manager);
        }

        let config_content = tokio::fs::read_to_string(&config_path).await?;
        let config = toml::from_str::<Config>(&config_content).map_err(std::io::Error::other)?;
        log(Info, "Loaded config.toml");
        Ok(Self {
            config_path,
            config: RwLock::new(config),
        })
    }

    pub async fn from(config_path: PathBuf, config: Config) -> Result<Self, Error> {
        let config_manager = Self {
            config_path,
            config: RwLock::new(config),
        };
        Ok(config_manager)
    }

    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    pub fn config(&self) -> Config {
        self.config.read().unwrap().clone()
    }

    pub(crate) fn replace_config(&self, config: Config) {
        *self.config.write().unwrap() = config;
    }

    pub(crate) async fn read_config(&self) -> Result<Config, Error> {
        let content = tokio::fs::read_to_string(&self.config_path).await?;
        toml::from_str(&content).map_err(Error::other)
    }

    pub fn config_exists(&self) -> Result<bool, Error> {
        fs::exists(&self.config_path)
    }

    pub async fn reload_config(&self) -> Result<(), Error> {
        self.replace_config(self.read_config().await?);
        log(Info, "Reloaded config.toml");

        Ok(())
    }

    pub async fn save_config(&self) -> Result<(), Error> {
        if let Some(parent) = self.config_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let config_toml = toml::to_string_pretty(&self.config()).map_err(std::io::Error::other)?;
        crate::util::file_utils::atomic_write(&self.config_path, config_toml.as_bytes())?;
        log(Info, "config.toml was saved.");
        Ok(())
    }
}
