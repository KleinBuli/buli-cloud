use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Instance {
    id: String,
    group_name: String,
    template_name: Option<String>,
    status: InstanceStatus,
}

impl Instance {
    pub fn new(id: &str, group_name: &str, template_name: Option<String>) -> Self {
        Self {
            id: id.to_string(),
            template_name,
            group_name: group_name.to_string(),
            status: InstanceStatus::Stopped,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn group_name(&self) -> &str {
        &self.group_name
    }

    pub fn template_name(&self) -> Option<&str> {
        self.template_name.as_deref()
    }

    pub fn status(&self) -> &InstanceStatus {
        &self.status
    }

    pub fn set_status(&mut self, status: InstanceStatus) {
        self.status = status
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstanceStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
}
