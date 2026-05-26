use std::path::Path;

use anyhow::Result;
use omah_lib::{config::load_toml_config, ops::status};
use owo_colors::OwoColorize;

pub fn run(config_path: &Path) -> Result<()> {
    let config = load_toml_config(config_path)?;
    let statuses = status(&config)?;

    println!("Vault: {}\n", config.vault_path);

    if statuses.is_empty() {
        println!("{}", "No dotfiles configured.".dimmed());
        return Ok(());
    }

    // Dynamic column widths
    let name_w = statuses.iter().map(|s| s.name.len()).max().unwrap_or(0).max(4) + 2;
    let src_w = statuses.iter().map(|s| s.source.len()).max().unwrap_or(0).max(6) + 2;

    for s in &statuses {
        let status_label = if s.backed_up {
            "backed up".green().to_string()
        } else {
            "NOT backed up".red().to_string()
        };

        let extra = if s.symlinked {
            format!("  {}", "[symlinked]".blue())
        } else if !s.source_exists {
            format!("  {}", "[source missing]".dimmed())
        } else {
            String::new()
        };

        println!(
            "  {:<name_w$}  {:<src_w$}  {}{}",
            s.name, s.source, status_label, extra,
            name_w = name_w,
            src_w = src_w,
        );

        let indent = " ".repeat(name_w + 4);

        if !s.missing_deps.is_empty() {
            println!(
                "{}{}  {}",
                indent,
                "missing deps:".yellow(),
                s.missing_deps.join(", ").yellow()
            );
        }
        for cmd in &s.pending_setup {
            println!(
                "{}{}  {}",
                indent,
                "pending setup:".yellow(),
                cmd.yellow()
            );
        }
    }

    // Summary line
    let total = statuses.len();
    let backed = statuses.iter().filter(|s| s.backed_up).count();
    let not_backed = total - backed;
    let issues = statuses
        .iter()
        .filter(|s| !s.missing_deps.is_empty() || !s.pending_setup.is_empty())
        .count();

    println!();
    print!("{} dotfile{}", total, if total == 1 { "" } else { "s" });
    print!(" · {} backed up", backed.to_string().green());
    if not_backed > 0 {
        print!(" · {} not backed up", not_backed.to_string().red());
    }
    if issues > 0 {
        print!(" · {} with issues", issues.to_string().yellow());
    }
    println!();

    Ok(())
}
