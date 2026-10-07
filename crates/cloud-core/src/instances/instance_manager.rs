use crate::{
    instances::{
        instance::Instance,
        instance_runtime::{InstanceInfo, InstanceRuntime, InstanceStatus, RuntimeEvent},
        port_allocator::PortAllocator,
    },
    logger::logger::{LogLevel, log},
    templates::template::Template,
    util::file_utils::{set_server_property, validate_name},
};
use std::{
    collections::HashMap,
    fs::File,
    io::{Error, ErrorKind, Write},
    path::PathBuf,
    sync::Arc,
};
use tokio::{
    fs,
    sync::{Mutex, mpsc},
};

/// Dynamic instances belong to this daemon's lifetime. Existing directories are never reused implicitly.
pub struct InstanceManager {
    running_path: PathBuf,
    state: Arc<Mutex<HashMap<String, InstanceRuntime>>>,
    port_allocator: PortAllocator,
    event_tx: mpsc::Sender<RuntimeEvent>,
    event_rx: Mutex<mpsc::Receiver<RuntimeEvent>>,
}

impl InstanceManager {
    pub fn new(running_path: PathBuf) -> Self {
        let (event_tx, mut event_rx) = mpsc::channel::<RuntimeEvent>(32);
        Self {
            running_path,
            state: Arc::new(Mutex::new(HashMap::new())),
            port_allocator: PortAllocator::new(),
            event_tx,
            event_rx: Mutex::new(event_rx),
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
                let instance = Instance::new(&id, group, Some(template.name().to_string()));
                state.insert(id, InstanceRuntime::new(instance.clone(), template, self.event_tx.clone()));
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

        if !properties_path.exists() {
            let mut file = File::create(&properties_path)?;
            writeln!(file, "server-port={port}")?;
        }

        if let Err(error) = set_server_property(&properties_path, "server-port", &port.to_string()) {
            if let Err(cleanup_error) = self.port_allocator().deallocate(port).await {
                log(LogLevel::Error, &format!("Failed to deallocate port {port}: {cleanup_error}"));
            }

            return Err(error);
        }

        Ok(port)
    }

    pub async fn handle_runtime_events(&self) {
        loop {
            let event = {
                let mut rx = self.event_rx.lock().await;
                rx.recv().await
            };

            let Some(event) = event else {
                break;
            };

            match event {
                RuntimeEvent::Exited(id) => {
                    if let Err(error) = self.handle_instance_exit(&id).await {
                        log(LogLevel::Error, &format!("Failed to cleanup instance {id}: {error}"));
                    }
                }
            }
        }
    }

    async fn handle_instance_exit(&self, id: &str) -> Result<(), Error> {
        if let Some(port) = self.port_allocator.get_corresponding_port(id).await {
            self.port_allocator.deallocate(port).await?;
        }

        self.cleanup_instance(id).await?;

        let mut state = self.state.lock().await;
        state.remove(id);

        Ok(())
    }

    // TODO: check the supervisor shutdown acknowledgment before deallocating port - maybe race condition
    pub async fn stop_instance(&self, id: &str) -> Result<(), Error> {
        let mut state = self.state.lock().await;

        let runtime = state
            .get_mut(id)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Instance not found"))?;

        runtime.stop().await?;

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
