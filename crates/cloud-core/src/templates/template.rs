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
    static_instance: bool,
    #[serde(default)]
    role: Option<ServiceRole>,
    #[serde(default)]
    port: Option<u16>,
    #[serde(default = "default_host")]
    host: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ServiceRole {
    Proxy,
    Backend,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

impl Template {
    pub fn role(&self) -> ServiceRole {
        self.role.clone().unwrap_or_else(|| {
            if self.server_software == ServerSoftware::Velocity {
                ServiceRole::Proxy
            } else {
                ServiceRole::Backend
            }
        })
    }
    pub fn port(&self) -> Option<u16> {
        self.port
    }
    pub fn host(&self) -> &str {
        &self.host
    }
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
            static_instance: matches!(server_software, ServerSoftware::Velocity),
            role: None,
            port: None,
            host: default_host(),
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_static_instance(&self) -> bool {
        self.static_instance
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_templates_and_fixed_proxy_ports_are_supported() {
        let template = Template::new("proxy", ServerSoftware::Velocity, None, None);
        let legacy = toml::to_string(&template)
            .unwrap()
            .lines()
            .filter(|line| !line.starts_with("host ="))
            .collect::<Vec<_>>()
            .join("\n");
        let legacy: Template = toml::from_str(&legacy).unwrap();
        assert_eq!(legacy.role(), ServiceRole::Proxy);
        assert_eq!(legacy.port(), None);
        assert_eq!(legacy.host(), "127.0.0.1");
        let configured = format!("{}\nport = 25577\nrole = 'Backend'\n", toml::to_string(&template).unwrap());
        let configured: Template = toml::from_str(&configured).unwrap();
        assert_eq!(configured.port(), Some(25577));
        assert_eq!(configured.role(), ServiceRole::Backend);
    }
}
