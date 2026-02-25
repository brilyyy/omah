use std::path::Path;

use anyhow::Result;
use omah_lib::{config::load_toml_config, ops::backup};

pub fn run(config_path: &Path) -> Result<()> {
    let config = load_toml_config(config_path)?;
    backup(&config)?;
    println!("Backup complete → {}", config.vault_path);
    Ok(())
}
