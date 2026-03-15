use std::io::{self, Write};
use std::path::Path;

use anyhow::Result;
use expand_tilde::ExpandTilde;
use omah_lib::{config::load_toml_config, git::auto_commit_vault, ops::backup};

pub fn run(config_path: &Path, no_git: bool, no_exclude: bool) -> Result<()> {
    let mut config = load_toml_config(config_path)?;

    // Respect --no-exclude: clear all exclude patterns before backing up
    if no_exclude {
        for dot in &mut config.dots {
            dot.exclude = None;
        }
    }

    // Warn before replacing sources with symlinks
    let symlink_dots: Vec<&str> = config
        .dots
        .iter()
        .filter(|d| d.symlink.unwrap_or(false))
        .map(|d| d.name.as_str())
        .collect();

    if !symlink_dots.is_empty() {
        println!("The following dotfiles will have their source replaced with a symlink:");
        for name in &symlink_dots {
            println!("  - {name}");
        }
        print!("\nContinue? [y/N] ");
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        if !answer.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    backup(&config)?;
    println!("Backup complete → {}", config.vault_path);

    // Git auto-commit if enabled and not suppressed
    if !no_git && config.git.unwrap_or(false) {
        let vault = config
            .vault_path
            .expand_tilde()
            .map(|p| p.to_path_buf())
            .map_err(|_| anyhow::anyhow!("Failed to expand vault path"))?;

        // Copy config into vault so it's version-controlled alongside dotfiles
        let config_dest = vault.join(".omah-config.toml");
        if let Err(e) = std::fs::copy(config_path, &config_dest) {
            eprintln!("Warning: could not copy config to vault: {e}");
        }

        match auto_commit_vault(&vault) {
            Ok(()) => println!("git: vault committed"),
            Err(e) => eprintln!("Warning: git commit failed: {e}"),
        }
    }

    Ok(())
}
