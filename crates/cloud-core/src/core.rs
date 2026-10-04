use std::{
    fs::{self, remove_dir_all},
    io::Error,
    path::PathBuf,
    process::Stdio,
};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

use crate::{
    config::config_manager::ConfigManager,
    instances::{instance::Instance, instance_manager::InstanceManager},
    logger::logger::{LogLevel::Info, log},
    templates::template_manager::TemplateManager,
    util::{file_utils::copy_dir_all, software::softwaremanager::ServerSoftwareManager},
};

/// Central runtime core of BuliCloud.
///
/// The `CloudCore` manages the root directory and provides access
/// to the main runtime paths used by the cloud system.
pub struct CloudCore {
    root_path: PathBuf,
    template_manager: TemplateManager,
    instance_manager: InstanceManager,
    config_manager: ConfigManager,
    server_software_manager: ServerSoftwareManager,
}

impl CloudCore {
    /// Create new `CloudCore` and initializes Managers.
    ///
    /// # Arguments
    /// * `root_path` - root directory for buli cloud
    ///
    /// # Returns
    /// * new CloudCore instance
    pub async fn new<P: Into<PathBuf>>(root_path: P) -> Result<Self, Error> {
        let root_path = root_path.into();
        let config_manager = ConfigManager::new(root_path.join("config/config.toml")).await?;

        Ok(Self {
            template_manager: TemplateManager::new(root_path.join("templates"), root_path.join("cache")),
            instance_manager: InstanceManager::new(root_path.join("running")),
            config_manager,
            server_software_manager: ServerSoftwareManager::new(root_path.join("cache")),
            root_path,
        })
    }

    pub fn template_manager(&self) -> &TemplateManager {
        &self.template_manager
    }

    pub fn instance_manager(&self) -> &InstanceManager {
        &self.instance_manager
    }

    pub fn server_software_manager(&self) -> &ServerSoftwareManager {
        &self.server_software_manager
    }

    pub fn config_manager(&self) -> &ConfigManager {
        &self.config_manager
    }

    /// Returns the path to the templates directory
    pub fn templates_path(&self) -> PathBuf {
        self.root_path.join("templates")
    }

    /// Returns the path to the running services directory
    pub fn running_path(&self) -> PathBuf {
        self.root_path.join("running")
    }

    /// Returns the path to the config directory
    pub fn config_path(&self) -> PathBuf {
        self.root_path.join("config")
    }

    /// Returns the path to the static servers directory
    pub fn static_servers_path(&self) -> PathBuf {
        self.root_path.join("static-servers")
    }

    // Returns the path to the cache directory
    pub fn cache_path(&self) -> PathBuf {
        self.root_path.join("cache")
    }

    pub async fn prepare_instance_starting(&self, instance: &Instance) -> Result<(), std::io::Error> {
        match instance.template_name() {
            Some(template_name) => {
                let template = self
                    .template_manager()
                    .get_template(template_name)
                    .await
                    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, format!("Template {template_name} not found")))?;

                let instance_path = self.running_path().join(instance.id());
                let template_path = self.templates_path().join(template.name());

                if instance_path.exists() {
                    return Err(Error::new(std::io::ErrorKind::AlreadyExists, "Instance directory already exists."));
                }

                let instance_software = format!(
                    "{}-{}.jar",
                    template.server_software().to_string(),
                    template.minecraft_version().clone().unwrap()
                );

                copy_dir_all(template_path, &instance_path)?;

                let mut child = Command::new("java")
                    .arg("-Dcom.mojang.eula.agree=true")
                    .arg("-jar")
                    .arg(&instance_software)
                    .arg("--nogui")
                    .current_dir(&instance_path)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?;

                if let Some(stdout) = child.stdout.take() {
                    tokio::spawn(async move {
                        let reader = BufReader::new(stdout);
                        let mut lines = reader.lines();

                        while let Ok(Some(line)) = lines.next_line().await {
                            log(Info, &line);
                        }
                    });
                }
                Ok(())
            }

            // TODO: static instances
            None => Ok(()),
        }
    }

    pub async fn prepare_instance_stopping(&self, instance: &Instance) -> Result<(), Error> {
        let instance_path = self.running_path().join(instance.id());
        self.instance_manager
            .stop_instance(instance.id())
            .await
            .map_err(|_| Error::new(std::io::ErrorKind::NotFound, format!("Instance {} not found", instance.id())))?;

        remove_dir_all(instance_path)?;

        self.instance_manager
            .remove_instance(instance.id())
            .await
            .map_err(|_| Error::new(std::io::ErrorKind::Other, format!("Could not remove instance {}", instance.id())))?;

        Ok(())
    }

    /// initalizes the CloudCore
    /// # Returns
    /// * Result<()>
    pub async fn initialize(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.config_path())?;
        fs::create_dir_all(self.templates_path())?;
        fs::create_dir_all(self.static_servers_path())?;
        fs::create_dir_all(self.running_path())?;
        fs::create_dir_all(self.cache_path())?;

        let version = self.config_manager.config().fallback_minecraft_version();
        self.server_software_manager.ensure_paper_available(version).await?;
        self.template_manager().load_templates().await?;
        Ok(())
    }
}
