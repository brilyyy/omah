use std::path::Path;

use anyhow::Result;
use omah_lib::{config::load_toml_config, ops::status};
use owo_colors::OwoColorize;

enum DeployState {
    Deployed,
    DeployedSymlink,
    Available,
    Unbacked,
    Unresolvable,
}

impl DeployState {
    fn from(s: &omah_lib::ops::DotStatus) -> Self {
        match (s.source_exists, s.backed_up, s.symlinked) {
            (true, true, true) => Self::DeployedSymlink,
            (true, true, false) => Self::Deployed,
            (false, true, _) => Self::Available,
            (true, false, _) => Self::Unbacked,
            (false, false, _) => Self::Unresolvable,
        }
    }

    fn label(&self) -> String {
        match self {
            Self::Deployed => "✓ deployed".green().to_string(),
            Self::DeployedSymlink => "🔗 deployed".blue().to_string(),
            Self::Available => "○ available".cyan().to_string(),
            Self::Unbacked => "⚠ unbacked".yellow().to_string(),
            Self::Unresolvable => "✗ missing".red().to_string(),
        }
    }
}

pub fn run(config_path: &Path, json: bool) -> Result<()> {
    let config = load_toml_config(config_path)?;
    let statuses = status(&config)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&statuses)?);
        return Ok(());
    }

    println!("Vault: {}\n", config.vault_path);

    if statuses.is_empty() {
        println!("{}", "No dotfiles configured.".dimmed());
        return Ok(());
    }

    // Dynamic column widths
    let name_w = statuses.iter().map(|s| s.name.len()).max().unwrap_or(0).max(4) + 2;
    let src_w = statuses.iter().map(|s| s.source.len()).max().unwrap_or(0).max(6) + 2;

    for s in &statuses {
        let state = DeployState::from(s);
        let label = state.label();
        let extra = if !s.source_exists && s.backed_up {
            format!("  {}", "[restore to deploy]".dimmed())
        } else {
            String::new()
        };

        println!(
            "  {:<name_w$}  {:<src_w$}  {}{}",
            s.name, s.source, label, extra,
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

    // Summary
    let total = statuses.len();
    let deployed = statuses.iter().filter(|s| s.source_exists && s.backed_up).count();
    let available = statuses.iter().filter(|s| !s.source_exists && s.backed_up).count();
    let missing = statuses.iter().filter(|s| !s.source_exists && !s.backed_up).count();
    let unbacked = statuses.iter().filter(|s| s.source_exists && !s.backed_up).count();
    let issues = statuses
        .iter()
        .filter(|s| !s.missing_deps.is_empty() || !s.pending_setup.is_empty())
        .count();

    println!();
    print!("{} dotfile{}", total, if total == 1 { "" } else { "s" });
    print!(" · {} deployed", deployed.to_string().green());
    if available > 0 {
        print!(" · {} available", available.to_string().cyan());
    }
    if unbacked > 0 {
        print!(" · {} unbacked", unbacked.to_string().yellow());
    }
    if missing > 0 {
        print!(" · {} missing", missing.to_string().red());
    }
    if issues > 0 {
        print!(" · {} with issues", issues.to_string().yellow());
    }
    println!();

    Ok(())
}
