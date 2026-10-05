use serde::{Deserialize, Serialize};

use crate::util::software::softwaremanager::ServerSoftware;

#[derive(Serialize, Deserialize, Debug)]
pub struct Template {
    name: String,
    server_software: ServerSoftware,
    minecraft_version: Option<String>,
    min_memory_mb: i32,
    max_memory_mb: i32,
    min_instances: i16,
    max_instances: i16,
    new_instances_player_percentage: i8,
    auto_copy_on_stop: bool,
}

impl Clone for Template {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            server_software: self.server_software.clone(),
            minecraft_version: self.minecraft_version.clone(),
            min_memory_mb: self.min_memory_mb.clone(),
            max_memory_mb: self.max_memory_mb.clone(),
            min_instances: self.min_instances.clone(),
            max_instances: self.max_instances.clone(),
            new_instances_player_percentage: self.new_instances_player_percentage.clone(),
            auto_copy_on_stop: self.auto_copy_on_stop.clone(),
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

    pub fn minecraft_version(&self) -> &Option<String> {
        &self.minecraft_version
    }

    pub fn min_memory_mb(&self) -> &i32 {
        &self.min_memory_mb
    }

    pub fn max_memory_mb(&self) -> &i32 {
        &self.max_memory_mb
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
