use std::{
    fs,
    io::{Error, ErrorKind, Write},
    path::Path,
};

/// Only accept portable, single path components for user-controlled identifiers.
pub fn validate_name(name: &str) -> std::io::Result<()> {
    let reserved = name.split('.').next().unwrap_or_default().to_ascii_uppercase();
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with('.')
        || !name.bytes().all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
        || matches!(reserved.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (reserved.len() == 4 && (reserved.starts_with("COM") || reserved.starts_with("LPT")) && matches!(reserved.as_bytes()[3], b'1'..=b'9'))
    {
        return Err(Error::new(ErrorKind::InvalidInput, format!("Invalid name: '{name}'")));
    }
    Ok(())
}

/// Persist a complete file before replacing the previous version on the same filesystem.
pub fn atomic_write(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(contents)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

/// A reversible directory removal. Dropping before commit restores the original path.
pub(crate) struct StagedDirectory {
    original: std::path::PathBuf,
    staged: Option<tempfile::TempDir>,
    committed: bool,
    root: std::path::PathBuf,
}

impl StagedDirectory {
    pub(crate) fn new(root: &Path, path: &Path) -> std::io::Result<Self> {
        let root = root.canonicalize()?;
        let resolved = path.canonicalize()?;
        if resolved == root || !resolved.starts_with(&root) || fs::symlink_metadata(path)?.file_type().is_symlink() {
            return Err(Error::new(ErrorKind::InvalidInput, "Directory is outside the managed root"));
        }
        let staged = tempfile::Builder::new().prefix(".deleting-").tempdir_in(&root)?;
        atomic_write(
            &staged.path().join("original"),
            resolved.strip_prefix(&root).map_err(Error::other)?.to_string_lossy().as_bytes(),
        )?;
        fs::rename(path, staged.path().join("contents"))?;
        Ok(Self {
            original: path.to_path_buf(),
            staged: Some(staged),
            committed: false,
            root,
        })
    }

    pub(crate) fn backup_metadata(&self, paths: &[std::path::PathBuf]) -> std::io::Result<()> {
        let base = self.root.parent().ok_or_else(|| Error::other("Missing data root"))?;
        let mut files = Vec::new();
        for path in paths {
            let path = path.canonicalize()?;
            let relative = path.strip_prefix(base).map_err(Error::other)?.to_path_buf();
            files.push((relative, fs::read_to_string(path)?));
        }
        let metadata = RecoveryMetadata { files };
        atomic_write(
            &self.staged.as_ref().unwrap().path().join("metadata.toml"),
            toml::to_string(&metadata).map_err(Error::other)?.as_bytes(),
        )
    }

    pub(crate) fn commit(mut self) -> std::io::Result<()> {
        atomic_write(&self.staged.as_ref().unwrap().path().join("committed"), b"committed")?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for StagedDirectory {
    fn drop(&mut self) {
        if !self.committed
            && let Some(staged) = self.staged.take()
        {
            let result = fs::rename(staged.path().join("contents"), &self.original).and_then(|_| restore_metadata(staged.path(), &self.root));
            if let Err(error) = result {
                let preserved = staged.keep();
                eprintln!(
                    "Cannot restore {}: {error}; files preserved at {}",
                    self.original.display(),
                    preserved.display()
                );
            }
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RecoveryMetadata {
    files: Vec<(std::path::PathBuf, String)>,
}

fn restore_metadata(staged: &Path, root: &Path) -> std::io::Result<()> {
    let path = staged.join("metadata.toml");
    if !path.exists() {
        return Ok(());
    }
    let metadata: RecoveryMetadata = toml::from_str(&fs::read_to_string(path)?).map_err(Error::other)?;
    let base = root.parent().ok_or_else(|| Error::other("Missing recovery root"))?;
    for (relative, contents) in metadata.files {
        if relative.components().any(|part| !matches!(part, std::path::Component::Normal(_))) {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid metadata recovery path"));
        }
        let path = base.join(relative);
        if !path.parent().unwrap().canonicalize()?.starts_with(base) {
            return Err(Error::new(ErrorKind::InvalidData, "Metadata recovery escapes root"));
        }
        atomic_write(&path, contents.as_bytes())?;
    }
    Ok(())
}

pub(crate) fn recover_staged_directories(root: &Path) -> std::io::Result<()> {
    if !root.exists() {
        return Ok(());
    }
    let root = root.canonicalize()?;
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let staged = entry.path();
        if entry.file_name().to_string_lossy().starts_with(".preparing-") && staged.join(".destination").is_file() {
            let name = fs::read_to_string(staged.join(".destination"))?;
            validate_name(&name)?;
            let destination = root.join(name);
            if destination.exists() {
                return Err(Error::new(ErrorKind::AlreadyExists, "Prepared destination already exists"));
            }
            fs::rename(staged, &destination)?;
            fs::remove_file(destination.join(".destination"))?;
            continue;
        }
        if !entry.file_name().to_string_lossy().starts_with(".deleting-") {
            continue;
        }
        if staged.join("committed").is_file() {
            fs::remove_dir_all(&staged)?;
            continue;
        }
        let relative = fs::read_to_string(staged.join("original"))?;
        let relative = Path::new(&relative);
        if relative.components().any(|part| !matches!(part, std::path::Component::Normal(_))) {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid recovery path"));
        }
        let original = root.join(relative);
        let parent = original.parent().unwrap().canonicalize()?;
        if !parent.starts_with(&root) {
            return Err(Error::new(ErrorKind::InvalidData, "Recovery path escapes root"));
        }
        if staged.join("contents").exists() {
            if original.exists() {
                return Err(Error::new(ErrorKind::AlreadyExists, "Recovery destination already exists"));
            }
            fs::rename(staged.join("contents"), original)?;
        }
        restore_metadata(&staged, &root)?;
        fs::remove_dir_all(staged)?;
    }
    Ok(())
}

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    if !src.as_ref().is_dir() {
        return Err(Error::new(ErrorKind::NotFound, "Template directory does not exist"));
    }
    fs::create_dir_all(&dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;

        if file_type.is_symlink() {
            return Err(Error::new(ErrorKind::InvalidInput, "Linked template contents are not supported"));
        }

        if file_type.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }

    Ok(())
}

pub fn set_velocity_property(path: &Path, key: &str, value: toml::Value) -> Result<(), Error> {
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    let mut config: toml::Table = toml::from_str(&content).map_err(Error::other)?;
    config.insert(key.to_string(), value);
    return atomic_write(path, toml::to_string_pretty(&config).map_err(Error::other)?.as_bytes());
}

pub fn clear_velocity_servers(path: &Path) -> Result<(), Error> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };

    let mut config: toml::Table = toml::from_str(&content).map_err(Error::other)?;

    let servers = config
        .entry("servers")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "[servers] is not a table"))?;

    servers.clear();
    servers.insert("try".to_string(), toml::Value::Array(vec![]));
    config.remove("try");
    config.insert("forced-hosts".to_string(), toml::Value::Table(toml::Table::new()));

    atomic_write(path, toml::to_string_pretty(&config).map_err(Error::other)?.as_bytes())
}
pub async fn set_paper_property(paper_yml_path: &Path, entry: &str, value: serde_yml::Value) -> Result<(), Error> {
    if !paper_yml_path.exists() || paper_yml_path.file_name().unwrap() != "paper-global.yml" {
        return Err(Error::new(ErrorKind::NotFound, "Path is not a paper-global.yml file"));
    }

    let content = tokio::fs::read_to_string(&paper_yml_path).await?;
    let mut config: serde_yml::Value = serde_yml::from_str(&content).map_err(Error::other)?;

    let entries = entry.split('.');
    let mut current = &mut config;

    for key in entries {
        current = &mut current[key];
    }

    *current = value;

    let content = serde_yml::to_string(&config).map_err(Error::other)?;
    atomic_write(&paper_yml_path, content.as_bytes())?;
    Ok(())
}

pub fn set_server_property(properties_path: &Path, key: &str, value: &str) -> Result<(), Error> {
    if !properties_path.exists() || properties_path.file_name().unwrap() != "server.properties" {
        return Err(Error::new(ErrorKind::NotFound, "Path is not a server.properties file"));
    }

    let content = fs::read_to_string(properties_path)?;
    let target = format!("{key}=");

    let mut found = false;

    let mut lines = content
        .lines()
        .map(|line| {
            if line.starts_with(&target) {
                found = true;
                format!("{key}={value}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>();
    if !found {
        lines.push(format!("{key}={value}"));
    }
    atomic_write(properties_path, lines.join("\n").as_bytes())?;
    Ok(())
}
