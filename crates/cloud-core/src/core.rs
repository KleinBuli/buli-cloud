use crate::{
    config::config_manager::ConfigManager,
    groups::{group::Group, group_manager::GroupManager},
    instances::{instance::Instance, instance_manager::InstanceManager},
    templates::template_manager::{GROUP_CONFIG, TemplateManager},
    util::{
        file_utils::{StagedDirectory, copy_dir_all, validate_name},
        software::softwaremanager::{ServerSoftware, ServerSoftwareManager},
    },
};
use std::{
    fs,
    io::{Error, ErrorKind},
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::{Mutex, MutexGuard};

#[derive(Default)]
struct Lifecycle {
    initialized: bool,
    shutting_down: bool,
}

/// Coordinates mutations across groups, templates and runtime instances.
pub struct CloudCore {
    root_path: PathBuf,
    group_manager: Arc<GroupManager>,
    template_manager: Arc<TemplateManager>,
    instance_manager: Arc<InstanceManager>,
    config_manager: ConfigManager,
    server_software_manager: ServerSoftwareManager,
    operations: Mutex<Lifecycle>,
}

impl CloudCore {
    pub async fn new<P: Into<PathBuf>>(root_path: P) -> Result<Self, Error> {
        let root_path = root_path.into();
        let config_manager = ConfigManager::new(root_path.join("config/config.toml")).await?;
        Ok(Self {
            group_manager: Arc::new(GroupManager::new(root_path.join("config/groups.toml"))?),
            template_manager: Arc::new(TemplateManager::new(root_path.join("templates"))),
            instance_manager: Arc::new(InstanceManager::new(root_path.join("running"))),
            config_manager,
            server_software_manager: ServerSoftwareManager::new(root_path.join("cache/versions")),
            root_path,
            operations: Mutex::new(Lifecycle::default()),
        })
    }

    pub async fn spawn_instance_event_listener(&self) {
        let instance_manager = Arc::clone(&self.instance_manager);
        tokio::spawn(async move {
            instance_manager.handle_runtime_events().await;
        });
    }

    async fn operation(&self) -> Result<MutexGuard<'_, Lifecycle>, Error> {
        let guard = self.operations.lock().await;
        if guard.shutting_down {
            return Err(Error::new(ErrorKind::ResourceBusy, "Cloud is shutting down"));
        }
        if !guard.initialized {
            return Err(Error::new(
                ErrorKind::ResourceBusy,
                "Initialize the cloud before changing its state",
            ));
        }
        Ok(guard)
    }

    pub fn group_manager(&self) -> &GroupManager {
        &self.group_manager
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
    pub fn templates_path(&self) -> PathBuf {
        self.root_path.join("templates")
    }
    pub fn running_path(&self) -> PathBuf {
        self.root_path.join("running")
    }
    pub fn config_path(&self) -> PathBuf {
        self.root_path.join("config")
    }
    pub fn static_servers_path(&self) -> PathBuf {
        self.root_path.join("static-servers")
    }
    pub fn cache_path(&self) -> PathBuf {
        self.root_path.join("cache")
    }

    async fn require_group(&self, name: &str) -> Result<Group, Error> {
        validate_name(name)?;
        self.group_manager
            .get_group(name)
            .await
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Group not found"))
    }

    async fn ensure_global(&self, group: &str) -> Result<(), Error> {
        if !self.template_manager.exists_on_disk(group, "global") {
            self.template_manager
                .create_new_template(
                    group,
                    "global",
                    ServerSoftware::Paper,
                    Some(self.config_manager.config().fallback_minecraft_version().to_string()),
                    None,
                )
                .await?;
        }
        Ok(())
    }

    pub async fn create_group(&self, name: &str) -> Result<(), Error> {
        let _operation = self.operation().await?;
        validate_name(name)?;
        if self
            .group_manager
            .groups()
            .await
            .iter()
            .any(|group| group.name().eq_ignore_ascii_case(name))
        {
            return Err(Error::new(ErrorKind::AlreadyExists, "Group already exists"));
        }
        let global_existed = self.template_manager.exists_on_disk(name, "global");
        self.ensure_global(name).await?;
        self.template_manager
            .load_group_templates(
                name,
                &["global".to_string()],
                self.config_manager.config().fallback_minecraft_version(),
            )
            .await?;
        let group = Group::new(name.to_string(), vec!["global".to_string()], self.templates_path().join(name));
        if let Err(error) = self.group_manager.add_group(group).await {
            if !global_existed {
                StagedDirectory::new(&self.templates_path(), &self.templates_path().join(name).join("global"))?.commit();
            }
            self.template_manager.forget_group(name).await;
            return Err(error);
        }
        Ok(())
    }

    pub async fn remove_group(&self, name: &str) -> Result<(), Error> {
        let _operation = self.operation().await?;
        let group = self.require_group(name).await?;
        if group.template_names().iter().any(|name| name != "global")
            || self
                .instance_manager
                .instances_list()
                .await
                .iter()
                .any(|instance| instance.instance().group_name() == name)
        {
            return Err(Error::new(
                ErrorKind::ResourceBusy,
                "Remove the group's instances and custom templates first",
            ));
        }
        let path = self.templates_path().join(name);
        if path.exists() {
            for entry in fs::read_dir(&path)? {
                let entry = entry?;
                let file_name = entry.file_name();

                if file_name != "global" && file_name != GROUP_CONFIG {
                    return Err(Error::new(ErrorKind::ResourceBusy, "Group directory contains unregistered files"));
                }
            }
        }
        let staged = if path.exists() {
            Some(StagedDirectory::new(&self.templates_path(), &path)?)
        } else {
            None
        };
        self.group_manager.remove_group(name).await?;
        if let Some(staged) = staged {
            staged.commit();
        }
        self.template_manager.forget_group(name).await;
        Ok(())
    }

    pub async fn set_group_maintenance(&self, name: &str, enabled: bool) -> Result<(), Error> {
        let _operation = self.operation().await?;
        self.group_manager.set_maintenance(name, enabled).await
    }

    pub async fn create_template(
        &self,
        group: &str,
        name: &str,
        software: ServerSoftware,
        version: Option<String>,
        custom_jar_name: Option<String>,
    ) -> Result<(), Error> {
        let _operation = self.operation().await?;
        self.require_group(group).await?;
        self.template_manager
            .create_new_template(group, name, software, version, custom_jar_name)
            .await?;
        if let Err(error) = self.group_manager.add_template(group, name).await {
            let path = self.template_manager.checked_path(group, name)?;
            StagedDirectory::new(&self.templates_path(), &path)?.commit();
            self.template_manager.forget_template(group, name).await?;
            return Err(error);
        }
        Ok(())
    }

    pub async fn delete_template(&self, group: &str, name: &str) -> Result<(), Error> {
        let _operation = self.operation().await?;
        let group_config = self.require_group(group).await?;
        if name == "global" {
            return Err(Error::new(ErrorKind::ResourceBusy, "The global template is required"));
        }
        if !group_config.template_names().iter().any(|n| n == name) {
            return Err(Error::new(ErrorKind::NotFound, "Template not assigned to group"));
        }
        if self
            .instance_manager
            .instances_list()
            .await
            .iter()
            .any(|instance| instance.instance().group_name() == group && instance.instance().template_name() == Some(name))
        {
            return Err(Error::new(ErrorKind::ResourceBusy, "Template is used by an instance"));
        }
        let path = self.template_manager.checked_path(group, name)?;
        let staged = if path.exists() {
            Some(StagedDirectory::new(&self.templates_path(), &path)?)
        } else {
            None
        };
        self.group_manager.delete_template(group, name).await?;
        if let Some(staged) = staged {
            staged.commit();
        }
        self.template_manager.forget_template(group, name).await?;
        Ok(())
    }

    /// Create and prepare a stopped instance. Starting is a separate operation.
    pub async fn create_instance_from_template(&self, group: &str, name: &str) -> Result<Instance, Error> {
        let _operation = self.operation().await?;

        let group_config = self.require_group(group).await?;

        if group_config.maintenance() {
            return Err(Error::new(ErrorKind::ResourceBusy, "Group is in maintenance"));
        }

        if !group_config.template_names().iter().any(|n| n == name) {
            return Err(Error::new(ErrorKind::NotFound, "Template not assigned to group"));
        }

        let template = self
            .template_manager
            .get_template(group, name)
            .await
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Template not found"))?;

        let source = self.template_manager.checked_path(group, name)?;
        let jar_name = template.jar_name()?;

        let cached_jar = match template.server_software() {
            ServerSoftware::Custom => {
                if !source.join(&jar_name).is_file() {
                    return Err(Error::new(ErrorKind::NotFound, "Custom template JAR is missing"));
                }

                None
            }

            software => {
                let version = template.minecraft_version().as_deref();

                Some(self.server_software_manager.get_server_jar_path(software, version).await?)
            }
        };

        let instance = self.instance_manager.create_instance_from_template(group, template).await?;

        let prepare = (|| -> Result<(), Error> {
            fs::create_dir_all(self.running_path())?;

            let staged = tempfile::Builder::new().prefix(".preparing-").tempdir_in(self.running_path())?;

            copy_dir_all(&source, staged.path())?;

            if let Some(jar_path) = cached_jar {
                fs::copy(jar_path, staged.path().join(&jar_name))?;
            }

            let metadata = staged.path().join("templates.toml");

            if metadata.exists() {
                fs::remove_file(metadata)?;
            }

            fs::rename(staged.path(), self.running_path().join(instance.id()))?;

            Ok(())
        })();

        if let Err(error) = prepare {
            self.instance_manager.remove_instance(instance.id()).await?;

            return Err(error);
        }

        Ok(instance)
    }

    pub async fn start_instance(&self, id: &str) -> Result<(), Error> {
        let _operation = self.operation().await?;
        let instance = self
            .instance_manager
            .get_instance(id)
            .await
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Instance not found"))?;
        if self.require_group(instance.instance().group_name()).await?.maintenance() {
            return Err(Error::new(ErrorKind::ResourceBusy, "Group is in maintenance"));
        }
        self.instance_manager.start_instance(id).await
    }

    pub async fn stop_instance(&self, id: &str) -> Result<(), Error> {
        let _operation = self.operation().await?;
        self.instance_manager.stop_instance(id).await
    }

    pub async fn remove_instance(&self, id: &str) -> Result<(), Error> {
        let _operation = self.operation().await?;
        validate_name(id)?;
        self.instance_manager.stop_instance(id).await?;
        let path = self.running_path().join(id);
        let staged = if path.exists() {
            Some(StagedDirectory::new(&self.running_path(), &path)?)
        } else {
            None
        };
        self.instance_manager.remove_instance(id).await?;
        if let Some(staged) = staged {
            staged.commit();
        }
        Ok(())
    }

    pub async fn validate_groups(&self) -> Result<(), Error> {
        for group in self.group_manager.groups().await {
            for name in group.template_names() {
                if self.template_manager.get_template(group.name(), name).await.is_none() {
                    return Err(Error::new(
                        ErrorKind::NotFound,
                        format!("Template '{name}' in group '{}' not found", group.name()),
                    ));
                }
            }
        }
        Ok(())
    }

    pub async fn cleanup_running_directory(&self) -> Result<(), Error> {
        if !self.running_path().exists() {
            tokio::fs::create_dir_all(&self.running_path()).await?;
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(&self.running_path()).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = tokio::fs::symlink_metadata(&path).await?;

            if metadata.is_dir() {
                tokio::fs::remove_dir_all(&path).await?;
            } else {
                tokio::fs::remove_file(&path).await?;
            }
        }

        Ok(())
    }

    pub async fn initialize(&self) -> Result<(), Error> {
        let mut operation = self.operations.lock().await;
        if operation.shutting_down {
            return Err(Error::new(ErrorKind::ResourceBusy, "Cloud is shutting down"));
        }
        if operation.initialized {
            return Ok(());
        }

        self.cleanup_running_directory().await?;
        for path in [
            self.config_path(),
            self.templates_path(),
            self.static_servers_path(),
            self.running_path(),
            self.cache_path(),
        ] {
            fs::create_dir_all(path)?;
        }

        fs::create_dir_all(self.cache_path().join("versions"))?;
        let version = self.config_manager.config().fallback_minecraft_version();
        self.server_software_manager.ensure_paper_available(version).await?;
        self.server_software_manager.ensure_mojang_mapping(version).await?;
        self.server_software_manager.ensure_velocity_available().await?;

        self.group_manager.load_groups_from_config().await?;
        for group in self.group_manager.groups().await {
            let expected = self.templates_path().join(group.name());
            if group.path() != &expected {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("Group '{}' path must be {}", group.name(), expected.display()),
                ));
            }
            self.ensure_global(group.name()).await?;
            self.group_manager.add_template(group.name(), "global").await?;
            let group = self.require_group(group.name()).await?;
            self.template_manager
                .load_group_templates(group.name(), group.template_names(), version)
                .await?;
        }
        self.validate_groups().await?;
        operation.initialized = true;
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), Error> {
        let mut operation = self.operations.lock().await;
        operation.shutting_down = true;
        self.instance_manager.shutdown().await?;
        // self.cleanup_running_directory().await?; TODO:  fix race condition
        Ok(())
    }
}
