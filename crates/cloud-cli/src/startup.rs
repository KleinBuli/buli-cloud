use std::{io::Error, path::Path};

use cloud_core::config::config_manager::ConfigManager;

use crate::setup::run_setup;

pub(crate) async fn ensure_config(cli_path: &Path) -> Result<(), Error> {
    let data_path = cli_path.parent().unwrap().parent().unwrap().join("data");

    let config_path = data_path.join("config/config.toml");
    if !config_path.exists() {
        let config = run_setup()?;
        let config_manager = ConfigManager::from(config_path.to_path_buf(), config).await?;
        config_manager.save_config().await?;
    }

    Ok(())
}
