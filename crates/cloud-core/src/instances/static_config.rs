use std::{
    io::{Error, ErrorKind},
    path::Path,
};

use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Serialize, Deserialize, Clone)]
pub struct StaticInstanceConfig {
    group: String,
    template: String,
}

impl StaticInstanceConfig {
    pub fn new(group: String, template: String) -> Self {
        Self { group, template }
    }

    pub fn group(&self) -> &str {
        &self.group
    }

    pub fn template(&self) -> &str {
        &self.template
    }

    pub async fn load(config_path: &Path) -> Result<Self, Error> {
        let content = fs::read_to_string(config_path).await?;

        let config: Self = toml::from_str(&content).map_err(Error::other)?;

        Ok(config)
    }

    pub async fn save(&self, config_path: &Path) -> Result<(), Error> {
        let content = toml::to_string_pretty(&self).map_err(Error::other)?;

        if config_path.exists() {
            return Err(Error::new(ErrorKind::NotFound, "config path not found"));
        }

        fs::write(config_path, content).await
    }
}
