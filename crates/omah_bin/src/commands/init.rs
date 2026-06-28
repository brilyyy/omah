use std::path::Path;

use anyhow::{Context, Result};
use owo_colors::OwoColorize;

pub fn run(config_path: &Path) -> Result<()> {
    if let Some(parent) = config_path.parent()
        && !parent.is_dir()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    if !config_path.is_file() {
        std::fs::write(config_path, include_str!("../assets/config.template.toml"))
            .with_context(|| format!("Failed to write default config: {}", config_path.display()))?;
    }

    println!("Initialized: {}", config_path.display());
    println!();
    println!("{}", "Next steps:".bold());
    println!(
        "  {}  — add a dotfile entry",
        "omah add <name> <source>".cyan()
    );
    println!(
        "  {}        — back up all dotfiles to the vault",
        "omah backup".cyan()
    );
    println!(
        "  {}        — check sync state",
        "omah status".cyan()
    );
    Ok(())
}
