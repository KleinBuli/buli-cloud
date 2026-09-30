use std::collections::HashMap;

use tokio::sync::RwLock;

use crate::{instances::instance::Instance, templates::template::Template};

pub struct InstanceManager {
    instances: RwLock<HashMap<String, Instance>>,
}

impl InstanceManager {
    pub fn new() -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
        }
    }

    pub async fn add_instance(&self, instance: Instance) {
        let mut instances = self.instances.write().await;
        instances.insert(instance.id().to_string(), instance);
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
        let instances = self.instances_list().await;
        let mut number = 1;

        loop {
            let id = format!("{}-{}", template.name(), number);

            if !instances.iter().any(|inst| inst.id() == id) {
                let new_instance = Instance::new(&id, Some(template.name().to_string()));

                self.add_instance(new_instance.clone()).await;
                return new_instance;
            }

            number += 1;
        }
    }

    /// TODO
    pub fn create_instance_as_static(&self) {}
}
