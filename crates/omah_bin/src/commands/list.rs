use std::path::Path;

use anyhow::Result;
use omah_lib::config::load_toml_config;
use owo_colors::OwoColorize;
use tabled::{settings::Style, Table, Tabled};

#[derive(Tabled)]
struct ListRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Source")]
    source: String,
    #[tabled(rename = "Symlink")]
    symlink: String,
    #[tabled(rename = "Deps")]
    deps: String,
}

fn build_list_rows(config: &omah_lib::OmahConfig) -> Vec<ListRow> {
    config
        .dots
        .iter()
        .map(|dot| {
            let symlink = if dot.symlink.unwrap_or(false) {
                "yes".to_string()
            } else {
                "\u{2014}".to_string()
            };
            let deps = match &dot.deps {
                Some(d) if !d.is_empty() => d.join(", "),
                _ => "\u{2014}".to_string(),
            };
            ListRow {
                name: dot.name.clone(),
                source: dot.source.clone(),
                symlink,
                deps,
            }
        })
        .collect()
}

pub fn run(config_path: &Path, json: bool) -> Result<()> {
    let config = load_toml_config(config_path)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&config)?);
        return Ok(());
    }
    println!("Vault: {}\n", config.vault_path);

    if config.dots.is_empty() {
        println!("{}", "No dotfiles configured.".dimmed());
        return Ok(());
    }

    let mut table = Table::new(build_list_rows(&config));
    table.with(Style::rounded());
    println!("{table}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use omah_lib::DotfileConfig;

    fn make_dot(name: &str, source: &str) -> DotfileConfig {
        DotfileConfig {
            name: name.to_string(),
            source: source.to_string(),
            symlink: None,
            deps: None,
            setup: None,
            exclude: None,
        }
    }

    #[test]
    fn test_build_list_rows_plain() {
        let config = omah_lib::OmahConfig {
            vault_path: "/vault".into(),
            dots: vec![make_dot("Zsh", "/home/.zshrc")],
            os: None,
            pkg_manager: None,
        };
        let rows = build_list_rows(&config);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Zsh");
        assert_eq!(rows[0].source, "/home/.zshrc");
        assert_eq!(rows[0].symlink, "\u{2014}");
        assert_eq!(rows[0].deps, "\u{2014}");
    }

    #[test]
    fn test_build_list_rows_symlink_and_deps() {
        let mut dot = make_dot("Nvim", "/home/.config/nvim");
        dot.symlink = Some(true);
        dot.deps = Some(vec!["git".into(), "curl".into()]);
        let config = omah_lib::OmahConfig {
            vault_path: "/vault".into(),
            dots: vec![dot],
            os: None,
            pkg_manager: None,
        };
        let rows = build_list_rows(&config);
        assert_eq!(rows[0].symlink, "yes");
        assert_eq!(rows[0].deps, "git, curl");
    }

    #[test]
    fn test_build_list_rows_empty() {
        let rows = build_list_rows(&omah_lib::OmahConfig {
            vault_path: "/vault".into(),
            dots: vec![],
            os: None,
            pkg_manager: None,
        });
        assert!(rows.is_empty());
    }

    #[test]
    fn test_list_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("config.toml");
        std::fs::write(&cfg, r#"vault_path = "/tmp/vault""#).unwrap();
        let result = run(&cfg, true);
        assert!(result.is_ok());
    }
}
