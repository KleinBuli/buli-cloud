use serde::Serialize;

#[derive(Serialize)]
pub struct Template {
    name: String,
}

impl Template {
    pub fn new(name: &str) -> Self {
        Self { name: String::from(name) }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
