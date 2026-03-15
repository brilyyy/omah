use std::path::Path;

use anyhow::Result;
use omah_lib::{config::load_toml_config, ops::{diff, ChangeKind}};
use owo_colors::OwoColorize;

pub fn run(config_path: &Path) -> Result<()> {
    let config = load_toml_config(config_path)?;
    let changes = diff(&config)?;

    if changes.is_empty() {
        println!("{}", "✓ All dotfiles are in sync with the vault.".green());
        return Ok(());
    }

    // Group by dot name
    let mut last_dot = "";
    for c in &changes {
        if c.dot_name != last_dot {
            println!("\n{}", c.dot_name.bold());
            last_dot = &c.dot_name;
        }
        let (symbol, label) = match c.kind {
            ChangeKind::Added    => ("+".green().bold().to_string(),    "new in source"),
            ChangeKind::Modified => ("~".yellow().bold().to_string(),   "modified"),
            ChangeKind::Removed  => ("-".red().bold().to_string(),      "only in vault"),
        };
        println!("  {symbol}  {}  {}", c.path, label.dimmed());
    }
    println!();

    Ok(())
}
