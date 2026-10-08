use std::{
    io::{Error, ErrorKind},
    path::PathBuf,
};

use tokio::{fs, sync::RwLock};

use crate::{
    instances::static_instances::{StaticInstanceConfig, StaticInstancesConfig},
    util::file_utils::validate_name,
};

pub struct StaticInstanceManager {
    static_path: PathBuf,
    config_path: PathBuf,
    config: RwLock<StaticInstancesConfig>,
}

pub enum StaticInstanceType {
    Server,
    Proxy,
}

impl StaticInstanceManager {
    pub fn new(static_path: PathBuf) -> Self {
        let config_path = static_path.join("static_instances.toml");
        Self {
            static_path,
            config_path,
            config: RwLock::new(StaticInstancesConfig::default()),
        }
    }

    pub fn static_path(&self) -> &PathBuf {
        &self.static_path
    }

    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    pub fn config(&self) -> &RwLock<StaticInstancesConfig> {
        &self.config
    }

    pub async fn create_server(&self, name: String, config: StaticInstanceConfig) -> Result<(), Error> {
        self.create_static_instance(name, config, StaticInstanceType::Server).await
    }

    pub async fn create_proxy(&self, name: String, config: StaticInstanceConfig) -> Result<(), Error> {
        self.create_static_instance(name, config, StaticInstanceType::Proxy).await
    }

    async fn create_static_instance(
        &self,
        name: String,
        config: StaticInstanceConfig,
        instance_type: StaticInstanceType,
    ) -> Result<(), Error> {
        validate_name(&name)?;
        self.validate_files().await?;

        let folder = match instance_type {
            StaticInstanceType::Server => "server",
            StaticInstanceType::Proxy => "proxy",
        };

        let path = self.static_path.join(folder).join(&name);

        if config.min_memory_mb() > config.max_memory_mb() {
            return Err(Error::new(ErrorKind::InvalidInput, "Minimum memory cannot exceed maximum memory"));
        }

        {
            let cfg = self.config.read().await;

            let instances = match instance_type {
                StaticInstanceType::Server => &cfg.server,
                StaticInstanceType::Proxy => &cfg.proxy,
            };

            if instances.keys().any(|n| n.eq_ignore_ascii_case(&name)) {
                return Err(Error::new(ErrorKind::AlreadyExists, "Static instance already exists"));
            }
        }

        if path.try_exists()? {
            return Err(Error::new(ErrorKind::AlreadyExists, "Static instance directory already exists"));
        }

        fs::create_dir(&path).await?;

        {
            let mut cfg = self.config.write().await;

            let instances = match instance_type {
                StaticInstanceType::Server => &mut cfg.server,
                StaticInstanceType::Proxy => &mut cfg.proxy,
            };

            instances.insert(name.clone(), config);
        }

        if let Err(save_error) = self.save().await {
            {
                let mut cfg = self.config.write().await;

                let instances = match instance_type {
                    StaticInstanceType::Server => &mut cfg.server,
                    StaticInstanceType::Proxy => &mut cfg.proxy,
                };

                instances.remove(&name);
            }

            if let Err(rollback_error) = fs::remove_dir(&path).await {
                return Err(Error::other(format!(
                    "Failed to save static instance: {save_error}. Directory rollback also failed: {rollback_error}"
                )));
            }

            return Err(save_error);
        }

        Ok(())
    }

    pub async fn get_server(&self, name: String) -> Option<StaticInstanceConfig> {
        let config = self.config.read().await;

        config
            .server
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(&name))
            .map(|(_, instance)| instance.clone())
    }

    pub async fn get_proxy(&self, name: String) -> Option<StaticInstanceConfig> {
        let config = self.config.read().await;

        config
            .proxy
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(&name))
            .map(|(_, instance)| instance.clone())
    }

    pub async fn all(&self) -> Vec<(StaticInstanceType, String, StaticInstanceConfig)> {
        let config = self.config.read().await;
        config
            .server
            .iter()
            .map(|(name, cfg)| (StaticInstanceType::Server, name.clone(), cfg.clone()))
            .chain(
                config
                    .proxy
                    .iter()
                    .map(|(name, cfg)| (StaticInstanceType::Proxy, name.clone(), cfg.clone())),
            )
            .collect()
    }

    pub fn remove_server() {}
    pub fn remove_proxy() {}

    pub async fn save(&self) -> Result<(), Error> {
        self.validate_files().await?;

        let content = {
            let config = self.config.read().await;
            toml::to_string_pretty(&*config).map_err(Error::other)?
        };

        fs::write(self.config_path(), content).await?;

        Ok(())
    }

    pub async fn load(&self) -> Result<(), Error> {
        self.validate_files().await?;

        let content = tokio::fs::read_to_string(&self.config_path).await?;
        let config: StaticInstancesConfig = toml::from_str(&content).map_err(std::io::Error::other)?;

        *self.config.write().await = config;

        Ok(())
    }

    async fn validate_files(&self) -> Result<(), Error> {
        fs::create_dir_all(self.static_path.join("proxy")).await?;
        fs::create_dir_all(self.static_path.join("server")).await?;

        if !self.config_path.exists() {
            let config = StaticInstancesConfig::default();
            let content = toml::to_string_pretty(&config).map_err(std::io::Error::other)?;
            tokio::fs::write(&self.config_path, content).await?;
        }

        Ok(())
    }
}
