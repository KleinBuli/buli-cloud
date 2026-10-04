use serde::Serialize;

use crate::util::software::softwaremanager::ServerSoftware;

#[derive(Serialize)]
pub struct Template {
    name: String,
    server_software: ServerSoftware,
    minecraft_version: Option<String>,
}

impl Clone for Template {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            server_software: self.server_software.clone(),
            minecraft_version: self.minecraft_version.clone(),
        }
    }
}

impl Template {
    pub fn new(name: &str, server_software: ServerSoftware, minecraft_version: Option<String>) -> Self {
        Self {
            name: String::from(name),
            server_software,
            minecraft_version,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn server_software(&self) -> &ServerSoftware {
        &self.server_software
    }

    pub fn minecraft_version(&self) -> &Option<String> {
        &self.minecraft_version
    }
}
