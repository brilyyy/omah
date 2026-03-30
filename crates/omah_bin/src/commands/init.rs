use anyhow::Result;
use omah_lib::config::{get_default_config_path, init_setup};
use owo_colors::OwoColorize;

pub fn run() -> Result<()> {
    init_setup()?;
    let config_path = get_default_config_path()?;
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
