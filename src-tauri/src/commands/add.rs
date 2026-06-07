use std::path::Path;

use anyhow::{bail, Result};
use omah_lib::{
    DotfileConfig,
    config::{load_toml_config, save_toml_config},
};

pub fn run(config_path: &Path, name: String, source: String, symlink: bool) -> Result<()> {
    let mut config = load_toml_config(config_path)?;

    if config.dots.iter().any(|d| d.name == name) {
        bail!("Dotfile '{}' already exists in config", name);
    }

    config.dots.push(DotfileConfig {
        name: name.clone(),
        source,
        symlink: if symlink { Some(true) } else { None },
        deps: None,
        setup: None,
        exclude: None,
    });

    save_toml_config(&config, config_path)?;
    println!("Added '{}' to config.", name);
    println!("Run `omah backup {}` to back it up to the vault.", name);
    Ok(())
}
