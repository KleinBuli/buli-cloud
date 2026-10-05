use std::{collections::HashMap, io::Error};

use tokio::sync::RwLock;
pub struct PortAllocator {
    ports: RwLock<HashMap<String, i32>>,
}

impl PortAllocator {
    pub fn new() -> Self {
        Self {
            ports: RwLock::new(HashMap::new()),
        }
    }

    pub async fn allocate(&self, instance_name: &str, port: i32) -> Result<(), Error> {
        let mut ports = self.ports.write().await;

        if self.is_allocated(port).await {
            return Err(Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("Error while allocating Port: Port {port} is already allocated"),
            ));
        }

        ports.insert(instance_name.to_string(), port);

        Ok(())
    }

    pub async fn deallocate(&self, port: i32) -> Result<(), Error> {
        let mut ports = self.ports.write().await;

        let instance_name = ports
            .iter()
            .find(|(_, allocated_port)| **allocated_port == port)
            .map(|(instance_name, _)| instance_name.clone());

        match instance_name {
            Some(instance_name) => {
                ports.remove(&instance_name);
                Ok(())
            }
            None => Err(Error::new(std::io::ErrorKind::NotFound, format!("Port {port} is not allocated"))),
        }
    }

    pub async fn get_corresponding_instance_name(&self, test: i32) -> Option<String> {
        let ports = self.ports.read().await;

        ports
            .iter()
            .find(|(_, port)| **port == test)
            .map(|(instance_name, _)| instance_name.clone())
    }

    pub async fn is_allocated(&self, test: i32) -> bool {
        let ports = self.ports.read().await;
        ports.values().any(|port| port == &test)
    }

    pub async fn allocated_ports(&self) -> Vec<i32> {
        let ports = self.ports.read().await;
        ports.values().cloned().collect()
    }
}
