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
    shutting_down: std::sync::atomic::AtomicBool,
    running_path: PathBuf,
    static_path: PathBuf,
    state: Arc<Mutex<HashMap<String, Arc<Mutex<InstanceRuntime>>>>>,
    port_allocator: PortAllocator,
    event_tx: mpsc::UnboundedSender<RuntimeEvent>,
    event_rx: Mutex<mpsc::UnboundedReceiver<RuntimeEvent>>,
}

impl InstanceManager {
    pub fn new(running_path: PathBuf, static_path: PathBuf) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel::<RuntimeEvent>();
        Self {
            shutting_down: std::sync::atomic::AtomicBool::new(false),
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
        let runtime = self.runtime(id).await.ok()?;
        Some(runtime.lock().await.info())
    }

    pub async fn instances_list(&self) -> Vec<InstanceInfo> {
        let runtimes: Vec<_> = self.state.lock().await.values().cloned().collect();
        let mut result = Vec::new();
        for runtime in runtimes {
            result.push(runtime.lock().await.info());
        }
        result.sort_by(|a, b| a.instance().id().cmp(b.instance().id()));
        result
    }

    async fn runtime(&self, id: &str) -> Result<Arc<Mutex<InstanceRuntime>>, std::io::Error> {
        self.state
            .lock()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Instance not found"))
    }

    pub fn port_allocator(&self) -> &PortAllocator {
        &self.port_allocator
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{templates::template::Template, util::software::softwaremanager::ServerSoftware};

    #[tokio::test]
    async fn busy_instance_does_not_lock_other_instances() {
        let root = tempfile::tempdir().unwrap();
        let manager = InstanceManager::new(root.path().join("running"), root.path().join("static"));
        let template = Template::new("test", ServerSoftware::Paper, Some("1.21".to_string()), None);
        for id in ["one", "two"] {
            manager
                .register_instance(Instance::new(id, "test", "test".to_string(), InstanceMode::Static), template.clone())
                .await
                .unwrap();
        }
        let first = manager.runtime("one").await.unwrap();
        let _busy = first.lock().await;
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(250), manager.get_instance("two"))
                .await
                .unwrap()
                .is_some()
        );
    }
}
