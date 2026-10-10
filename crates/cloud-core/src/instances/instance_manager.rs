mod console;
mod lifecycle;
mod management;

use crate::instances::{
    instance::{Instance, InstanceMode},
    instance_runtime::{InstanceInfo, InstanceRuntime, RuntimeEvent},
    port_allocator::PortAllocator,
};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, mpsc};

/// Dynamic instances belong to this daemon's lifetime. Existing directories are never reused implicitly.
pub struct InstanceManager {
    running_path: PathBuf,
    static_path: PathBuf,
    state: Arc<Mutex<HashMap<String, InstanceRuntime>>>,
    port_allocator: PortAllocator,
    event_tx: mpsc::UnboundedSender<RuntimeEvent>,
    event_rx: Mutex<mpsc::UnboundedReceiver<RuntimeEvent>>,
}

impl InstanceManager {
    pub fn new(running_path: PathBuf, static_path: PathBuf) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel::<RuntimeEvent>();
        Self {
            running_path,
            static_path,
            state: Arc::new(Mutex::new(HashMap::new())),
            port_allocator: PortAllocator::new(),
            event_tx,
            event_rx: Mutex::new(event_rx),
        }
    }

    pub fn working_directory(&self, instance: &Instance) -> PathBuf {
        let base_path = match instance.instance_mode() {
            InstanceMode::Dynamic => &self.running_path,
            InstanceMode::Static => &self.static_path,
        };

        base_path.join(instance.id())
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

    pub fn port_allocator(&self) -> &PortAllocator {
        &self.port_allocator
    }
}
