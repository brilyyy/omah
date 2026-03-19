use anyhow::{Context, Result};
use expand_tilde::ExpandTilde;
use omah_structs::OmahConfig;
use std::{
    collections::HashSet,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

#[derive(serde::Serialize)]
pub struct DotStatus {
    pub name: String,
    pub source: String,
    pub source_exists: bool,
    pub backed_up: bool,
    /// Source is a symlink pointing at the vault entry.
    pub symlinked: bool,
    pub missing_deps: Vec<String>,
    /// Setup step install commands that are still pending.
    pub pending_setup: Vec<String>,
}

// ── Diff types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum ChangeKind {
    /// In source but not yet in vault — would be newly backed up.
    Added,
    /// Content differs between source and vault.
    Modified,
    /// In vault but no longer in source — orphaned backup.
    Removed,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileChange {
    pub dot_name: String,
    /// Path relative to the dotfile root (e.g. `init.lua` inside `~/.config/nvim`).
    pub path: String,
    pub kind: ChangeKind,
}

// ── Internal helpers ───────────────────────────────────────────────────────

fn expand_path(path: &str) -> Result<PathBuf> {
    path.expand_tilde()
        .map(|p| p.to_path_buf())
        .with_context(|| format!("Failed to expand path: {}", path))
}

/// Returns true if the entry's filename matches any glob pattern.
fn is_excluded(path: &Path, excludes: &[String]) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    excludes.iter().any(|pat| {
        glob::Pattern::new(pat)
            .map(|p| p.matches(name))
            .unwrap_or(false)
    })
}

fn copy_recursive(src: &Path, dst: &Path, excludes: &[String]) -> Result<()> {
    if src.is_file() {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
    } else if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            if is_excluded(&entry.path(), excludes) {
                continue;
            }
            copy_recursive(&entry.path(), &dst.join(entry.file_name()), excludes)?;
        }
    } else {
        anyhow::bail!("Source path does not exist: {}", src.display());
    }
    Ok(())
}

