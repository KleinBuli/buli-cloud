use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Group {
    name: String,
    template_names: Vec<String>,
    maintenance: bool,
    path: PathBuf,
}

impl Group {
    pub fn new(name: String, template_names: Vec<String>, path: PathBuf) -> Self {
        Self {
            name,
            template_names,
            path,
            maintenance: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
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
