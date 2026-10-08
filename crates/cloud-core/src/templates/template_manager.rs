use crate::{
    templates::template::Template,
    util::{
        file_utils::{atomic_write, validate_name},
        software::softwaremanager::ServerSoftware,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
};
use tokio::sync::RwLock;

pub const GROUP_CONFIG: &str = "templates.toml";

#[derive(Default, Serialize, Deserialize)]
struct GroupConfig {
    #[serde(default)]
    templates: BTreeMap<String, Template>,
}

pub struct TemplateManager {
    templates_path: PathBuf,
    templates: RwLock<HashMap<(String, String), Template>>,
}

impl TemplateManager {
    pub fn new(templates_path: PathBuf) -> Self {
        Self {
            templates_path,
            templates: RwLock::new(HashMap::new()),
        }
    }

    pub fn templates_path(&self) -> PathBuf {
        self.templates_path.clone()
    }

    fn checked_group_path(&self, group: &str) -> Result<PathBuf, Error> {
        validate_name(group)?;
        let path = self.templates_path.join(group);
        self.check_directory(&path)?;
        Ok(path)
    }

    fn check_directory(&self, path: &Path) -> Result<(), Error> {
        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(Error::new(ErrorKind::InvalidInput, "Expected an unlinked template directory"));
                }
                if !path.canonicalize()?.starts_with(self.templates_path.canonicalize()?) {
                    return Err(Error::new(ErrorKind::InvalidInput, "Template path escapes template root"));
                }
                Ok(())
            }
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub(crate) fn checked_path(&self, group: &str, name: &str) -> Result<PathBuf, Error> {
        validate_name(name)?;
        if name.eq_ignore_ascii_case(GROUP_CONFIG) {
            return Err(Error::new(ErrorKind::InvalidInput, "Template name conflicts with templates.toml"));
        }
        let path = self.checked_group_path(group)?.join(name);
        self.check_directory(&path)?;
        Ok(path)
    }

    fn read_group_config(&self, group: &str) -> Result<GroupConfig, Error> {
        let path = self.checked_group_path(group)?.join(GROUP_CONFIG);
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(GroupConfig::default()),
            Err(e) => return Err(e),
        };
        let config: GroupConfig = toml::from_str(&content).map_err(Error::other)?;
        let mut names = std::collections::HashSet::new();
        for (name, template) in &config.templates {
            self.checked_path(group, name)?;
            if !names.insert(name.to_ascii_lowercase()) {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Template names must be unique regardless of case",
                ));
            }
            Self::validate_metadata(name, template)?;
        }
        Ok(config)
    }

    fn write_group_config(&self, group: &str, config: &GroupConfig) -> Result<(), Error> {
        let path = self.checked_group_path(group)?;
        fs::create_dir_all(&path)?;
        let content = format!(
            "# Template metadata for this group.\n# For CUSTOM, custom_server_software_jar_name is the JAR filename in the template folder.\n{}",
            toml::to_string_pretty(config).map_err(Error::other)?
        );
        atomic_write(&path.join(GROUP_CONFIG), content.as_bytes())
    }

    fn validate_metadata(name: &str, template: &Template) -> Result<(), Error> {
        if template.name() != name {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Template metadata name does not match configuration key",
            ));
        }

        validate_name(&template.jar_name()?)?;
        Ok(())
    }

    fn validate_template(&self, group: &str, name: &str, template: &Template) -> Result<(), Error> {
        Self::validate_metadata(name, template)?;
        let path = self.checked_path(group, name)?;
        if !path.is_dir() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Template directory missing: {}", path.display()),
            ));
        }
        Ok(())
    }

    pub(crate) async fn load_group_templates(&self, group: &str, names: &[String], global_version: &str) -> Result<(), Error> {
        let mut templates = self.templates.write().await;
        let mut config = self.read_group_config(group)?;
        let mut loaded = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut changed = false;

        for name in names {
            if !seen.insert(name.to_ascii_lowercase()) {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Template names must be unique regardless of case",
                ));
            }
            let _path = self.checked_path(group, name)?;
            let template = if let Some(template) = config.templates.get(name) {
                template.clone()
            } else {
                if config.templates.keys().any(|existing| existing.eq_ignore_ascii_case(name)) {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "Template name case differs from group configuration",
                    ));
                }

                let template = if name == "global" {
                    Template::new(name, ServerSoftware::Paper, Some(global_version.to_string()), None)
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!("Missing metadata for {name}. Add [templates.{name}] to {group}/{GROUP_CONFIG}."),
                    ));
                };

                config.templates.insert(name.clone(), template.clone());
                changed = true;
                template
            };
            self.validate_template(group, name, &template)?;
            loaded.push(((group.to_string(), name.clone()), template));
        }
        if changed {
            self.write_group_config(group, &config)?;
        }
        templates.retain(|(g, _), _| g != group);
        templates.extend(loaded);
        Ok(())
    }

    pub(crate) async fn create_new_template(
        &self,
        group: &str,
        name: &str,
        software: ServerSoftware,
        version: Option<String>,
        custom_jar_name: Option<String>,
    ) -> Result<(), Error> {
        let mut templates = self.templates.write().await;
        let path = self.checked_path(group, name)?;
        let mut config = self.read_group_config(group)?;
        if path.try_exists()?
            || templates.keys().any(|(g, n)| g == group && n.eq_ignore_ascii_case(name))
            || config.templates.keys().any(|n| n.eq_ignore_ascii_case(name))
        {
            return Err(Error::new(ErrorKind::AlreadyExists, "Template already exists"));
        }
        let custom_name = match software {
            ServerSoftware::Custom => {
                let name = custom_jar_name.ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Custom JAR filename is required"))?;

                validate_name(&name)?;

                Some(name)
            }

            _ => {
                if custom_jar_name.is_some() {
                    return Err(Error::new(ErrorKind::InvalidInput, "Custom JAR name is only allowed for CUSTOM"));
                }

                None
            }
        };

        let template = Template::new(name, software, version, custom_name);
        Self::validate_metadata(name, &template)?;
        let parent = self.checked_group_path(group)?;
        fs::create_dir_all(&parent)?;
        let staged = tempfile::Builder::new().prefix(".creating-").tempdir_in(&parent)?;
        config.templates.insert(name.to_string(), template.clone());
        fs::rename(staged.path(), &path)?;
        if let Err(error) = self.write_group_config(group, &config) {
            if let Err(rollback) = fs::rename(&path, staged.path()) {
                return Err(Error::other(format!(
                    "Could not save group configuration: {error}; rollback failed: {rollback}. Inspect {}",
                    path.display()
                )));
            }
            return Err(error);
        }
        templates.insert((group.to_string(), name.to_string()), template);
        Ok(())
    }

    pub(crate) async fn forget_template(&self, group: &str, name: &str) -> Result<(), Error> {
        let mut templates = self.templates.write().await;
        self.checked_path(group, name)?;
        let mut config = self.read_group_config(group)?;
        if config.templates.remove(name).is_some() {
            self.write_group_config(group, &config)?;
        }
        templates.remove(&(group.to_string(), name.to_string()));
        Ok(())
    }

    pub(crate) async fn forget_group(&self, group: &str) {
        self.templates.write().await.retain(|(g, _), _| g != group);
    }

    pub fn get_template_path(&self, group: &str, name: &str) -> Option<PathBuf> {
        self.checked_path(group, name).ok().filter(|path| path.is_dir())
    }

    pub async fn templates_list(&self, group: &str) -> Vec<Template> {
        let templates = self.templates.read().await;
        let mut result: Vec<_> = templates.iter().filter(|((g, _), _)| g == group).map(|(_, t)| t.clone()).collect();
        result.sort_by(|a, b| a.name().cmp(b.name()));
        result
    }

    pub async fn all_templates(&self) -> Vec<Template> {
        self.templates.read().await.values().cloned().collect()
    }

    pub async fn get_template(&self, group: &str, name: &str) -> Option<Template> {
        self.templates.read().await.get(&(group.to_string(), name.to_string())).cloned()
    }

    pub fn exists_on_disk(&self, group: &str, name: &str) -> bool {
        self.get_template_path(group, name).is_some()
    }
}
