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

    // Warn before replacing sources with symlinks — but only for dots where the
    // source is NOT already a symlink pointing at the vault entry (those are
    // silently skipped by backup() anyway, no need to alarm the user).
    let vault = config
        .vault_path
        .expand_tilde()
        .map(|p| p.to_path_buf())
        .map_err(|_| anyhow::anyhow!("Failed to expand vault path"))?;

    let symlink_dots: Vec<&str> = config
        .dots
        .iter()
        .filter(|d| {
            if !d.symlink.unwrap_or(false) {
                return false;
            }
            // Skip dots already correctly symlinked — backup() will no-op them.
            let Ok(source) = d.source.expand_tilde().map(|p| p.to_path_buf()) else {
                return true;
            };
            let Ok(filename) = source.file_name().ok_or(()) else {
                return true;
            };
            let dest = vault.join(&d.name).join(filename);
            let already = source.is_symlink()
                && std::fs::read_link(&source)
                    .map(|t| t == dest)
                    .unwrap_or(false);
            !already
        })
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
