use serde::{Deserialize, Serialize};

use crate::util::software::softwaremanager::ServerSoftware;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Template {
    name: String,
    server_software: ServerSoftware,
    custom_server_software_jar_name: Option<String>,
    minecraft_version: Option<String>,
    min_memory_mb: u32,
    max_memory_mb: u32,
    min_instances: i16,
    max_instances: i16,
    new_instances_player_percentage: i8,
    auto_copy_on_stop: bool,
}

impl Template {
    pub fn jar_name(&self) -> std::io::Result<String> {
        use std::io::{Error, ErrorKind};
        match self.server_software {
            ServerSoftware::Velocity => Ok("velocity.jar".to_string()),
            ServerSoftware::Custom => {
                let name = self
                    .custom_server_software_jar_name
                    .as_deref()
                    .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Custom JAR filename is required"))?;

                if !name.to_ascii_lowercase().ends_with(".jar") {
                    return Err(Error::new(ErrorKind::InvalidInput, "Custom server file must have be .jar file"));
                }

                crate::util::file_utils::validate_name(name)?;
                Ok(name.to_string())
            }
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
    pub fn new(name: &str, server_software: ServerSoftware, minecraft_version: Option<String>, custom_jar_name: Option<String>) -> Self {
        Self {
            name: String::from(name),
            server_software,
            minecraft_version,
            custom_server_software_jar_name: custom_jar_name,
            min_memory_mb: 512,
            max_memory_mb: 512,
            min_instances: 0,
            max_instances: -1,
            new_instances_player_percentage: 100,
            auto_copy_on_stop: false,
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn server_software(&self) -> &ServerSoftware {
        &self.server_software
    }

    pub fn custom_jar_name(&self) -> &Option<String> {
        &self.custom_server_software_jar_name
    }

    pub fn minecraft_version(&self) -> &Option<String> {
        &self.minecraft_version
    }

    pub fn min_memory_mb(&self) -> u32 {
        self.min_memory_mb
    }

    pub fn max_memory_mb(&self) -> u32 {
        self.max_memory_mb
    }

    pub fn min_instances(&self) -> &i16 {
        &self.min_instances
    }

    pub fn max_instances(&self) -> &i16 {
        &self.max_instances
    }

    pub fn new_instances_player_percentage(&self) -> &i8 {
        &self.new_instances_player_percentage
    }

    pub fn auto_copy_on_stop(&self) -> bool {
        self.auto_copy_on_stop
    }
}
