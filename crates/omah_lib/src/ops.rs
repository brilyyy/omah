use anyhow::{Context, Result};
use expand_tilde::ExpandTilde;
use omah_structs::OmahConfig;
use std::{
    fs,
    path::{Path, PathBuf},
};

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

fn expand_path(path: &str) -> Result<PathBuf> {
    path.expand_tilde()
        .map(|p| p.to_path_buf())
        .with_context(|| format!("Failed to expand path: {}", path))
}

fn copy_recursive(src: &Path, dst: &Path) -> Result<()> {
    if src.is_file() {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
    } else if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &dst.join(entry.file_name()))?;
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

pub fn backup(config: &OmahConfig) -> Result<()> {
    let vault = expand_path(&config.vault_path)?;
    fs::create_dir_all(&vault)?;

    for dot in &config.dots {
        let source = expand_path(&dot.source)?;
        let filename = source
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Source has no filename: {}", source.display()))?;
        let dest = vault.join(&dot.name).join(filename);

        copy_recursive(&source, &dest).with_context(|| {
            format!("Failed to backup '{}' from {}", dot.name, source.display())
        })?;

        if dot.symlink.unwrap_or(false) {
            remove_path(&source)
                .with_context(|| format!("Failed to remove source for '{}'", dot.name))?;
            std::os::unix::fs::symlink(&dest, &source)
                .with_context(|| format!("Failed to create symlink for '{}'", dot.name))?;
        }
    }

    Ok(())
}

pub fn restore(config: &OmahConfig) -> Result<()> {
    let vault = expand_path(&config.vault_path)?;

    for dot in &config.dots {
        let source = expand_path(&dot.source)?;
        let filename = source
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Source has no filename: {}", source.display()))?;
        let vault_entry = vault.join(&dot.name).join(filename);

        if !vault_entry.exists() {
            anyhow::bail!(
                "Vault entry for '{}' not found at {}",
                dot.name,
                vault_entry.display()
            );
        }

        if dot.symlink.unwrap_or(false) {
            remove_path(&source)
                .with_context(|| format!("Failed to remove existing source for '{}'", dot.name))?;
            if let Some(parent) = source.parent() {
                fs::create_dir_all(parent)?;
            }
            std::os::unix::fs::symlink(&vault_entry, &source)
                .with_context(|| format!("Failed to create symlink for '{}'", dot.name))?;
        } else {
            copy_recursive(&vault_entry, &source).with_context(|| {
                format!("Failed to restore '{}' to {}", dot.name, source.display())
            })?;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use omah_structs::DotfileConfig;
    use tempfile::tempdir;

    fn make_config(vault: &str, dots: Vec<DotfileConfig>) -> OmahConfig {
        OmahConfig {
            vault_path: vault.to_string(),
            dots,
        }
    }

    fn dot(name: &str, source: &str, symlink: Option<bool>) -> DotfileConfig {
        DotfileConfig {
            name: name.to_string(),
            source: source.to_string(),
            symlink,
            deps: None,
            setup: None,
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
        assert_eq!(
            fs::read_to_string(&vault_entry).unwrap(),
            "export PATH=~/bin:$PATH"
        );
        // source should still exist as a regular file
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
        // source should now be a symlink pointing at the vault entry
        assert!(source.is_symlink());
        assert_eq!(fs::read_link(&source).unwrap(), vault_entry);
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
}
