use std::{
    fs::{self},
    path::PathBuf,
};

/// Central runtime core of BuliCloud.
///
/// The `CloudCore` manages the root directory and provides access
/// to the main runtime paths used by the cloud system.
pub struct CloudCore {
    root_path: PathBuf,
}

impl CloudCore {
    /// Create new `CloudCore`
    ///
    /// # Arguments
    /// * `root_path` - root directory for buli cloud
    /// # Returns
    /// * new CloudCore instance
    pub fn new<P: Into<PathBuf>>(root_path: P) -> Self {
        Self {
            root_path: root_path.into(),
        }
    }

    /// Returns the path to the templates directory
    pub fn templates_path(&self) -> PathBuf {
        self.root_path.join("templates")
    }

    /// Returns the path to the running services directory
    pub fn running_path(&self) -> PathBuf {
        self.root_path.join("running")
    }

    /// Returns the path to the config directory
    pub fn config_path(&self) -> PathBuf {
        self.root_path.join("config")
    }

    /// Returns the path to the static servers directory
    pub fn static_servers_path(&self) -> PathBuf {
        self.root_path.join("static-servers")
    }

    /// initalizes the CloudCore
    /// # Returns
    /// * Result<()>
    pub fn initialize(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.templates_path())?;
        fs::create_dir_all(self.config_path())?;
        fs::create_dir_all(self.static_servers_path())?;
        fs::create_dir_all(self.running_path())?;

        Ok(())
    }
}
