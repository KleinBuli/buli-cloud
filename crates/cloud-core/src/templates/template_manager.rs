use std::{
    collections::HashMap,
    fs,
    io::{
        Error,
        ErrorKind::{AlreadyExists, NotFound},
    },
    path::PathBuf,
};

use tokio::sync::RwLock;

use crate::templates::template::Template;

/// Manages all template-related filesystem operations.
///
/// The `TemplateManager` is responsible for creating, deleting,
/// listing and locating server templates inside the configured
/// templates directory.
pub struct TemplateManager {
    templates_path: PathBuf,
    templates: RwLock<HashMap<String, Template>>,
}

impl TemplateManager {
    /// Creates a new `TemplateManager`.
    ///
    /// # Arguments
    ///
    /// * `templates_path` - The directory in which all templates are stored.
    ///
    /// # Returns
    ///
    /// A new `TemplateManager` instance.
    pub fn new(templates_path: PathBuf) -> Self {
        Self {
            templates_path,
            templates: RwLock::new(HashMap::new()),
        }
    }

    /// Reads all templates from the disk and puts them into the HashMap.
    ///
    /// # Errors
    /// throws an error if an error occurs while reading the templates from the disk
    pub async fn load_templates(&self) -> Result<(), Error> {
        let templates = self.templates_list_from_disk()?;
        let mut cache = self.templates.write().await;

        cache.clear();

        for template in templates {
            cache.insert(template.name().to_string(), template);
        }

        Ok(())
    }

    /// Returns the root directory in which templates are stored.
    ///
    /// # Returns
    ///
    /// A copy of the configured templates path.
    pub fn templates_path(&self) -> PathBuf {
        self.templates_path.to_path_buf()
    }

    /// The template will be created as a subdirectory of the configured
    /// templates directory.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the template to create.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// * a template with the same name already exists
    /// * the directory cannot be created
    pub async fn create_new_template(&self, name: &str) -> Result<(), Error> {
        let template_path = self.templates_path.join(name);

        if self.exists(name) {
            return Err(Error::new(AlreadyExists, format!("The template {name} already exists.")));
        }

        fs::create_dir_all(template_path)?;
        let template = Template::new(name);
        let mut templates = self.templates.write().await;

        templates.insert(name.to_string(), template);
        Ok(())
    }

    /// Deletes an existing template and all of its contents.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the template to delete.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// * the template does not exist
    /// * the template directory cannot be removed
    pub async fn delete_template(&self, name: &str) -> Result<(), Error> {
        let template_path = self.templates_path.join(name);
        if !self.exists(name) {
            return Err(Error::new(NotFound, format!("The template {name} doesn't exist.")));
        }

        fs::remove_dir_all(template_path)?;
        let mut templates = self.templates.write().await;

        templates.remove(name);
        Ok(())
    }

    /// Returns the filesystem path of a template.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the template.
    ///
    /// # Returns
    ///
    /// `Some(PathBuf)` if the template exists, otherwise `None`.
    pub fn get_template_path(&self, name: &str) -> Option<PathBuf> {
        if !self.exists(name) {
            return None;
        }

        Some(self.templates_path.join(name))
    }

    /// Lists all available templates.
    ///
    /// Only directories inside the configured templates directory
    /// are treated as templates.
    ///
    /// The returned template names are sorted alphabetically.
    ///
    /// # Returns
    ///
    /// A vector containing the names of all available templates.
    ///
    /// # Errors
    ///
    /// Returns an error if the templates directory cannot be read
    /// or one of its directory entries cannot be accessed.
    pub fn templates_list_from_disk(&self) -> Result<Vec<Template>, Error> {
        let mut templates: Vec<Template> = Vec::new();
        for entry in fs::read_dir(&self.templates_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(folder_name) = path.file_name() {
                    if let Some(name_str) = folder_name.to_str() {
                        templates.push(Template::new(name_str));
                    }
                }
            }
        }
        templates.sort_by(|a, b| a.name().cmp(b.name()));
        Ok(templates)
    }

    /// Returns all currently loaded templates.
    ///
    /// Templates are read from the in-memory cache and sorted
    /// alphabetically by name.
    ///
    /// # Returns
    ///
    /// A vector containing all loaded templates.
    pub async fn templates_list(&self) -> Vec<Template> {
        let templates = self.templates.read().await;
        templates.values().cloned().collect()
    }

    pub async fn get_template(&self, name: &str) -> Option<Template> {
        let templates = self.templates.read().await;

        templates.get(name).cloned()
    }

    /// Checks whether a template with the given name exists.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the template to check.
    ///
    /// # Returns
    ///
    /// `true` if the template path exists, otherwise `false`.
    pub fn exists(&self, name: &str) -> bool {
        self.templates_path.join(name).exists()
    }
}
