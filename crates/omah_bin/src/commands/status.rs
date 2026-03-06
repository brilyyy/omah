use std::path::Path;

use anyhow::Result;
use omah_lib::{config::load_toml_config, ops::status};

pub fn run(config_path: &Path) -> Result<()> {
    let config = load_toml_config(config_path)?;
    let statuses = status(&config)?;

    println!("Vault: {}\n", config.vault_path);

    for s in &statuses {
        let backed_up = if s.backed_up { "backed up" } else { "NOT backed up" };
        let extra = if s.symlinked {
            " [symlinked]"
        } else if !s.source_exists {
            " [source missing]"
        } else {
            ""
        };
        println!("  {:<20} {}  {}{}", s.name, s.source, backed_up, extra);
        if !s.missing_deps.is_empty() {
            println!("  {:<20} missing deps:  {}", "", s.missing_deps.join(", "));
        }
        for cmd in &s.pending_setup {
            println!("  {:<20} pending setup: {}", "", cmd);
        }
    }

    Ok(())
}
