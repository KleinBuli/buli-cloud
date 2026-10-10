use tokio::fs;

use super::InstanceManager;
use crate::{
    instances::{
        instance::{Instance, InstanceMode},
        instance_runtime::{InstanceRuntime, InstanceStatus},
    },
    templates::template::Template,
    util::file_utils::{StagedDirectory, validate_name},
};
use std::io::{Error, ErrorKind};

impl InstanceManager {
    pub async fn cleanup_instance(&self, id: &str) -> Result<(), Error> {
        validate_name(id)?;

        let state = self.state.lock().await;
        let runtime = state.get(id).ok_or_else(|| Error::new(ErrorKind::NotFound, "Instance not found"))?;
        if runtime.status() != InstanceStatus::Stopped {
            return Err(Error::new(ErrorKind::ResourceBusy, "Instance has not stopped"));
        }
        self.cleanup_instance_files(runtime.instance()).await
    }

    pub(super) async fn cleanup_instance_files(&self, instance: &Instance) -> Result<(), Error> {
        if instance.instance_mode() == &InstanceMode::Static {
            return Ok(());
        }

        let instance_path = self.working_directory(instance);

        if !fs::try_exists(&instance_path).await? {
            return Ok(());
        }

        let metadata = fs::symlink_metadata(&instance_path).await?;

        if metadata.file_type().is_symlink() {
            return Err(Error::new(ErrorKind::InvalidInput, "Instance directory must not be a symbolic link"));
        }

        if !metadata.is_dir() {
            return Err(Error::new(ErrorKind::InvalidInput, "Instance path is not a directory"));
        }

        let canonical_running = fs::canonicalize(&self.running_path).await?;
        let canonical_instance = fs::canonicalize(&instance_path).await?;

        if !canonical_instance.starts_with(&canonical_running) || canonical_instance == canonical_running {
            return Err(Error::new(ErrorKind::PermissionDenied, "Refusing to delete directory outside running path"));
        }

        fs::remove_dir_all(&canonical_instance).await?;

        Ok(())
    }

    pub(crate) async fn create_instance_from_template(&self, group: &str, template: &Template) -> Result<Instance, Error> {
        validate_name(group)?;
        validate_name(template.name())?;
        template.jar_name()?;

        let mut state = self.state.lock().await;
        let instance_mode = if template.is_static_instance() {
            InstanceMode::Static
        } else {
            InstanceMode::Dynamic
        };
        for number in 1u64.. {
            let id = format!("{group}-{number}");

            if !state.contains_key(&id) && !self.running_path.join(&id).try_exists()? && !self.static_path.join(&id).try_exists()? {
                let instance = Instance::new(&id, group, template.name().to_string(), instance_mode);
                state.insert(id, InstanceRuntime::new(instance.clone(), template.clone(), self.event_tx.clone()));
                return Ok(instance);
            }
        }
        Err(Error::other("No instance ID available"))
    }

    pub async fn register_instance(&self, instance: Instance, template: Template) -> Result<(), Error> {
        let mut state = self.state.lock().await;

        if state.contains_key(instance.id()) {
            return Err(Error::new(ErrorKind::AlreadyExists, "Instance already registered"));
        }

        state.insert(
            instance.id().to_string(),
            InstanceRuntime::new(instance.clone(), template, self.event_tx.clone()),
        );

        Ok(())
    }

    pub async fn static_id_exists(&self, id: &str) -> Result<bool, Error> {
        validate_name(id)?;
        tokio::fs::try_exists(self.static_path.join(id)).await
    }

    pub(crate) async fn discover_static_instances(&self) -> Result<Vec<String>, Error> {
        let path = &self.static_path;
        let mut instances = Vec::new();

        if !tokio::fs::try_exists(path).await? {
            return Ok(instances);
        }

        let mut entries = tokio::fs::read_dir(path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let file_type = entry.file_type().await?;

            if !file_type.is_dir() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_string();

            if validate_name(&name).is_err() {
                continue;
            }

            instances.push(name);
        }

        instances.sort();

        Ok(instances)
    }

    pub(crate) async fn remove_instance_with_files(&self, id: &str) -> Result<(), Error> {
        validate_name(id)?;
        let mut state = self.state.lock().await;
        let runtime = state.get_mut(id).ok_or_else(|| Error::new(ErrorKind::NotFound, "Instance not found"))?;
        runtime.stop().await?;
        let path = self.working_directory(runtime.instance());
        let root = match runtime.instance().instance_mode() {
            InstanceMode::Dynamic => &self.running_path,
            InstanceMode::Static => &self.static_path,
        };
        let staged = if path.try_exists()? { Some(StagedDirectory::new(root, &path)?) } else { None };
        if let Some(port) = self.port_allocator.get_corresponding_port(id).await {
            self.port_allocator.deallocate(port).await?;
        }
        state.remove(id);
        if let Some(staged) = staged {
            staged.commit();
        }
        Ok(())
    }

    pub(crate) async fn remove_instance(&self, id: &str) -> Result<Option<Instance>, Error> {
        let mut state = self.state.lock().await;
        if state.get(id).is_some_and(|entry| entry.status() != InstanceStatus::Stopped) {
            return Err(Error::new(ErrorKind::ResourceBusy, "Instance is still running"));
        }
        Ok(state.remove(id).map(|entry| entry.instance().clone()))
    }
}