fn remove_path(path: &Path) -> Result<()> {
    if path.is_symlink() || path.is_file() {
        fs::remove_file(path)?;
    } else if path.is_dir() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

// ── Public operations ──────────────────────────────────────────────────────

pub fn backup(config: &OmahConfig) -> Result<()> {
    let vault = expand_path(&config.vault_path)?;
    fs::create_dir_all(&vault)?;

    for dot in &config.dots {
        let source = expand_path(&dot.source)?;
        let filename = source
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Source has no filename: {}", source.display()))?;
        let dest = vault.join(&dot.name).join(filename);
        let excludes = dot.exclude.as_deref().unwrap_or(&[]);

        // If source is already a symlink pointing at dest, skip the copy.
        // fs::copy follows the symlink, which means it opens dest for writing
        // (truncating it to 0 bytes) before reading source — which now reads
        // through the symlink from the just-emptied dest, silently zeroing the vault.
        let already_symlinked_to_dest = source.is_symlink()
            && fs::read_link(&source).map(|t| t == dest).unwrap_or(false);

        if !already_symlinked_to_dest {
            copy_recursive(&source, &dest, excludes).with_context(|| {
                format!("Failed to backup '{}' from {}", dot.name, source.display())
            })?;

            if dot.symlink.unwrap_or(false) {
                remove_path(&source)
                    .with_context(|| format!("Failed to remove source for '{}'", dot.name))?;
                std::os::unix::fs::symlink(&dest, &source)
                    .with_context(|| format!("Failed to create symlink for '{}'", dot.name))?;
            }
        }
    }

    if config.git.unwrap_or(false) {
        crate::git::auto_commit_vault(&vault).context("git auto-commit failed")?;
    }

    Ok(())
}

pub fn restore(config: &OmahConfig) -> Result<()> {
    let vault = expand_path(&config.vault_path)?;
    let mut errors: Vec<String> = Vec::new();

    for dot in &config.dots {
        let source = match expand_path(&dot.source) {
            Ok(p) => p,
            Err(e) => {
                errors.push(e.to_string());
                continue;
            }
        };
        let filename = match source.file_name() {
            Some(f) => f.to_owned(),
            None => {
                errors.push(format!("'{}': source has no filename", dot.name));
                continue;
            }
        };
        let vault_entry = vault.join(&dot.name).join(&filename);

        if !vault_entry.exists() {
            errors.push(format!(
                "'{}': vault entry not found at {}",
                dot.name,
                vault_entry.display()
            ));
            continue;
        }

        let result = if dot.symlink.unwrap_or(false) {
            (|| -> Result<()> {
                remove_path(&source).with_context(|| {
                    format!("Failed to remove existing source for '{}'", dot.name)
                })?;
                if let Some(parent) = source.parent() {
                    fs::create_dir_all(parent)?;
                }
                std::os::unix::fs::symlink(&vault_entry, &source)
                    .with_context(|| format!("Failed to create symlink for '{}'", dot.name))
            })()
        } else {
            copy_recursive(&vault_entry, &source, &[]).with_context(|| {
                format!("Failed to restore '{}' to {}", dot.name, source.display())
            })
        };

        if let Err(e) = result {
            errors.push(e.to_string());
        }
    }

    if !errors.is_empty() {
        anyhow::bail!("Restore completed with errors:\n  {}", errors.join("\n  "));
    }

    Ok(())
}

pub fn status(config: &OmahConfig) -> Result<Vec<DotStatus>> {
    let vault = expand_path(&config.vault_path)?;

    config
        .dots
        .iter()
        .map(|dot| {
            let source = expand_path(&dot.source)?;
            let filename = source
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("Source has no filename: {}", source.display()))?;
            let vault_entry = vault.join(&dot.name).join(filename);

            let source_exists = source.exists() || source.is_symlink();
            let backed_up = vault_entry.exists();
            let symlinked = source.is_symlink()
                && fs::read_link(&source)
                    .map(|target| target == vault_entry)
                    .unwrap_or(false);

            Ok(DotStatus {
                name: dot.name.clone(),
                source: dot.source.clone(),
                source_exists,
                backed_up,
                symlinked,
                missing_deps: crate::deps::missing_deps(dot),
                pending_setup: crate::deps::pending_setup_steps(dot)
                    .into_iter()
                    .map(|s| s.install.clone())
                    .collect(),
            })
        })
        .collect()
}

/// Compare each dotfile's source against its vault copy and return a list of differences.
pub fn diff(config: &OmahConfig) -> Result<Vec<FileChange>> {
    let vault = expand_path(&config.vault_path)?;
    let mut changes = Vec::new();

    for dot in &config.dots {
        let source = expand_path(&dot.source)?;
        let filename = source
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Source has no filename: {}", source.display()))?;
        let vault_entry = vault.join(&dot.name).join(filename);
        diff_trees(
            &dot.name,
            &source,
            &vault_entry,
            &filename.to_string_lossy(),
            &mut changes,
        )?;
    }

    Ok(changes)
}

fn diff_trees(
    dot_name: &str,
    source: &Path,
    vault: &Path,
    rel: &str,
    out: &mut Vec<FileChange>,
) -> Result<()> {
    let src_exists = source.exists() || source.is_symlink();
    let vlt_exists = vault.exists();

    match (src_exists, vlt_exists) {
        (true, false) => {
            if source.is_dir() {
                for entry in fs::read_dir(source)? {
                    let entry = entry?;
                    let name = entry.file_name();
                    let child = child_rel(rel, &name);
                    diff_trees(dot_name, &entry.path(), &vault.join(&name), &child, out)?;
                }
            } else {
                out.push(FileChange {
                    dot_name: dot_name.to_string(),
                    path: rel.to_string(),
                    kind: ChangeKind::Added,
                });
            }
        }
        (false, true) => {
            if vault.is_dir() {
                for entry in fs::read_dir(vault)? {
                    let entry = entry?;
                    let name = entry.file_name();
                    let child = child_rel(rel, &name);
                    diff_trees(dot_name, &source.join(&name), &entry.path(), &child, out)?;
                }
            } else {
                out.push(FileChange {
                    dot_name: dot_name.to_string(),
                    path: rel.to_string(),
                    kind: ChangeKind::Removed,
                });
            }
        }
        (true, true) => {
            if source.is_dir() || vault.is_dir() {
                let mut names: HashSet<OsString> = HashSet::new();
                if source.is_dir() {
                    for e in fs::read_dir(source)? {
                        names.insert(e?.file_name());
                    }
                }
                if vault.is_dir() {
                    for e in fs::read_dir(vault)? {
                        names.insert(e?.file_name());
                    }
                }
                for name in names {
                    let child = child_rel(rel, &name);
                    diff_trees(
                        dot_name,
                        &source.join(&name),
                        &vault.join(&name),
                        &child,
                        out,
                    )?;
                }
            } else if fs::read(source)? != fs::read(vault)? {
                out.push(FileChange {
                    dot_name: dot_name.to_string(),
                    path: rel.to_string(),
                    kind: ChangeKind::Modified,
                });
            }
        }
        (false, false) => {}
    }
    Ok(())
}

