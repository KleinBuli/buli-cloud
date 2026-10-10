use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
};

use tokio::sync::RwLock;

pub const MIN_PORT: u16 = 25565;
pub const MAX_PORT: u16 = 35565;

pub struct PortAllocator {
    ports: RwLock<HashMap<String, u16>>,
}

impl PortAllocator {
    pub fn new() -> Self {
        Self {
            ports: RwLock::new(HashMap::new()),
        }
    }

    pub async fn allocate_next(&self, instance_name: &str) -> Result<u16, Error> {
        let mut ports = self.ports.write().await;

        if let Some(port) = ports.get(instance_name) {
            return Ok(*port);
        }
        let mut port = MIN_PORT;
        loop {
            let allocated = ports.values().any(|allocated_port| *allocated_port == port);

            if !allocated {
                ports.insert(instance_name.to_string(), port);
                return Ok(port);
            }

            port += 1;

            if port == MAX_PORT {
                return Err(Error::new(ErrorKind::NotFound, "No possible port found to allocate."));
            }
        }
    }

    pub async fn allocate(&self, instance_name: &str, port: u16) -> Result<(), Error> {
        let mut ports = self.ports.write().await;

        if ports.values().any(|allocated_port| *allocated_port == port) {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                format!("Error while allocating Port: Port {port} is already allocated"),
            ));
        }

        ports.insert(instance_name.to_string(), port);

        Ok(())
    }

    pub async fn deallocate(&self, port: u16) -> Result<(), Error> {
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

    pub async fn get_corresponding_instance_name(&self, test: u16) -> Option<String> {
        let ports = self.ports.read().await;

        ports
            .iter()
            .find(|(_, port)| **port == test)
            .map(|(instance_name, _)| instance_name.clone())
    }

    pub async fn get_corresponding_port(&self, instance_name: &str) -> Option<u16> {
        let ports = self.ports.read().await;

        ports.iter().find(|(name, _)| name == &instance_name).map(|(_, port)| port.clone())
    }

    pub async fn is_allocated(&self, test: u16) -> bool {
        let ports = self.ports.read().await;
        ports.values().any(|port| port == &test)
    }

    pub async fn allocated_ports(&self) -> Vec<u16> {
        let ports = self.ports.read().await;
        ports.values().cloned().collect()
    }
}
