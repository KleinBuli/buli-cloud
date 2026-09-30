use std::{
    fs::{self},
    path::PathBuf,
};

use crate::{
    instances::{instance::Instance, instance_manager::InstanceManager},
    templates::template_manager::TemplateManager,
    util::file_utils::copy_dir_all,
};

/// Central runtime core of BuliCloud.
///
/// The `CloudCore` manages the root directory and provides access
/// to the main runtime paths used by the cloud system.
pub struct CloudCore {
    root_path: PathBuf,
    template_manager: TemplateManager,
    instance_manager: InstanceManager,
}

impl CloudCore {
    /// Create new `CloudCore` and initializes Managers.
    ///
    /// # Arguments
    /// * `root_path` - root directory for buli cloud
    ///
    /// # Returns
    /// * new CloudCore instance
    pub fn new<P: Into<PathBuf>>(root_path: P) -> Self {
        let root_path = root_path.into();
        Self {
            template_manager: TemplateManager::new(root_path.join("templates")),
            instance_manager: InstanceManager::new(),
            root_path,
        }
    }

    pub fn template_manager(&self) -> &TemplateManager {
        &self.template_manager
    }

    pub fn instance_manager(&self) -> &InstanceManager {
        &self.instance_manager
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

    pub async fn prepare_instance(&self, instance: &Instance) -> Result<(), std::io::Error> {
        match instance.template_name() {
            Some(template_name) => {
                let template = self
                    .template_manager()
                    .get_template(template_name)
                    .await
                    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, format!("Template {template_name} not found")))?;

                let instance_path = self.running_path().join(instance.id());
                let template_path = self.templates_path().join(template.name());

                copy_dir_all(template_path, instance_path)?;

                Ok(())
            }

            // TODO: static instances
            None => Ok(()),
        }
    }

    /// initalizes the CloudCore
    /// # Returns
    /// * Result<()>
    pub fn initialize(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.templates_path())?;
        fs::create_dir_all(self.config_path())?;
        fs::create_dir_all(self.static_servers_path())?;
        fs::create_dir_all(self.running_path())?;

        Ok(())
    }
}
