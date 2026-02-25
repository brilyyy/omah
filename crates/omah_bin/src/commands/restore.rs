use std::path::Path;

use anyhow::Result;
use omah_lib::{config::load_toml_config, ops::restore};

pub fn run(config_path: &Path) -> Result<()> {
    let config = load_toml_config(config_path)?;
    restore(&config)?;
    println!("Restore complete ← {}", config.vault_path);
    Ok(())
}
