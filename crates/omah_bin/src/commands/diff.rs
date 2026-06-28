use std::path::Path;

use anyhow::Result;
use omah_lib::{
    config::load_toml_config,
    ops::{diff, ChangeKind, FileChange},
};
use owo_colors::OwoColorize;
use tabled::{settings::Style, Table, Tabled};

#[derive(Tabled)]
struct DiffRow {
    #[tabled(rename = "File")]
    file: String,
    #[tabled(rename = "Change")]
    change: String,
}

fn change_cell(kind: &ChangeKind) -> String {
    match kind {
        ChangeKind::Added => format!("{}  {}", "+".green().bold(), "new in source".dimmed()),
        ChangeKind::Modified => format!("{}  {}", "~".yellow().bold(), "modified".dimmed()),
        ChangeKind::Removed => format!("{}  {}", "-".red().bold(), "only in vault".dimmed()),
    }
}

fn group_changes(changes: &[FileChange]) -> Vec<(String, Vec<FileChange>)> {
    let mut groups: Vec<(String, Vec<FileChange>)> = Vec::new();
    for c in changes {
        if groups.last().is_some_and(|(name, _)| name == &c.dot_name) {
            groups.last_mut().unwrap().1.push(c.clone());
            continue;
        }
        groups.push((c.dot_name.clone(), vec![c.clone()]));
    }
    groups
}

pub fn run(config_path: &Path, json: bool) -> Result<()> {
    let config = load_toml_config(config_path)?;
    let changes = diff(&config)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&changes)?);
        return Ok(());
    }

    if changes.is_empty() {
        println!("{}", "✓ All dotfiles are in sync with the vault.".green());
        return Ok(());
    }

    for (dot_name, entries) in &group_changes(&changes) {
        println!("{dot_name}");
        let mut table = Table::new(
            entries
                .iter()
                .map(|c| DiffRow {
                    file: c.path.clone(),
                    change: change_cell(&c.kind),
                })
                .collect::<Vec<_>>(),
        );
        table.with(Style::rounded());
        println!("{table}\n");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use omah_lib::ops::{ChangeKind, FileChange};

    #[test]
    fn test_change_cell_added() {
        let cell = change_cell(&ChangeKind::Added);
        assert!(cell.contains("+"));
        assert!(cell.contains("new in source"));
    }

    #[test]
    fn test_change_cell_modified() {
        let cell = change_cell(&ChangeKind::Modified);
        assert!(cell.contains("~"));
        assert!(cell.contains("modified"));
    }

    #[test]
    fn test_change_cell_removed() {
        let cell = change_cell(&ChangeKind::Removed);
        assert!(cell.contains("-"));
        assert!(cell.contains("only in vault"));
    }

    #[test]
    fn test_group_changes_single() {
        let changes = vec![FileChange {
            dot_name: "Zsh".into(),
            path: ".zshrc".into(),
            kind: ChangeKind::Modified,
        }];
        let groups = group_changes(&changes);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0, "Zsh");
        assert_eq!(groups[0].1.len(), 1);
    }

    #[test]
    fn test_group_changes_multiple_dots() {
        let changes = vec![
            FileChange { dot_name: "Zsh".into(), path: ".zshrc".into(), kind: ChangeKind::Modified },
            FileChange { dot_name: "Zsh".into(), path: ".zshenv".into(), kind: ChangeKind::Added },
            FileChange { dot_name: "Nvim".into(), path: "init.lua".into(), kind: ChangeKind::Removed },
        ];
        let groups = group_changes(&changes);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "Zsh");
        assert_eq!(groups[0].1.len(), 2);
        assert_eq!(groups[1].0, "Nvim");
        assert_eq!(groups[1].1.len(), 1);
    }

    #[test]
    fn test_group_changes_empty() {
        let groups = group_changes(&[]);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_diff_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("config.toml");
        std::fs::write(&cfg, r#"vault_path = "/tmp/vault""#).unwrap();
        let result = run(&cfg, true);
        assert!(result.is_ok());
    }
}
