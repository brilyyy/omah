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
        let default_config = concat!(
            "#:schema https://raw.githubusercontent.com/brilyyy/omah/main/docs/schemas/omah-config.schema.json\n",
            "# Panggonan kanggo nyimpen backup (The Vault)\n",
            "vault_path = \"~/.config/omah/vault\"\n",
            "\n",
            "# [[dots]]\n",
            "# name = \"Example\"\n",
            "# source = \"~/.zshrc\"\n",
            "# symlink = false\n",
        );
        std::fs::write(config_path, default_config)
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
