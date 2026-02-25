use std::path::Path;

use anyhow::Result;
use omah_lib::config::load_toml_config;

pub fn run(config_path: &Path) -> Result<()> {
    let config = load_toml_config(config_path)?;
    println!("Vault: {}\n", config.vault_path);
    for dot in &config.dots {
        let symlink_tag = if dot.symlink.unwrap_or(false) { " [symlink]" } else { "" };
        println!("  {}{}  →  {}", dot.name, symlink_tag, dot.source);
    }
    Ok(())
}
