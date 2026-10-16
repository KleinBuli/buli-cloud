use crate::{
    config::config_manager::ConfigManager,
    groups::{
        group::{Group, GroupRole},
        group_manager::GroupManager,
    },
    instances::{
        instance::{Instance, InstanceMode},
        instance_manager::InstanceManager,
        static_config::StaticInstanceConfig,
    },
    templates::{
        template::Template,
        template_manager::{GROUP_CONFIG, TemplateManager},
    },
    util::{
        file_utils::{StagedDirectory, copy_dir_all, validate_name},
        secret_generator,
        software::softwaremanager::{ServerSoftware, ServerSoftwareManager},
    },
};
use std::{
    io::{Error, ErrorKind},
    path::PathBuf,
    sync::Arc,
};
use tokio::{
    fs,
    sync::{RwLock, RwLockWriteGuard, watch},
};

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
    operations: RwLock<Lifecycle>,
    _data_lock: std::fs::File,
    shutdown_requested: watch::Sender<bool>,
}

impl CloudCore {
    pub async fn new<P: Into<PathBuf>>(root_path: P) -> Result<Self, Error> {
        let root_path = root_path.into();
        fs::create_dir_all(&root_path).await?;
        let data_lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root_path.join(".lock"))?;
        data_lock
            .try_lock()
            .map_err(|error| Error::new(ErrorKind::ResourceBusy, format!("Data directory is already in use: {error}")))?;
        for directory in ["static", "running"] {
            let directory = root_path.join(directory);
            if !directory.exists() {
                continue;
            }
            for entry in std::fs::read_dir(directory)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() && entry.path().join(".process").exists() {
                    return Err(Error::new(
                        ErrorKind::ResourceBusy,
                        format!(
                            "Unconfirmed process ownership in {}; verify the old process has stopped before removing .process",
                            entry.path().display()
                        ),
                    ));
                }
            }
        }
        for directory in ["templates", "static", "running"] {
            crate::util::file_utils::recover_staged_directories(&root_path.join(directory))?;
        }
        let config_manager = ConfigManager::new(root_path.join("config/config.toml")).await?;
        Ok(Self {
            group_manager: Arc::new(GroupManager::new(root_path.join("config/groups.toml"))?),
            template_manager: Arc::new(TemplateManager::new(root_path.join("templates"))),
            instance_manager: Arc::new(InstanceManager::new(
                root_path.join("running"),
                root_path.join("static"),
                root_path.join("config/forwarding.secret"),
            )),
            config_manager,
            server_software_manager: ServerSoftwareManager::new(root_path.join("cache/versions")),
            root_path,
            operations: RwLock::new(Lifecycle::default()),
            _data_lock: data_lock,
            shutdown_requested: watch::channel(false).0,
        })
    }

    pub async fn spawn_instance_event_listener(&self) {
        let instance_manager = Arc::clone(&self.instance_manager);
        tokio::spawn(async move {
            instance_manager.handle_runtime_events().await;
        });
    }

    async fn operation(&self) -> Result<RwLockWriteGuard<'_, Lifecycle>, Error> {
        let guard = self.operations.write().await;
        if guard.shutting_down || *self.shutdown_requested.borrow() {
            return Err(Error::new(ErrorKind::ResourceBusy, "Cloud is shutting down"));
        }
        if !guard.initialized {
            return Err(Error::new(ErrorKind::ResourceBusy, "Initialize the cloud before changing its state"));
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
        self.root_path.join("static")
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
        fs::create_dir_all(self.templates_path().join(group).join("global")).await
    }

    pub async fn create_group(&self, name: &str, group_role: GroupRole) -> Result<(), Error> {
        let _operation = self.operation().await?;
        validate_name(name)?;
        if self.group_manager.groups().await.iter().any(|group| group.name().eq_ignore_ascii_case(name)) {
            return Err(Error::new(ErrorKind::AlreadyExists, "Group already exists"));
        }
        let global_existed = self.template_manager.exists_on_disk(name, "global");
        self.ensure_global(name).await?;
        self.template_manager.load_group_templates(name, &[]).await?;
        let group = Group::new(name.to_string(), vec![], self.templates_path().join(name), group_role);
        if let Err(error) = self.group_manager.add_group(group).await {
            if !global_existed {
                StagedDirectory::new(&self.templates_path(), &self.templates_path().join(name).join("global"))?.commit()?;
            }
            self.template_manager.forget_group(name).await;
            return Err(error);
        }
        Ok(())
    }

    pub async fn remove_group(&self, name: &str) -> Result<(), Error> {
        let _operation = self.operation().await?;
        let group = self.require_group(name).await?;
        if !group.template_names().is_empty()
            || self
                .instance_manager
                .instances_list()
                .await
                .iter()
                .any(|instance| instance.instance().group_name() == name)
        {
            return Err(Error::new(ErrorKind::ResourceBusy, "Remove the group's instances and custom templates first"));
        }
        let path = self.templates_path().join(name);
        if path.exists() {
            let mut entries = fs::read_dir(&path).await?;
            while let Some(entry) = entries.next_entry().await? {
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
        if let Some(staged) = &staged {
            staged.backup_metadata(&[self.config_path().join("groups.toml")])?;
        }
        self.group_manager.remove_group(name).await?;
        if let Some(staged) = staged {
            staged.commit()?;
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
        let result = self.require_group(group).await?;
        let role = result.role();
        let role_name = role.to_string();

        if !role.allows_software(&software) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("Group with role {role_name} can't use ServerSoftware: {software}"),
            ));
        }

        self.template_manager
            .create_new_template(group, name, software, version, custom_jar_name)
            .await?;
        if let Err(error) = self.group_manager.add_template(group, name).await {
            let path = self.template_manager.checked_path(group, name)?;
            StagedDirectory::new(&self.templates_path(), &path)?.commit()?;
            self.template_manager.forget_template(group, name).await?;
            return Err(error);
        }
        Ok(())
    }

    pub async fn delete_template(&self, group: &str, name: &str) -> Result<(), Error> {
        let _operation = self.operation().await?;
        let group_config = self.require_group(group).await?;
        if name.eq_ignore_ascii_case("global") {
            return Err(Error::new(ErrorKind::ResourceBusy, "Global is a reserved directory"));
        }
        if !group_config.template_names().iter().any(|n| n == name) {
            return Err(Error::new(ErrorKind::NotFound, "Template not assigned to group"));
        }
        if self
            .instance_manager
            .instances_list()
            .await
            .iter()
            .any(|instance| instance.instance().group_name() == group && instance.instance().template_name() == name)
        {
            return Err(Error::new(ErrorKind::ResourceBusy, "Template is used by an instance"));
        }
        let path = self.template_manager.checked_path(group, name)?;
        let staged = if path.exists() {
            Some(StagedDirectory::new(&self.templates_path(), &path)?)
        } else {
            None
        };
        if let Some(staged) = &staged {
            staged.backup_metadata(&[self.config_path().join("groups.toml"), self.templates_path().join(group).join(GROUP_CONFIG)])?;
        }
        self.group_manager.delete_template(group, name).await?;
        if let Err(error) = self.template_manager.forget_template(group, name).await {
            self.group_manager
                .add_template(group, name)
                .await
                .map_err(|rollback| Error::other(format!("Could not remove template metadata: {error}; restoring group failed: {rollback}")))?;
            return Err(error);
        }
        if let Some(staged) = staged {
            staged.commit()?;
        }
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

        let instance = self.instance_manager.create_instance_from_template(group, &template).await?;
        let base_path = match instance.instance_mode() {
            InstanceMode::Dynamic => self.running_path(),
            InstanceMode::Static => self.static_servers_path(),
        };

        let global_source = source
            .parent()
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Template has no parent directory"))?
            .join("global");

        let prepare = (async || -> Result<(), Error> {
            fs::create_dir_all(&base_path).await?;

            let staged = tempfile::Builder::new().prefix(".preparing-").tempdir_in(&base_path)?;

            if !global_source.is_dir() {
                return Err(Error::new(ErrorKind::NotFound, "Global template directory missing"));
            }

            copy_dir_all(&global_source, staged.path())?;
            copy_dir_all(&source, staged.path())?;

            if let Some(jar_path) = cached_jar {
                fs::copy(jar_path, staged.path().join(&jar_name)).await?;
            }

            let metadata = staged.path().join("templates.toml");

            if metadata.exists() {
                fs::remove_file(metadata).await?;
            }

            if template.is_static_instance() {
                let config = StaticInstanceConfig::new(group.to_string(), template.name().to_string());

                config.save(&staged.path().join("instance.toml")).await?;
            }

            crate::util::file_utils::atomic_write(&staged.path().join(".destination"), instance.id().as_bytes())?;
            fs::rename(staged.path(), &base_path.join(instance.id())).await?;
            fs::remove_file(base_path.join(instance.id()).join(".destination")).await?;

            Ok(())
        })();

        if let Err(error) = prepare.await {
            self.instance_manager.remove_instance(instance.id()).await?;

            return Err(error);
        }

        Ok(instance)
    }

    pub async fn start_instance(&self, id: &str) -> Result<(), Error> {
        let operation = self.operations.read().await;
        if !operation.initialized || operation.shutting_down || *self.shutdown_requested.borrow() {
            return Err(Error::new(ErrorKind::ResourceBusy, "Cloud is unavailable"));
        }
        let instance = self
            .instance_manager
            .get_instance(id)
            .await
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Instance not found"))?;

        let group = self.require_group(instance.instance().group_name()).await?;
        let group_role = group.role();

        self.instance_manager.start_instance(id, group_role).await
    }

    pub async fn stop_instance(&self, id: &str) -> Result<(), Error> {
        let operation = self.operations.read().await;
        if !operation.initialized || operation.shutting_down || *self.shutdown_requested.borrow() {
            return Err(Error::new(ErrorKind::ResourceBusy, "Cloud is unavailable"));
        }
        self.instance_manager.stop_instance(id).await
    }

    pub async fn remove_instance(&self, id: &str) -> Result<(), Error> {
        let _operation = self.operation().await?;
        validate_name(id)?;
        self.instance_manager.remove_instance_with_files(id).await
    }

    pub async fn validate_groups(&self) -> Result<(), Error> {
        for group in self.group_manager.groups().await {
            for name in group.template_names() {
                if name == "global" {
                    continue;
                }

                let template = self
                    .template_manager
                    .get_template(group.name(), name)
                    .await
                    .ok_or_else(|| Error::new(ErrorKind::NotFound, format!("Template '{name}' not found")))?;
                Self::validate_group_template(&group, &template)?;
            }
        }
        Ok(())
    }

    pub async fn ensure_bridge_secret(&self) -> Result<(), Error> {
        let path = self.config_path().join("bridge.secret");

        match tokio::fs::symlink_metadata(&path).await {
            Ok(metadata) => {
                if !metadata.file_type().is_file() {
                    return Err(Error::new(ErrorKind::InvalidData, "bridge secret path is not a regular file"));
                }
            }

            Err(error) if error.kind() == ErrorKind::NotFound => {
                secret_generator::create_secret_file(&path).await?;
            }

            Err(error) => return Err(error),
        }

        let secret = tokio::fs::read_to_string(&path).await?;

        if secret.len() != 64 || !secret.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error::new(ErrorKind::InvalidData, "bridge secret is invalid (expected 64 hex characters)"));
        }

        Ok(())
    }

    pub async fn ensure_forwarding_secret(&self) -> Result<(), Error> {
        let path = self.config_path().join("forwarding.secret");

        match tokio::fs::symlink_metadata(&path).await {
            Ok(metadata) => {
                if !metadata.file_type().is_file() {
                    return Err(Error::new(ErrorKind::InvalidData, "Forwarding secret path is not a regular file"));
                }
            }

            Err(error) if error.kind() == ErrorKind::NotFound => {
                secret_generator::create_secret_file(&path).await?;
            }

            Err(error) => return Err(error),
        }

        let secret = tokio::fs::read_to_string(&path).await?;

        if secret.len() != 64 || !secret.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error::new(ErrorKind::InvalidData, "Forwarding secret is invalid (expected 64 hex characters)"));
        }

        Ok(())
    }

    pub(crate) async fn restore_static_instances(&self) -> Result<(), Error> {
        let instances = self.instance_manager().discover_static_instances().await?;

        for id in &instances {
            let config = StaticInstanceConfig::load(&&self.static_servers_path().join(id).join("instance.toml")).await?;
            let group = config.group();
            let template_name = config.template();
            let template = self
                .template_manager()
                .get_template(group, template_name)
                .await
                .ok_or_else(|| Error::new(ErrorKind::NotFound, format!("Didn't find template {template_name} for group {group}")))?;

            let instance = Instance::new(id, group, template_name.to_string(), InstanceMode::Static);
            self.instance_manager().register_instance(instance, template).await?
        }

        Ok(())
    }

    pub async fn initialize(&self) -> Result<(), Error> {
        let mut operation = self.operations.write().await;
        if operation.shutting_down {
            return Err(Error::new(ErrorKind::ResourceBusy, "Cloud is shutting down"));
        }
        if operation.initialized {
            return Ok(());
        }

        for path in [
            self.config_path(),
            self.templates_path(),
            self.static_servers_path(),
            self.running_path(),
            self.cache_path(),
        ] {
            fs::create_dir_all(path).await?;
        }

        fs::create_dir_all(self.cache_path().join("versions")).await?;
        self.ensure_bridge_secret().await?;
        self.ensure_forwarding_secret().await?;

        let config = self.config_manager.config();
        let version = config.fallback_minecraft_version();
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
            let group = self.require_group(group.name()).await?;
            self.template_manager.load_group_templates(group.name(), group.template_names()).await?;
        }
        self.validate_groups().await?;
        self.restore_static_instances().await?;

        operation.initialized = true;
        Ok(())
    }

    pub async fn reload(&self) -> Result<(), Error> {
        let _operation = self.operation().await?;

        let config = self.config_manager.read_config().await?;
        let groups = self.group_manager.read_groups_from_config()?;
        let templates = self.template_manager.prepare_reload(&groups)?;

        for group in groups.values() {
            for name in group.template_names() {
                let key = (group.name().to_string(), name.to_string());

                let template = templates
                    .get(&key)
                    .ok_or_else(|| Error::new(ErrorKind::NotFound, format!("Template '{name}' in group '{}' not found", group.name())))?;

                Self::validate_group_template(group, template)?;
            }
        }
        self.group_manager.replace_groups(groups).await;
        self.template_manager.replace_templates(templates).await;
        self.config_manager.replace_config(config);

        Ok(())
    }

    fn validate_group_template(group: &Group, template: &Template) -> Result<(), Error> {
        let software = template.server_software();

        if !group.role().allows_software(software) {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("Group '{}' cannot use software {software}", group.name()),
            ));
        }

        Ok(())
    }

    pub fn request_shutdown(&self) {
        self.shutdown_requested.send_replace(true);
    }

    pub async fn wait_for_shutdown(&self) {
        let mut receiver = self.shutdown_requested.subscribe();
        let _ = receiver.wait_for(|requested| *requested).await;
    }

    pub async fn shutdown(&self) -> Result<(), Error> {
        self.request_shutdown();
        if let Ok(mut operation) = self.operations.try_write() {
            operation.shutting_down = true;
        }
        self.instance_manager.shutdown().await?;
        Ok(())
    }
}
