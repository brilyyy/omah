use std::io::{self, IsTerminal, Write};
use std::path::Path;

use anyhow::Result;
use omah_lib::{config::load_toml_config, ops::status};
use owo_colors::OwoColorize;
use tabled::{settings::Style, Table, Tabled};

#[derive(Tabled)]
struct StatusRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Source")]
    source: String,
    #[tabled(rename = "State")]
    state: String,
}

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

fn state_label(s: &omah_lib::ops::DotStatus) -> String {
    let mut label = DeployState::from(s).label();
    if !s.source_exists && s.backed_up {
        label.push_str(&format!("  {}", "[restore to deploy]".dimmed()));
    }
    label
}

fn build_status_rows(statuses: &[omah_lib::ops::DotStatus]) -> Vec<StatusRow> {
    statuses
        .iter()
        .map(|s| StatusRow {
            name: s.name.clone(),
            source: s.source.clone(),
            state: state_label(s),
        })
        .collect()
}

pub fn run(config_path: &Path, json: bool) -> Result<()> {
    let config = load_toml_config(config_path)?;

    // Show spinner for each dot (TTY only)
    if io::stderr().is_terminal() {
        for dot in &config.dots {
            eprint!("\r  Checking {}... ", dot.name);
            io::stderr().flush().ok();
        }
        eprint!("\r");
        io::stderr().flush().ok();
    }

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

    let mut table = Table::new(build_status_rows(&statuses));
    table.with(Style::rounded());
    println!("{table}");

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

    // Footnotes for deps / setup issues
    for s in &statuses {
        if s.missing_deps.is_empty() && s.pending_setup.is_empty() {
            continue;
        }
        println!();
        if !s.missing_deps.is_empty() {
            println!(
                "  {}  {}",
                "missing deps:".yellow(),
                s.missing_deps.join(", ").yellow()
            );
        }
        for cmd in &s.pending_setup {
            println!(
                "  {}  {}",
                "pending setup:".yellow(),
                cmd.yellow()
            );
        }
    }

    // Check for upgrade
    if let Some(latest) = crate::upgrade_check::check_for_upgrade() {
        println!();
        println!(
            "  {}  {}  {}",
            "↑".cyan(),
            format!("New release {latest} available.").cyan(),
            "Run 'omah upgrade'".cyan()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use omah_lib::ops::DotStatus;

    fn make_status(name: &str, source: &str, exists: bool, backed: bool, sym: bool) -> DotStatus {
        DotStatus {
            name: name.to_string(),
            source: source.to_string(),
            source_exists: exists,
            backed_up: backed,
            symlinked: sym,
            missing_deps: vec![],
            pending_setup: vec![],
        }
    }

    #[test]
    fn test_build_status_rows_single() {
        let s = make_status("Zsh", "/home/.zshrc", true, true, false);
        let rows = build_status_rows(&[s]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Zsh");
        assert_eq!(rows[0].source, "/home/.zshrc");
        assert!(rows[0].state.contains("deployed"));
    }

    #[test]
    fn test_build_status_rows_multiple_states() {
        let statuses = vec![
            make_status("A", "/a", true, true, true),
            make_status("B", "/b", false, true, false),
            make_status("C", "/c", true, false, false),
            make_status("D", "/d", false, false, false),
        ];
        let rows = build_status_rows(&statuses);
        assert_eq!(rows.len(), 4);
        assert!(rows[0].state.contains("🔗"));
        assert!(rows[1].state.contains("○"));
        assert!(rows[1].state.contains("restore to deploy"));
        assert!(rows[2].state.contains("⚠"));
        assert!(rows[3].state.contains("✗"));
    }

    #[test]
    fn test_build_status_rows_with_deps() {
        let mut s = make_status("Nvim", "/nvim", true, true, false);
        s.missing_deps = vec!["git".to_string()];
        s.pending_setup = vec!["brew install lazygit".to_string()];
        let rows = build_status_rows(&[s]);
        assert_eq!(rows.len(), 1);
        // state stays clean; deps handled by footnotes
        assert!(!rows[0].state.contains("git"));
    }

    #[test]
    fn test_status_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("config.toml");
        std::fs::write(&cfg, r#"vault_path = "/tmp/vault""#).unwrap();
        // no dots — returns Ok(()), prints "No dotfiles configured."
        let result = run(&cfg, true);
        assert!(result.is_ok());
    }
}
