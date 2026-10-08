use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::util::software::softwaremanager::ServerSoftware;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticInstanceConfig {
    software: ServerSoftware,
    jar_name: String,
    port: u16,
    min_memory_mb: u32,
    max_memory_mb: u32,
    auto_start: bool,
}

impl StaticInstanceConfig {
    pub fn software(&self) -> &ServerSoftware {
        &self.software
    }

    pub fn jar_name(&self) -> &str {
        &self.jar_name
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn min_memory_mb(&self) -> u32 {
        self.min_memory_mb
    }

    pub fn max_memory_mb(&self) -> u32 {
        self.max_memory_mb
    }

    pub fn auto_start(&self) -> bool {
        self.auto_start
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct StaticInstancesConfig {
    #[serde(default)]
    pub proxy: HashMap<String, StaticInstanceConfig>,

    #[serde(default)]
    pub server: HashMap<String, StaticInstanceConfig>,
}
