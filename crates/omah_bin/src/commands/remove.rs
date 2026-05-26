use std::path::Path;

use anyhow::{bail, Result};
use omah_lib::config::{load_toml_config, save_toml_config};

pub fn run(config_path: &Path, name: &str) -> Result<()> {
    let mut config = load_toml_config(config_path)?;

    let before = config.dots.len();
    config.dots.retain(|d| d.name != name);

    if config.dots.len() == before {
        bail!("Dotfile '{}' not found in config", name);
    }

    save_toml_config(&config, config_path)?;
    println!("Removed '{}' from config.", name);
    println!("Source files and vault copy were not deleted.");
    Ok(())
}
