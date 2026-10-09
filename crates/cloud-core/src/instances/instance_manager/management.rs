use super::InstanceManager;
use crate::{
    instances::{
        instance::Instance,
        instance_runtime::{InstanceMode, InstanceRuntime, InstanceStatus},
    },
    templates::template::Template,
    util::file_utils::validate_name,
};
use std::io::{Error, ErrorKind};
use tokio::fs;

impl InstanceManager {
    pub async fn cleanup_instance(&self, id: &str) -> Result<(), Error> {
        validate_name(id)?;

        let running_path = &self.running_path;
        let instance_path = running_path.join(id);

        if !instance_path.exists() {
            return Ok(());
        }

        let metadata = fs::symlink_metadata(&instance_path).await?;

        if metadata.file_type().is_symlink() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Instance directory must not be a symbolic link",
            ));
        }

        if !metadata.is_dir() {
            return Err(Error::new(ErrorKind::InvalidInput, "Instance path is not a directory"));
        }

        let canonical_running = fs::canonicalize(running_path).await?;
        let canonical_instance = fs::canonicalize(&instance_path).await?;

        if !canonical_instance.starts_with(&canonical_running) || canonical_instance == canonical_running {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                "Refusing to delete directory outside running path",
            ));
        }

        fs::remove_dir_all(&canonical_instance).await?;

        Ok(())
    }

    pub(crate) async fn create_instance_from_template(&self, group: &str, template: Template) -> Result<Instance, Error> {
        validate_name(group)?;
        validate_name(template.name())?;
        template.jar_name()?;
        let mut state = self.state.lock().await;
        for number in 1u64.. {
            let id = format!("{group}-{number}");
            if !state.contains_key(&id) && !self.running_path.join(&id).try_exists()? {
                let instance = Instance::new(&id, group, Some(template.name().to_string()), InstanceMode::Dynamic);
                state.insert(id, InstanceRuntime::new(instance.clone(), template, self.event_tx.clone()));
                return Ok(instance);
            }
        }
        Err(Error::other("No instance ID available"))
    }

    pub(crate) async fn create_instance_static(&self, group: &str) -> Result<(), Error> {
        validate_name(group)?;

        Ok(())
    }

    pub(crate) async fn remove_instance(&self, id: &str) -> Result<Option<Instance>, Error> {
        let mut state = self.state.lock().await;
        if state.get(id).is_some_and(|entry| entry.status() == InstanceStatus::Running) {
            return Err(Error::new(ErrorKind::ResourceBusy, "Instance is still running"));
        }
        Ok(state.remove(id).map(|entry| entry.instance().clone()))
    }
}