fn child_rel(parent: &str, name: &OsString) -> String {
    let name = name.to_string_lossy();
    if parent.is_empty() {
        name.into_owned()
    } else {
        format!("{parent}/{name}")
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use omah_structs::DotfileConfig;
    use tempfile::tempdir;

    fn make_config(vault: &str, dots: Vec<DotfileConfig>) -> OmahConfig {
        OmahConfig { vault_path: vault.to_string(), dots, git: None, os: None, pkg_manager: None }
    }

    fn dot(name: &str, source: &str, symlink: Option<bool>) -> DotfileConfig {
        DotfileConfig {
            name: name.to_string(),
            source: source.to_string(),
            symlink,
            deps: None,
            setup: None,
            exclude: None,
        }
    }

    fn dot_excl(name: &str, source: &str, pats: Vec<&str>) -> DotfileConfig {
        DotfileConfig {
            exclude: Some(pats.into_iter().map(String::from).collect()),
            ..dot(name, source, None)
        }
    }

    // ── backup ────────────────────────────────────────────────────────────────

    #[test]
    fn test_backup_file() {
        let src_dir = tempdir().unwrap();
        let vault_dir = tempdir().unwrap();
        let source = src_dir.path().join("zshrc");
        fs::write(&source, "export PATH=~/bin:$PATH").unwrap();

        let config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Zsh", source.to_str().unwrap(), None)],
        );
        backup(&config).unwrap();

        let vault_entry = vault_dir.path().join("Zsh").join("zshrc");
        assert!(vault_entry.is_file());
        assert_eq!(fs::read_to_string(&vault_entry).unwrap(), "export PATH=~/bin:$PATH");
        assert!(source.is_file());
        assert!(!source.is_symlink());
    }

    #[test]
    fn test_backup_directory() {
        let src_dir = tempdir().unwrap();
        let vault_dir = tempdir().unwrap();
        let nvim = src_dir.path().join("nvim");
        fs::create_dir(&nvim).unwrap();
        fs::write(nvim.join("init.lua"), "vim.opt.number = true").unwrap();

        let config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Nvim", nvim.to_str().unwrap(), None)],
        );
        backup(&config).unwrap();

        let vault_entry = vault_dir.path().join("Nvim").join("nvim");
        assert!(vault_entry.is_dir());
        assert!(vault_entry.join("init.lua").is_file());
    }

    #[test]
    fn test_backup_creates_vault_if_missing() {
        let src_dir = tempdir().unwrap();
        let vault_parent = tempdir().unwrap();
        let vault = vault_parent.path().join("new_vault");
        let source = src_dir.path().join("file.txt");
        fs::write(&source, "hello").unwrap();

        let config = make_config(
            vault.to_str().unwrap(),
            vec![dot("File", source.to_str().unwrap(), None)],
        );
        backup(&config).unwrap();

        assert!(vault.is_dir());
        assert!(vault.join("File").join("file.txt").is_file());
    }

    #[test]
    fn test_backup_with_symlink() {
        let src_dir = tempdir().unwrap();
        let vault_dir = tempdir().unwrap();
        let source = src_dir.path().join("zshrc");
        fs::write(&source, "# zsh config").unwrap();

        let config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Zsh", source.to_str().unwrap(), Some(true))],
        );
        backup(&config).unwrap();

        let vault_entry = vault_dir.path().join("Zsh").join("zshrc");
        assert!(vault_entry.is_file());
        assert!(source.is_symlink());
        assert_eq!(fs::read_link(&source).unwrap(), vault_entry);
    }

    #[test]
    fn test_backup_symlink_twice_preserves_vault_content() {
        // Regression: second backup when source is already a symlink to the vault
        // was zeroing out the vault file (fs::copy truncated dest before reading
        // source which pointed to the same file via the symlink).
        let src_dir = tempdir().unwrap();
        let vault_dir = tempdir().unwrap();
        let source = src_dir.path().join("zshrc");
        fs::write(&source, "# my zsh config").unwrap();

        let config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Zsh", source.to_str().unwrap(), Some(true))],
        );

        backup(&config).unwrap(); // first backup: copies file, creates symlink
        backup(&config).unwrap(); // second backup: must not zero out the vault

        let vault_entry = vault_dir.path().join("Zsh").join("zshrc");
        assert_eq!(fs::read_to_string(&vault_entry).unwrap(), "# my zsh config");
        assert!(source.is_symlink());
    }

    #[test]
    fn test_backup_missing_source_errors() {
        let vault_dir = tempdir().unwrap();
        let config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Missing", "/nonexistent/path/file.txt", None)],
        );
        assert!(backup(&config).is_err());
    }

    #[test]
    fn test_backup_exclude_patterns() {
        let src_dir = tempdir().unwrap();
        let vault_dir = tempdir().unwrap();
        let dir = src_dir.path().join("cfg");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("init.lua"), "config").unwrap();
        fs::write(dir.join("session.log"), "log data").unwrap();
        fs::create_dir(dir.join(".git")).unwrap();
        fs::write(dir.join(".git").join("HEAD"), "ref").unwrap();

        let config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot_excl("Cfg", dir.to_str().unwrap(), vec!["*.log", ".git"])],
        );
        backup(&config).unwrap();

        let vault = vault_dir.path().join("Cfg").join("cfg");
        assert!(vault.join("init.lua").is_file());
        assert!(!vault.join("session.log").exists());
        assert!(!vault.join(".git").exists());
    }

    // ── restore ───────────────────────────────────────────────────────────────

    #[test]
    fn test_restore_file() {
        let src_dir = tempdir().unwrap();
        let vault_dir = tempdir().unwrap();
        let vault_name_dir = vault_dir.path().join("Zsh");
        fs::create_dir_all(&vault_name_dir).unwrap();
        let vault_entry = vault_name_dir.join("zshrc");
        fs::write(&vault_entry, "# restored zsh").unwrap();
        let dest = src_dir.path().join("zshrc");

        let config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Zsh", dest.to_str().unwrap(), None)],
        );
        restore(&config).unwrap();

        assert!(dest.is_file());
        assert_eq!(fs::read_to_string(&dest).unwrap(), "# restored zsh");
    }

    #[test]
    fn test_restore_directory() {
        let src_dir = tempdir().unwrap();
        let vault_dir = tempdir().unwrap();
        let vault_name_dir = vault_dir.path().join("Nvim");
        let vault_entry = vault_name_dir.join("nvim");
        fs::create_dir_all(&vault_entry).unwrap();
        fs::write(vault_entry.join("init.lua"), "-- config").unwrap();
        let dest = src_dir.path().join("nvim");

        let config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Nvim", dest.to_str().unwrap(), None)],
        );
        restore(&config).unwrap();

        assert!(dest.is_dir());
        assert!(dest.join("init.lua").is_file());
    }

    #[test]
    fn test_restore_with_symlink() {
        let src_dir = tempdir().unwrap();
        let vault_dir = tempdir().unwrap();
        let vault_name_dir = vault_dir.path().join("Zsh");
        fs::create_dir_all(&vault_name_dir).unwrap();
        let vault_entry = vault_name_dir.join("zshrc");
        fs::write(&vault_entry, "# symlinked zsh").unwrap();
        let dest = src_dir.path().join("zshrc");

        let config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Zsh", dest.to_str().unwrap(), Some(true))],
        );
        restore(&config).unwrap();

        assert!(dest.is_symlink());
        assert_eq!(fs::read_link(&dest).unwrap(), vault_entry);
    }

    #[test]
    fn test_restore_missing_vault_entry_errors() {
        let src_dir = tempdir().unwrap();
        let vault_dir = tempdir().unwrap();
        let dest = src_dir.path().join("zshrc");

        let config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Zsh", dest.to_str().unwrap(), None)],
        );
        assert!(restore(&config).is_err());
    }

    // ── status ────────────────────────────────────────────────────────────────

    #[test]
    fn test_status_not_backed_up() {
        let src_dir = tempdir().unwrap();
        let vault_dir = tempdir().unwrap();
        let source = src_dir.path().join("zshrc");
        fs::write(&source, "# zsh").unwrap();

        let config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Zsh", source.to_str().unwrap(), None)],
        );
        let statuses = status(&config).unwrap();

        assert_eq!(statuses.len(), 1);
        assert!(statuses[0].source_exists);
        assert!(!statuses[0].backed_up);
        assert!(!statuses[0].symlinked);
    }

    #[test]
    fn test_status_backed_up() {
        let src_dir = tempdir().unwrap();
        let vault_dir = tempdir().unwrap();
        let source = src_dir.path().join("zshrc");
        fs::write(&source, "# zsh").unwrap();

        let config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Zsh", source.to_str().unwrap(), None)],
        );
        backup(&config).unwrap();
        let statuses = status(&config).unwrap();

        assert!(statuses[0].source_exists);
        assert!(statuses[0].backed_up);
        assert!(!statuses[0].symlinked);
    }

    #[test]
    fn test_status_symlinked() {
        let src_dir = tempdir().unwrap();
        let vault_dir = tempdir().unwrap();
        let source = src_dir.path().join("zshrc");
        fs::write(&source, "# zsh").unwrap();

        let config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Zsh", source.to_str().unwrap(), Some(true))],
        );
        backup(&config).unwrap();
        let statuses = status(&config).unwrap();

        assert!(statuses[0].source_exists);
        assert!(statuses[0].backed_up);
        assert!(statuses[0].symlinked);
    }

    #[test]
    fn test_status_source_missing() {
        let vault_dir = tempdir().unwrap();
        let config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Ghost", "/nonexistent/path/ghost", None)],
        );
        let statuses = status(&config).unwrap();

        assert!(!statuses[0].source_exists);
        assert!(!statuses[0].backed_up);
    }

    // ── diff ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_diff_no_vault_shows_added() {
        let src_dir = tempdir().unwrap();
        let vault_dir = tempdir().unwrap();
        let source = src_dir.path().join("zshrc");
        fs::write(&source, "# zsh").unwrap();

        let config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Zsh", source.to_str().unwrap(), None)],
        );
        let changes = diff(&config).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].kind, ChangeKind::Added);
    }

    #[test]
    fn test_diff_synced_shows_no_changes() {
        let src_dir = tempdir().unwrap();
        let vault_dir = tempdir().unwrap();
        let source = src_dir.path().join("zshrc");
        fs::write(&source, "# zsh").unwrap();

        let config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Zsh", source.to_str().unwrap(), None)],
        );
        backup(&config).unwrap();
        let changes = diff(&config).unwrap();
        assert!(changes.is_empty());
    }

    #[test]
    fn test_diff_modified_after_source_change() {
        let src_dir = tempdir().unwrap();
        let vault_dir = tempdir().unwrap();
        let source = src_dir.path().join("zshrc");
        fs::write(&source, "# zsh").unwrap();

        let config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Zsh", source.to_str().unwrap(), None)],
        );
        backup(&config).unwrap();
        fs::write(&source, "# zsh edited").unwrap();
        let changes = diff(&config).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].kind, ChangeKind::Modified);
    }
}
