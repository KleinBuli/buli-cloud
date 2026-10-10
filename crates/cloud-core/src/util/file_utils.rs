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
        || (reserved.len() == 4
            && (reserved.starts_with("COM") || reserved.starts_with("LPT"))
            && matches!(reserved.as_bytes()[3], b'1'..=b'9'))
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
}

impl StagedDirectory {
    pub(crate) fn new(root: &Path, path: &Path) -> std::io::Result<Self> {
        let root = root.canonicalize()?;
        let resolved = path.canonicalize()?;
        if resolved == root || !resolved.starts_with(&root) || fs::symlink_metadata(path)?.file_type().is_symlink() {
            return Err(Error::new(ErrorKind::InvalidInput, "Directory is outside the managed root"));
        }
        let staged = tempfile::Builder::new().prefix(".deleting-").tempdir_in(&root)?;
        fs::rename(path, staged.path().join("contents"))?;
        Ok(Self {
            original: path.to_path_buf(),
            staged: Some(staged),
            committed: false,
        })
    }

    pub(crate) fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for StagedDirectory {
    fn drop(&mut self) {
        if !self.committed
            && let Some(staged) = self.staged.take()
            && let Err(error) = fs::rename(staged.path().join("contents"), &self.original)
        {
            let preserved = staged.keep();
            eprintln!(
                "Cannot restore {}: {error}; files preserved at {}",
                self.original.display(),
                preserved.display()
            );
        }
    }
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
