use std::path::Path;

use anyhow::Result;
use expand_tilde::ExpandTilde;
use omah_lib::{
    config::load_toml_config,
    deps::{is_installed, pending_setup_steps},
    ops::status,
};
use owo_colors::OwoColorize;

fn vault_entry_size(path: &Path) -> (u64, u64) {
    let mut files = 0u64;
    let mut bytes = 0u64;
    if path.is_file() {
        files = 1;
        bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    } else if path.is_dir() {
        walk_dir(path, &mut files, &mut bytes);
    }
    (files, bytes)
}

fn walk_dir(dir: &Path, files: &mut u64, bytes: &mut u64) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_dir(&path, files, bytes);
            } else if path.is_file() {
                *files += 1;
                if let Ok(meta) = path.metadata() {
                    *bytes += meta.len();
                }
            }
        }
    }
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size > 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{size} {}", UNITS[unit])
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

fn vault_entry_for(dot_name: &str, source: &str, vault_path: &str) -> Option<std::path::PathBuf> {
    let vault = vault_path.expand_tilde().ok()?;
    let filename = source.rsplit('/').next().unwrap_or(source);
    Some(vault.join(dot_name).join(filename))
}

pub fn run(config_path: &Path, name: Option<&str>) -> Result<()> {
    let mut config = load_toml_config(config_path)?;

    if let Some(n) = name {
        config.dots.retain(|d| d.name == n);
        if config.dots.is_empty() {
            anyhow::bail!("Dotfile '{}' not found in config", n);
        }
    }

    let statuses = status(&config)?;

    for s in &statuses {
        let vault_entry = vault_entry_for(&s.name, &s.source, &config.vault_path);
        let (file_count, total_bytes) = match vault_entry {
            Some(ref p) if p.exists() => vault_entry_size(p),
            _ => (0, 0),
        };

        let state = if s.symlinked {
            "✓ deployed (symlink)".green().to_string()
        } else if s.source_exists && s.backed_up {
            "✓ deployed".green().to_string()
        } else if s.backed_up {
            "○ available".cyan().to_string()
        } else if s.source_exists {
            "⚠ unbacked".yellow().to_string()
        } else {
            "✗ missing".red().to_string()
        };

        let vault_display = format!("{}/{}", config.vault_path, s.name);

        println!("{}", s.name.bold());
        println!("  source:   {}", s.source);
        println!("  vault:    {}", vault_display);
        println!("  state:    {}", state);

        if file_count > 0 {
            println!("  files:    {} ({})", file_count, format_size(total_bytes));
        }

        if let Some(dot) = config.dots.iter().find(|d| d.name == s.name) {
            if let Some(ref deps) = dot.deps
                && !deps.is_empty()
            {
                let dep_list: Vec<String> = deps
                    .iter()
                    .map(|d| {
                        if is_installed(d) {
                            format!("{} (installed)", d.green())
                        } else {
                            format!("{} (missing)", d.red())
                        }
                    })
                    .collect();
                println!("  deps:     {}", dep_list.join(" · "));
            }

            if let Some(ref setup_steps) = dot.setup
                && !setup_steps.is_empty()
            {
                let pending = pending_setup_steps(dot);
                let steps: Vec<String> = setup_steps
                    .iter()
                    .map(|step| {
                        let is_pending = pending.iter().any(|p| p.install == step.install);
                        if is_pending {
                            format!("{} (pending)", step.install.yellow())
                        } else {
                            format!("{} (done)", step.install.dimmed())
                        }
                    })
                    .collect();
                println!("  setup:    {}", steps.join("\n            "));
            }

            if let Some(ref exclude) = dot.exclude
                && !exclude.is_empty()
            {
                println!("  exclude:  {}", exclude.join(" · "));
            }
        }

        println!();
    }

    Ok(())
}
