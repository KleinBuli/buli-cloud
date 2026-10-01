use std::{collections::HashMap, path::PathBuf};

use tokio::{
    process::{Child, Command},
    sync::{Mutex, RwLock},
};

use crate::{
    instances::instance::{Instance, InstanceStatus},
    logger::logger::{LogLevel, log},
    templates::template::Template,
};

pub struct InstanceManager {
    running_path: PathBuf,
    instances: RwLock<HashMap<String, Instance>>,
    processes: Mutex<HashMap<String, Child>>,
}

impl InstanceManager {
    pub fn new(running_path: PathBuf) -> Self {
        Self {
            running_path,
            instances: RwLock::new(HashMap::new()),
            processes: Mutex::new(HashMap::new()),
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

    pub async fn remove_instance(&self, id: &str) -> Result<Option<Instance>, ()> {
        let processes = self.processes.lock().await;

        if processes.contains_key(id) {
            return Err(());
        }

        let mut instances = self.instances.write().await;

        Ok(instances.remove(id))
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
        {
            let instances = self.instances.read().await;

            if !instances.contains_key(id) {
                return Err(());
            }
        }

        {
            let mut processes = self.processes.lock().await;

            if processes.contains_key(id) {
                log(
                    LogLevel::Warn,
                    &format!("Couldn't start instance {} since it's already running.", id),
                );

                return Ok(());
            }

            {
                let mut instances = self.instances.write().await;
                let instance = instances.get_mut(id).ok_or(())?;

                instance.set_status(InstanceStatus::Starting);

                log(LogLevel::Info, &format!("Starting Instance: {} ...", instance.id()));
            }

            let child = match Command::new("java")
                .arg("-jar")
                .arg("paper.jar")
                .current_dir(self.running_path.join(id))
                .spawn()
            {
                Ok(child) => child,
                Err(_) => {
                    let mut instances = self.instances.write().await;

                    if let Some(instance) = instances.get_mut(id) {
                        instance.set_status(InstanceStatus::Stopped);
                    }

                    log(LogLevel::Error, &format!("Could not start instance {}", id));

                    return Err(());
                }
            };

            processes.insert(id.to_string(), child);
        }

        {
            let mut instances = self.instances.write().await;
            let instance = instances.get_mut(id).ok_or(())?;

            instance.set_status(InstanceStatus::Running);

            log(LogLevel::Info, &format!("Instance: {} started successfully", instance.id()));
        }

        Ok(())
    }

    pub async fn stop_instance(&self, id: &str) -> Result<(), ()> {
        {
            let instances = self.instances.read().await;

            if !instances.contains_key(id) {
                return Err(());
            }
        }

        let mut child = {
            let mut processes = self.processes.lock().await;

            match processes.remove(id) {
                Some(child) => child,
                None => {
                    log(LogLevel::Warn, &format!("Couldn't stop instance {} since it isn't running.", id));

                    return Ok(());
                }
            }
        };

        {
            let mut instances = self.instances.write().await;

            if let Some(instance) = instances.get_mut(id) {
                instance.set_status(InstanceStatus::Stopping);
            }
        }

        if child.kill().await.is_err() {
            let mut processes = self.processes.lock().await;
            processes.insert(id.to_string(), child);

            let mut instances = self.instances.write().await;

            if let Some(instance) = instances.get_mut(id) {
                instance.set_status(InstanceStatus::Running);
            }
            log(LogLevel::Error, &format!("Failed to stop instance {}", id));

            return Err(());
        }

        {
            let mut instances = self.instances.write().await;

            if let Some(instance) = instances.get_mut(id) {
                instance.set_status(InstanceStatus::Stopped);
            }
        }

        log(LogLevel::Info, &format!("Instance: {} stopped", id));

        Ok(())
    }
}
