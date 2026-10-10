use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Instance {
    id: String,
    group_name: String,
    template_name: String,
    instance_mode: InstanceMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstanceMode {
    Dynamic,
    Static,
}

impl Instance {
    pub fn new(id: &str, group_name: &str, template_name: String, instance_mode: InstanceMode) -> Self {
        Self {
            id: id.to_string(),
            template_name,
            group_name: group_name.to_string(),
            instance_mode,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn group_name(&self) -> &str {
        &self.group_name
    }

    pub fn template_name(&self) -> &str {
        &self.template_name
    }

    pub fn instance_mode(&self) -> &InstanceMode {
        &self.instance_mode
    }
}
