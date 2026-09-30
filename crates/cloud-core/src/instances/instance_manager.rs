use std::collections::HashMap;

use tokio::sync::RwLock;

use crate::{
    instances::instance::{Instance, InstanceStatus},
    logger::logger::{LogLevel, log},
    templates::template::Template,
};

pub struct InstanceManager {
    instances: RwLock<HashMap<String, Instance>>,
}

impl InstanceManager {
    pub fn new() -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
        }
    }

    pub async fn add_instance(&self, instance: Instance) -> bool {
        let mut instances = self.instances.write().await;

        if instances.contains_key(instance.id()) {
            return false;
        }

        instances.insert(instance.id().to_string(), instance);
        true
    }

    pub async fn exists(&self, id: &str) -> bool {
        let instances = self.instances.read().await;
        instances.contains_key(id)
    }

    pub async fn get_instance(&self, id: &str) -> Option<Instance> {
        let instances = self.instances.read().await;
        instances.get(id).cloned()
    }

    pub async fn remove_instance(&self, id: &str) -> Option<Instance> {
        let mut instances = self.instances.write().await;
        instances.remove(id)
    }

    pub async fn instances_list(&self) -> Vec<Instance> {
        let instances = self.instances.read().await;
        instances.values().cloned().collect()
    }

    pub async fn create_instance_from_template(&self, template: Template) -> Instance {
        let mut instances = self.instances.write().await;
        let mut number = 1;

        loop {
            let id = format!("{}-{}", template.name(), number);

            if !instances.contains_key(&id) {
                let new_instance = Instance::new(&id, Some(template.name().to_string()));

                instances.insert(id, new_instance.clone());

                return new_instance;
            }

            number += 1;
        }
    }

    /// TODO
    pub async fn create_instance_as_static(&self) {}

    pub async fn start_instance(&self, id: &str) -> Result<(), ()> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or(())?;
        instance.set_status(InstanceStatus::Starting);
        log(LogLevel::Info, &format!("Starting Instance: {} ...", instance.id()));

        // TODO: Paper Server wirklich starten ....

        instance.set_status(InstanceStatus::Running);
        log(LogLevel::Info, &format!("Instance: {} started successfully", instance.id()));
        Ok(())
    }

    pub async fn stop_instance(&self, id: &str) -> Result<(), ()> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or(())?;

        // TODO: Paper Server stoppen.

        instance.set_status(InstanceStatus::Stopped);
        log(LogLevel::Info, &format!("Instance: {} stopped", instance.id()));

        Ok(())
    }
}
