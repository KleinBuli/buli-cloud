use crate::{
    groups::group::Group,
    util::file_utils::{atomic_write, validate_name},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    io::{Error, ErrorKind},
    path::PathBuf,
};
use tokio::sync::RwLock;

pub struct GroupManager {
    groups: RwLock<HashMap<String, Group>>,
    config_path: PathBuf,
}

#[derive(Default, Serialize, Deserialize)]
struct GroupsConfig {
    #[serde(default)]
    group: BTreeMap<String, GroupConfig>,
}

#[derive(Serialize, Deserialize)]
struct GroupConfig {
    path: PathBuf,
    #[serde(default)]
    maintenance: bool,
    #[serde(default)]
    template_names: Vec<String>,
}

impl GroupManager {
    pub fn new(config_path: PathBuf) -> Result<Self, Error> {
        if !config_path.exists() {
            atomic_write(&config_path, b"")?;
        }
        Ok(Self {
            groups: RwLock::new(HashMap::new()),
            config_path,
        })
    }

    pub(crate) async fn load_groups_from_config(&self) -> Result<(), Error> {
        let mut groups = self.groups.write().await;
        let content = fs::read_to_string(&self.config_path)?;
        let config: GroupsConfig = toml::from_str(&content).map_err(Error::other)?;
        let mut loaded = HashMap::new();
        for (name, config) in config.group {
            validate_name(&name)?;
            if loaded.keys().any(|existing: &String| existing.eq_ignore_ascii_case(&name)) {
                return Err(Error::new(ErrorKind::InvalidData, "Group names must be unique regardless of case"));
            }
            for template in &config.template_names {
                validate_name(template)?;
            }
            let mut group = Group::new(name.clone(), Vec::new(), config.path);
            for template in config.template_names {
                group.add_template(template);
            }
            group.set_maintenance(config.maintenance);
            loaded.insert(name, group);
        }
        *groups = loaded;
        Ok(())
    }

    fn persist(&self, groups: &HashMap<String, Group>) -> Result<(), Error> {
        let config = GroupsConfig {
            group: groups
                .iter()
                .map(|(name, group)| {
                    (
                        name.clone(),
                        GroupConfig {
                            path: group.path().clone(),
                            maintenance: group.maintenance(),
                            template_names: group.template_names().to_vec(),
                        },
                    )
                })
                .collect(),
        };
        atomic_write(&self.config_path, toml::to_string_pretty(&config).map_err(Error::other)?.as_bytes())
    }

    async fn update(&self, change: impl FnOnce(&mut HashMap<String, Group>) -> Result<(), Error>) -> Result<(), Error> {
        let mut groups = self.groups.write().await;
        let mut next = groups.clone();
        change(&mut next)?;
        self.persist(&next)?;
        *groups = next;
        Ok(())
    }

    pub(crate) async fn add_group(&self, group: Group) -> Result<(), Error> {
        validate_name(group.name())?;
        for name in group.template_names() {
            validate_name(name)?;
        }
        self.update(|groups| {
            if groups.keys().any(|name| name.eq_ignore_ascii_case(group.name())) {
                return Err(Error::new(ErrorKind::AlreadyExists, "Group already exists"));
            }
            groups.insert(group.name().to_string(), group);
            Ok(())
        })
        .await
    }

    pub(crate) async fn remove_group(&self, name: &str) -> Result<(), Error> {
        self.update(|groups| {
            groups
                .remove(name)
                .ok_or_else(|| Error::new(ErrorKind::NotFound, "Group not found"))?;
            Ok(())
        })
        .await
    }

    pub(crate) async fn add_template(&self, group: &str, template: &str) -> Result<(), Error> {
        validate_name(template)?;
        self.update(|groups| {
            groups
                .get_mut(group)
                .ok_or_else(|| Error::new(ErrorKind::NotFound, "Group not found"))?
                .add_template(template.to_string());
            Ok(())
        })
        .await
    }

    pub(crate) async fn delete_template(&self, group: &str, template: &str) -> Result<(), Error> {
        self.update(|groups| {
            let group = groups
                .get_mut(group)
                .ok_or_else(|| Error::new(ErrorKind::NotFound, "Group not found"))?;
            if !group.template_names().iter().any(|name| name == template) {
                return Err(Error::new(ErrorKind::NotFound, "Template not assigned to group"));
            }
            group.remove_template(template);
            Ok(())
        })
        .await
    }

    pub(crate) async fn set_maintenance(&self, name: &str, maintenance: bool) -> Result<(), Error> {
        self.update(|groups| {
            groups
                .get_mut(name)
                .ok_or_else(|| Error::new(ErrorKind::NotFound, "Group not found"))?
                .set_maintenance(maintenance);
            Ok(())
        })
        .await
    }

    pub async fn get_group(&self, name: &str) -> Option<Group> {
        self.groups.read().await.get(name).cloned()
    }

    pub async fn groups(&self) -> Vec<Group> {
        let mut groups: Vec<_> = self.groups.read().await.values().cloned().collect();
        groups.sort_by(|a, b| a.name().cmp(b.name()));
        groups
    }
}
