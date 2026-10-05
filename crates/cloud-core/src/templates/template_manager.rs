use crate::{
    templates::template::Template,
    util::{
        file_utils::{atomic_write, validate_name},
        software::softwaremanager::ServerSoftware,
    },
};
use std::{
    collections::HashMap,
    fs,
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
};
use tokio::sync::RwLock;

pub const TEMPLATE_CONFIG: &str = ".template.toml";

pub struct TemplateManager {
    templates_path: PathBuf,
    software_path: PathBuf,
    templates: RwLock<HashMap<(String, String), Template>>,
}

impl TemplateManager {
    pub fn new(templates_path: PathBuf, software_path: PathBuf) -> Self {
        Self {
            templates_path,
            software_path,
            templates: RwLock::new(HashMap::new()),
        }
    }

    pub fn templates_path(&self) -> PathBuf {
        self.templates_path.clone()
    }

    pub(crate) fn checked_path(&self, group: &str, name: &str) -> Result<PathBuf, Error> {
        validate_name(group)?;
        validate_name(name)?;
        let group_path = self.templates_path.join(group);
        let path = group_path.join(name);
        for candidate in [&group_path, &path] {
            if candidate.exists() && !candidate.canonicalize()?.starts_with(self.templates_path.canonicalize()?) {
                return Err(Error::new(ErrorKind::InvalidInput, "Template path escapes template root"));
            }
            if let Ok(metadata) = fs::symlink_metadata(candidate)
                && metadata.file_type().is_symlink()
            {
                return Err(Error::new(ErrorKind::InvalidInput, "Linked template directories are not supported"));
            }
        }
        Ok(path)
    }

    fn read_template(&self, path: &Path, name: &str, global_version: &str) -> Result<Template, Error> {
        let config = path.join(TEMPLATE_CONFIG);
        let template = if config.exists() {
            toml::from_str::<Template>(&fs::read_to_string(&config)?).map_err(Error::other)?
        } else {
            let mut candidates = Vec::new();
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                if !entry.file_type()?.is_file() {
                    continue;
                }
                let file_name = entry.file_name();
                let file_name = file_name.to_string_lossy();
                if file_name == "velocity.jar" {
                    candidates.push(Template::new(name, ServerSoftware::Velocity, None));
                }
                for (prefix, software) in [("paper-", ServerSoftware::Paper), ("vanilla-", ServerSoftware::Vanilla)] {
                    if let Some(version) = file_name.strip_prefix(prefix).and_then(|s| s.strip_suffix(".jar")) {
                        candidates.push(Template::new(name, software, Some(version.to_string())));
                    }
                }
            }
            let template = match candidates.len() {
                1 => candidates.remove(0),
                0 if name == "global" => {
                    let template = Template::new(name, ServerSoftware::Paper, Some(global_version.to_string()));
                    let jar = template.jar_name()?;
                    fs::copy(self.software_path.join(&jar), path.join(&jar))?;
                    template
                }
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!("Cannot infer software/version for {}. Add {TEMPLATE_CONFIG}.", path.display()),
                    ));
                }
            };
            atomic_write(&config, toml::to_string_pretty(&template).map_err(Error::other)?.as_bytes())?;
            template
        };
        if template.name() != name {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Template metadata name does not match directory",
            ));
        }
        if !path.join(template.jar_name()?).is_file() {
            return Err(Error::new(ErrorKind::NotFound, format!("Executable missing in {}", path.display())));
        }
        Ok(template)
    }

    pub(crate) async fn load_group_templates(&self, group: &str, names: &[String], global_version: &str) -> Result<(), Error> {
        let mut templates = self.templates.write().await;
        let mut loaded = Vec::new();
        for name in names {
            if loaded
                .iter()
                .any(|((_, existing), _): &((String, String), Template)| existing.eq_ignore_ascii_case(name))
            {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Template names must be unique regardless of case",
                ));
            }
            let path = self.checked_path(group, name)?;
            loaded.push(((group.to_string(), name.clone()), self.read_template(&path, name, global_version)?));
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
    ) -> Result<(), Error> {
        let mut templates = self.templates.write().await;
        let path = self.checked_path(group, name)?;
        if path.exists() || templates.keys().any(|(g, n)| g == group && n.eq_ignore_ascii_case(name)) {
            return Err(Error::new(ErrorKind::AlreadyExists, "Template already exists"));
        }
        let template = Template::new(name, software, version);
        let jar = template.jar_name()?;
        let source = self.software_path.join(&jar);
        if !source.is_file() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Server software is not cached: {}", source.display()),
            ));
        }
        let parent = path.parent().unwrap();
        fs::create_dir_all(parent)?;
        let staged = tempfile::Builder::new().prefix(".creating-").tempdir_in(parent)?;
        fs::copy(source, staged.path().join(&jar))?;
        atomic_write(
            &staged.path().join(TEMPLATE_CONFIG),
            toml::to_string_pretty(&template).map_err(Error::other)?.as_bytes(),
        )?;
        fs::rename(staged.path(), &path)?;
        templates.insert((group.to_string(), name.to_string()), template);
        Ok(())
    }

    pub(crate) async fn forget_template(&self, group: &str, name: &str) {
        self.templates.write().await.remove(&(group.to_string(), name.to_string()));
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

    pub fn exists(&self, group: &str, name: &str) -> bool {
        self.get_template_path(group, name).is_some()
    }
}
