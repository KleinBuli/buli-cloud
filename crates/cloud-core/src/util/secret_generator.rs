use std::{
    io::Error,
    path::{Path, PathBuf},
};

use rand::TryRngCore;
use tokio::io::AsyncWriteExt;

pub fn generate_secret() -> Result<String, Error> {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.try_fill_bytes(&mut bytes).expect("failed to generate secret bytes");
    Ok(hex::encode(bytes))
}

pub async fn create_file(path: &Path, content: &[u8]) -> Result<(), Error> {
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options.open(path).await?;

    file.write_all(content).await?;
    file.sync_all().await?;

    Ok(())
}

pub async fn create_secret_file(path: &PathBuf) -> Result<(), Error> {
    let secret = generate_secret()?;
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options.open(path).await?;

    file.write_all(secret.as_bytes()).await?;
    file.sync_all().await?;

    Ok(())
}
