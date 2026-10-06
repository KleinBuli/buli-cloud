use crate::{
    instances::{
        instance::Instance,
        instance_runtime::{InstanceInfo, InstanceRuntime, InstanceStatus},
        port_allocator::PortAllocator,
    },
    logger::logger::{LogLevel, log},
    templates::template::Template,
    util::file_utils::{set_server_property, validate_name},
};
use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::{Mutex, broadcast::error};

/// Dynamic instances belong to this daemon's lifetime. Existing directories are never reused implicitly.
pub struct InstanceManager {
    running_path: PathBuf,
    state: Arc<Mutex<HashMap<String, InstanceRuntime>>>,
    port_allocator: PortAllocator,
}

impl InstanceManager {
    pub fn new(running_path: PathBuf) -> Self {
        Self {
            running_path,
            state: Arc::new(Mutex::new(HashMap::new())),
            port_allocator: PortAllocator::new(),
        }
    }

    pub async fn exists(&self, id: &str) -> bool {
        self.state.lock().await.contains_key(id)
    }

    pub async fn get_instance(&self, id: &str) -> Option<InstanceInfo> {
        let state = self.state.lock().await;
        state.get(id).map(|runtime| runtime.info())
    }

    pub async fn instances_list(&self) -> Vec<InstanceInfo> {
        let state = self.state.lock().await;
        let mut result: Vec<_> = state.values().map(|entry| entry.info()).collect();
        result.sort_by(|a, b| a.instance().id().cmp(b.instance().id()));
        result
    }

    pub(crate) async fn create_instance_from_template(&self, group: &str, template: Template) -> Result<Instance, Error> {
        validate_name(group)?;
        validate_name(template.name())?;
        template.jar_name()?;
        let mut state = self.state.lock().await;
        for number in 1u64.. {
            let id = format!("{group}-{number}");
            if !state.contains_key(&id) && !self.running_path.join(&id).try_exists()? {
                let instance = Instance::new(&id, group, Some(template.name().to_string()));
                state.insert(id, InstanceRuntime::new(instance.clone(), template));
                return Ok(instance);
            }
        }
        Err(Error::other("No instance ID available"))
    }

    pub(crate) async fn remove_instance(&self, id: &str) -> Result<Option<Instance>, Error> {
        let mut state = self.state.lock().await;
        if state.get(id).is_some_and(|entry| entry.status() == InstanceStatus::Running) {
            return Err(Error::new(ErrorKind::ResourceBusy, "Instance is still running"));
        }
        Ok(state.remove(id).map(|entry| entry.instance().clone()))
    }

    pub async fn start_instance(&self, id: &str) -> Result<(), Error> {
        let mut state = self.state.lock().await;

        let runtime = state
            .get_mut(id)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Instance not found"))?;

        if runtime.status() != InstanceStatus::Stopped {
            return Ok(());
        }

        let port = self.set_port(id).await?;

        if let Err(error) = runtime.start(&self.running_path) {
            if let Err(cleanup_error) = self.port_allocator().deallocate(port).await {
                log(LogLevel::Error, &format!("Failed to deallocate port {port}: {cleanup_error}"));
            }

            return Err(error);
        }

        Ok(())
    }

    async fn set_port(&self, id: &str) -> Result<u16, Error> {
        let port = self.port_allocator.allocate_next(id).await?;
        let properties_path = self.running_path.join(id).join("server.properties");

        if let Err(error) = set_server_property(&properties_path, "server-port", &port.to_string()) {
            if let Err(cleanup_error) = self.port_allocator().deallocate(port).await {
                log(LogLevel::Error, &format!("Failed to deallocate port {port}: {cleanup_error}"));
            }

            return Err(error);
        }

        Ok(port)
    }

    pub async fn stop_instance(&self, id: &str) -> Result<(), Error> {
        let mut state = self.state.lock().await;
        let runtime = state
            .get_mut(id)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Instance not found"))?;

        let port = self.port_allocator().get_corresponding_port(runtime.info().instance().id()).await;
        runtime.stop().await?;

        if let Some(port) = port {
            self.port_allocator().deallocate(port).await?;
        }

        Ok(())
    }

    pub(crate) async fn shutdown(&self) -> Result<(), Error> {
        let mut state = self.state.lock().await;
        let mut failure = None;

        for runtime in state.values_mut() {
            if let Err(error) = runtime.stop().await {
                failure = Some(error);
            }
        }

        failure.map_or(Ok(()), Err)
    }

    pub fn port_allocator(&self) -> &PortAllocator {
        &self.port_allocator
    }
}
