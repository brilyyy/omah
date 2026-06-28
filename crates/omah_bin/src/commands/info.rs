use std::path::Path;

use anyhow::Result;
use expand_tilde::ExpandTilde;
use omah_lib::{
    config::load_toml_config,
    deps::{is_installed, pending_setup_steps},
    ops::status,
};
use owo_colors::OwoColorize;
use tabled::{settings::Style, Table, Tabled};

#[derive(Tabled)]
struct InfoRow {
    #[tabled(rename = "Field")]
    field: String,
    #[tabled(rename = "Value")]
    value: String,
}

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
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{size} {}", UNITS[unit])
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

fn vault_entry_for(dot_name: &str, source: &str, vault_path: &str, id: Option<&str>) -> Option<std::path::PathBuf> {
    let vault = vault_path.expand_tilde().ok()?;
    let dir_name = match id {
        Some(id) => format!("{}_{}", id, dot_name),
        None => dot_name.to_string(),
    };
    let filename = source.rsplit('/').next().unwrap_or(source);
    Some(vault.join(dir_name).join(filename))
}

fn build_info_rows(s: &omah_lib::ops::DotStatus, config: &omah_lib::OmahConfig) -> Vec<InfoRow> {
    let dot_id = config.dots.iter().find(|d| d.name == s.name).and_then(|d| d.id.as_deref());
    let vault_entry = vault_entry_for(&s.name, &s.source, &config.vault_path, dot_id);
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

    let vault_display = match dot_id {
        Some(id) => format!("{}/{}_{}", config.vault_path, id, s.name),
        None => format!("{}/{}", config.vault_path, s.name),
    };

    let mut rows = vec![
        InfoRow { field: "source".into(), value: s.source.clone() },
        InfoRow { field: "vault".into(), value: vault_display },
        InfoRow { field: "state".into(), value: state },
    ];

    if file_count > 0 {
        rows.push(InfoRow {
            field: "files".into(),
            value: format!("{} ({})", file_count, format_size(total_bytes)),
        });
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
            rows.push(InfoRow {
                field: "deps".into(),
                value: dep_list.join(" · "),
            });
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
            rows.push(InfoRow {
                field: "setup".into(),
                value: steps.join(", "),
            });
        }

        if let Some(ref exclude) = dot.exclude
            && !exclude.is_empty()
        {
            rows.push(InfoRow {
                field: "exclude".into(),
                value: exclude.join(" · "),
            });
        }
    }

    rows
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
        println!("{}", s.name.bold());
        let mut table = Table::new(build_info_rows(s, &config));
        table.with(Style::rounded());
        println!("{table}\n");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use omah_lib::ops::DotStatus;

    fn make_status(name: &str, source: &str) -> DotStatus {
        DotStatus {
            name: name.to_string(),
            source: source.to_string(),
            source_exists: true,
            backed_up: true,
            symlinked: false,
            missing_deps: vec![],
            pending_setup: vec![],
        }
    }

    #[test]
    fn test_build_info_rows_basic() {
        let config = omah_lib::OmahConfig {
            vault_path: "/vault".into(),
            dots: vec![],
            os: None,
            pkg_manager: None,
        };
        let s = make_status("Zsh", "/home/.zshrc");
        let rows = build_info_rows(&s, &config);
        assert!(rows.iter().any(|r| r.field == "source" && r.value == "/home/.zshrc"));
        assert!(rows.iter().any(|r| r.field == "state"));
        // files not present since vault doesn't exist
        assert!(!rows.iter().any(|r| r.field == "files"));
    }

    #[test]
    fn test_build_info_rows_with_files() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("Zsh");
        std::fs::create_dir_all(&vault).unwrap();
        std::fs::write(vault.join(".zshrc"), "export PATH").unwrap();

        let config = omah_lib::OmahConfig {
            vault_path: dir.path().to_str().unwrap().to_string(),
            dots: vec![],
            os: None,
            pkg_manager: None,
        };
        let s = DotStatus {
            name: "Zsh".into(),
            source: dir.path().join(".zshrc").to_string_lossy().to_string(),
            source_exists: true,
            backed_up: true,
            symlinked: false,
            missing_deps: vec![],
            pending_setup: vec![],
        };
        let rows = build_info_rows(&s, &config);
        let files_row = rows.iter().find(|r| r.field == "files");
        assert!(files_row.is_some());
        assert!(files_row.unwrap().value.contains("1 ("));
    }

    #[test]
    fn test_build_info_rows_state_labels() {
        let config = omah_lib::OmahConfig {
            vault_path: "/vault".into(),
            dots: vec![],
            os: None,
            pkg_manager: None,
        };

        let deployed = make_status("A", "/a");
        let rows = build_info_rows(&deployed, &config);
        assert!(rows.iter().any(|r| r.field == "state" && r.value.contains("✓")));

        let mut available = make_status("B", "/b");
        available.backed_up = true;
        available.source_exists = false;
        let rows = build_info_rows(&available, &config);
        assert!(rows.iter().any(|r| r.field == "state" && r.value.contains("○")));

        let mut unbacked = make_status("C", "/c");
        unbacked.backed_up = false;
        unbacked.source_exists = true;
        let rows = build_info_rows(&unbacked, &config);
        assert!(rows.iter().any(|r| r.field == "state" && r.value.contains("⚠")));

        let mut missing = make_status("D", "/d");
        missing.backed_up = false;
        missing.source_exists = false;
        let rows = build_info_rows(&missing, &config);
        assert!(rows.iter().any(|r| r.field == "state" && r.value.contains("✗")));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(2048), "2.0 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
    }

    #[test]
    fn test_vault_entry_for() {
        let path = vault_entry_for("Zsh", "/home/.zshrc", "/vault", Some("abc123"));
        assert_eq!(path, Some(std::path::PathBuf::from("/vault/abc123_Zsh/.zshrc")));
    }

    #[test]
    fn test_vault_entry_for_no_id() {
        let path = vault_entry_for("Zsh", "/home/.zshrc", "/vault", None);
        assert_eq!(path, Some(std::path::PathBuf::from("/vault/Zsh/.zshrc")));
    }
}
