use serde::{Deserialize, Serialize};

use crate::util::software::softwaremanager::ServerSoftware;

#[derive(Serialize, Deserialize, Debug)]
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
    pub fn jar_name(&self) -> std::io::Result<String> {
        use std::io::{Error, ErrorKind};
        match self.server_software {
            ServerSoftware::Velocity => Ok("velocity.jar".to_string()),
            _ => {
                let version = self
                    .minecraft_version
                    .as_deref()
                    .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Minecraft version is required"))?;
                crate::util::file_utils::validate_name(version)?;
                Ok(format!("{}-{version}.jar", self.server_software))
            }
        }
    }
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
