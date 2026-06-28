use std::path::Path;

use anyhow::Result;
use expand_tilde::ExpandTilde;
use omah_lib::{config::load_toml_config, ops::ensure_ids};

pub fn run(config_path: &Path) -> Result<()> {
    let mut config = load_toml_config(config_path)?;

    let legacy_count = config.dots.iter().filter(|d| d.id.is_none()).count();

    if legacy_count == 0 {
        println!("All dotfiles already have IDs. Nothing to migrate.");
        return Ok(());
    }

    println!("Migrating {} dotfile(s) to ID-based vault structure...\n", legacy_count);

    // Record legacy vault paths before migration (for rename reporting)
    let vault = config
        .vault_path
        .expand_tilde()
        .map(|p| p.to_path_buf())
        .map_err(|_| anyhow::anyhow!("Failed to expand vault path"))?;

    let legacy_names: Vec<String> = config
        .dots
        .iter()
        .filter(|d| d.id.is_none())
        .map(|d| d.name.clone())
        .collect();

    let legacy_paths: Vec<(String, std::path::PathBuf)> = legacy_names
        .iter()
        .map(|name| (name.clone(), vault.join(name)))
        .collect();

    ensure_ids(&mut config, config_path)?;

    // Reload to get the assigned IDs
    let config = load_toml_config(config_path)?;

    for dot in &config.dots {
        if let Some(id) = &dot.id {
            let was_renamed = legacy_paths
                .iter()
                .any(|(name, old_path)| name == &dot.name && !old_path.exists());
            if was_renamed {
                println!(
                    "  {} → assigned ID {}, renamed vault dir",
                    dot.name, id
                );
            } else {
                println!("  {} → assigned ID {}", dot.name, id);
            }
        }
    }

    println!("\nMigration complete. Config saved to {}.", config_path.display());
    Ok(())
}
