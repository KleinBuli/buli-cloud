use std::{
    fs::{self},
    io::Error,
    path::PathBuf,
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
    config: Config,
}

impl ConfigManager {
    pub async fn new(config_path: PathBuf) -> Result<Self, Error> {
        if !fs::exists(&config_path)? {
            let default_config = Config::default();
            let config_manager = Self {
                config_path,
                config: default_config,
            };

            log(Warn, "config.toml not found. Creating default configuration.");
            return Ok(config_manager);
        }

        let config_content = tokio::fs::read_to_string(&config_path).await?;
        let config = toml::from_str::<Config>(&config_content).map_err(std::io::Error::other)?;
        log(Info, "Loaded config.toml");
        Ok(Self { config_path, config })
    }

    pub async fn from(config_path: PathBuf, config: Config) -> Result<Self, Error> {
        let config_manager = Self { config_path, config };
        config_manager.save_config().await?;
        Ok(config_manager)
    }

    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn config_exists(&self) -> Result<bool, Error> {
        fs::exists(&self.config_path)
    }

    pub async fn reload_config(&mut self) -> Result<(), Error> {
        let config_content = tokio::fs::read_to_string(&self.config_path).await?;
        let config = toml::from_str::<Config>(&config_content).map_err(std::io::Error::other)?;

        self.config = config;
        log(Info, "Reloaded config.toml");

        Ok(())
    }

    pub async fn save_config(&self) -> Result<(), Error> {
        let config_toml = toml::to_string_pretty(&self.config).map_err(std::io::Error::other)?;
        tokio::fs::write(&self.config_path, config_toml).await?;
        log(Info, "config.toml was saved.");
        Ok(())
    }
}
