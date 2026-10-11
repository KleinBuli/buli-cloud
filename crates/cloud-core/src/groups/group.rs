use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    groups::group::GroupRole::{ProxiedServer, Proxy, StandaloneServer},
    util::software::softwaremanager::ServerSoftware,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Group {
    name: String,
    template_names: Vec<String>,
    role: GroupRole,
    maintenance: bool,
    path: PathBuf,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum GroupRole {
    Proxy,
    StandaloneServer,
    ProxiedServer,
}

impl GroupRole {
    pub fn to_string(&self) -> String {
        match self {
            Proxy => "Proxy".to_string(),
            StandaloneServer => "StandaloneServer".to_string(),
            ProxiedServer => "ProxiedServer".to_string(),
        }
    }

    pub fn allows_software(&self, software: &ServerSoftware) -> bool {
        match self {
            Self::Proxy => software == &ServerSoftware::Velocity,
            Self::StandaloneServer | Self::ProxiedServer => software == &ServerSoftware::Paper || software == &ServerSoftware::Custom,
        }
    }
}

impl Group {
    pub fn new(name: String, template_names: Vec<String>, path: PathBuf, role: GroupRole) -> Self {
        Self {
            name,
            template_names,
            role,
            path,
            maintenance: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn role(&self) -> GroupRole {
        self.role
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn maintenance(&self) -> bool {
        self.maintenance
    }

    pub fn set_maintenance(&mut self, value: bool) {
        self.maintenance = value
    }

    pub fn add_template(&mut self, template_name: String) {
        if !self.template_names.contains(&template_name) {
            self.template_names.push(template_name);
        }
    }

    pub fn remove_template(&mut self, template_name: &str) {
        self.template_names.retain(|name| name != template_name);
    }

    pub fn template_names(&self) -> &[String] {
        &self.template_names
    }
}
