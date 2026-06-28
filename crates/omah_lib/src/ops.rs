use anyhow::{Context, Result};
use expand_tilde::ExpandTilde;
use omah_structs::OmahConfig;
use std::{
    collections::HashSet,
    ffi::OsString,
    fs,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
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

/// Files always excluded regardless of user config (OS metadata noise).
const ALWAYS_EXCLUDE: &[&str] = &[".DS_Store"];

fn expand_path(path: &str) -> Result<PathBuf> {
    path.expand_tilde()
        .map(|p| p.to_path_buf())
        .with_context(|| format!("Failed to expand path: {}", path))
}

fn always_excluded(name: &OsString) -> bool {
    let s = name.to_string_lossy();
    ALWAYS_EXCLUDE.iter().any(|e| *e == s.as_ref())
}

/// Returns the vault directory for a dot using the `{id}_{name}` format.
/// Falls back to `{name}` if id is None (legacy configs).
pub fn vault_dir(vault: &Path, dot: &omah_structs::DotfileConfig) -> PathBuf {
    match &dot.id {
        Some(id) => vault.join(format!("{}_{}", id, dot.name)),
        None => vault.join(&dot.name),
    }
}

/// Returns true if the entry's filename matches any always-excluded name or glob pattern.
fn is_excluded(path: &Path, excludes: &[String]) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if ALWAYS_EXCLUDE.contains(&name) {
        return true;
    }
    excludes.iter().any(|pat| {
        glob::Pattern::new(pat)
            .map(|p| p.matches(name))
            .unwrap_or(false)
    })
}

fn count_files(path: &Path, excludes: &[String]) -> u64 {
    if path.is_file() {
        return 1;
    }
    if path.is_dir() {
        let mut count = 0;
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if is_excluded(&entry.path(), excludes) {
                    continue;
                }
                count += count_files(&entry.path(), excludes);
            }
        }
        return count;
    }
    0
}

fn copy_recursive(
    src: &Path,
    dst: &Path,
    excludes: &[String],
    progress: Option<(&AtomicU64, u64)>,
) -> Result<()> {
    if src.is_file() {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
        if let Some((counter, total)) = progress {
            let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
            if io::stderr().is_terminal() {
                let pct = (done as f64 / total as f64) * 100.0;
                eprint!("\r  {done}/{total} files ({pct:.0}%)");
                io::stderr().flush().ok();
            }
        }
    } else if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            if is_excluded(&entry.path(), excludes) {
                continue;
            }
            copy_recursive(
                &entry.path(),
                &dst.join(entry.file_name()),
                excludes,
                progress,
            )?;
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

// ── Progress helpers ───────────────────────────────────────────────────────

/// Print a dot-level progress header, e.g. `[1/3] Zsh: `.
/// Returns true if output was printed (TTY only).
pub fn print_dot_header(name: &str, idx: usize, total: usize) -> bool {
    if io::stderr().is_terminal() {
        if total > 1 {
            eprint!("\r  [{}/{}] {}: ", idx, total, name);
        } else {
            eprint!("\r  {}: ", name);
        }
        io::stderr().flush().ok();
        true
    } else {
        false
    }
}

/// Print progress footer for a dot (newline only if header was printed).
pub fn print_dot_footer(had_header: bool) {
    if had_header {
        eprintln!();
    }
}

// ── Migration ──────────────────────────────────────────────────────────────

/// Ensures all dots have IDs. For legacy configs without IDs:
/// 1. Generates an 8-char nanoid
/// 2. Renames old vault directory from `{name}` to `{id}_{name}`
/// 3. Saves the updated config
pub fn ensure_ids(config: &mut OmahConfig, config_path: &Path) -> Result<()> {
    let vault = expand_path(&config.vault_path)?;
    let mut changed = false;

    for dot in &mut config.dots {
        if dot.id.is_none() {
            let id = nanoid::nanoid!(8);
            dot.id = Some(id);
            changed = true;

            // Rename old vault directory if it exists
            let old_vault = vault.join(&dot.name);
            let new_vault = vault.join(format!("{}_{}", dot.id.as_ref().unwrap(), dot.name));
            if old_vault.exists() {
                fs::rename(&old_vault, &new_vault)?;
            }
        }
    }

    if changed {
        crate::config::save_toml_config(config, config_path)?;
    }

    Ok(())
}

// ── Public operations ──────────────────────────────────────────────────────

pub fn backup(config: &OmahConfig, dry_run: bool) -> Result<()> {
    let vault = expand_path(&config.vault_path)?;
    let total_dots = config.dots.len();

    if dry_run {
        println!("Backup plan:\n");
    } else {
        fs::create_dir_all(&vault)?;
    }

    for (i, dot) in config.dots.iter().enumerate() {
        let source = expand_path(&dot.source)?;
        let filename = match source.file_name() {
            Some(f) => f.to_owned(),
            None => anyhow::bail!("'{}': source has no filename", dot.name),
        };
        let dest = vault_dir(&vault, dot).join(filename);
        let excludes = dot.exclude.as_deref().unwrap_or(&[]);

        let had_header = print_dot_header(&dot.name, i + 1, total_dots);

        // If source is already a symlink pointing at dest, skip the copy.
        // fs::copy follows the symlink, which means it opens dest for writing
        // (truncating it to 0 bytes) before reading source — which now reads
        // through the symlink from the just-emptied dest, silently zeroing the vault.
        let already_symlinked_to_dest = source.is_symlink()
            && fs::read_link(&source).map(|t| t == dest).unwrap_or(false);

        if already_symlinked_to_dest {
            if dry_run {
                print_dot_footer(had_header);
                println!("  {}: up-to-date (symlink → vault)", dot.name);
            }
            continue;
        }

        if !source.exists() {
            print_dot_footer(had_header);
            if dry_run {
                println!("  {}: !!! source not found", dot.name);
                continue;
            }
            anyhow::bail!("Source not found: {}", source.display());
        }

        let total = count_files(&source, excludes);

        if dry_run {
            print_dot_footer(had_header);
            let sym = if dot.symlink.unwrap_or(false) { " [symlink]" } else { "" };
            println!("  {}: {} → {} ({} files){}", dot.name, source.display(), dest.display(), total, sym);
            continue;
        }

        let show_file_progress = total > 5 && io::stderr().is_terminal();
        if show_file_progress {
            // print_dot_header already wrote our line, file-progress overwrites it
        }
        let counter = AtomicU64::new(0);
        let progress = show_file_progress.then_some((&counter, total));

        copy_recursive(&source, &dest, excludes, progress).with_context(
            || format!("Failed to backup '{}' from {}", dot.name, source.display()),
        )?;

        print_dot_footer(had_header);

        if dot.symlink.unwrap_or(false) {
            remove_path(&source)
                .with_context(|| format!("Failed to remove source for '{}'", dot.name))?;
            std::os::unix::fs::symlink(&dest, &source)
                .with_context(|| format!("Failed to create symlink for '{}'", dot.name))?;
        }
    }

    Ok(())
}

pub fn restore(config: &OmahConfig, dry_run: bool) -> Result<()> {
    let vault = expand_path(&config.vault_path)?;
    let mut errors: Vec<String> = Vec::new();
    let total_dots = config.dots.len();

    if dry_run {
        println!("Restore plan:\n");
    }

    for (i, dot) in config.dots.iter().enumerate() {
        let source = match expand_path(&dot.source) {
            Ok(p) => p,
            Err(e) => {
                let msg = e.to_string();
                if dry_run {
                    println!("  {}: !!! source error: {}", dot.name, msg);
                } else {
                    errors.push(msg);
                }
                continue;
            }
        };
        let filename = match source.file_name() {
            Some(f) => f.to_owned(),
            None => {
                let msg = format!("'{}': source has no filename", dot.name);
                if dry_run {
                    println!("  {}: !!! {}", dot.name, msg);
                } else {
                    errors.push(msg);
                }
                continue;
            }
        };
        let vault_entry = vault_dir(&vault, dot).join(&filename);

        let had_header = print_dot_header(&dot.name, i + 1, total_dots);

        if !vault_entry.exists() {
            print_dot_footer(had_header);
            let msg = format!("'{}': vault entry not found at {}", dot.name, vault_entry.display());
            if dry_run {
                println!("  {}: !!! {}", dot.name, msg);
            } else {
                errors.push(msg);
            }
            continue;
        }

        let makes_symlink = dot.symlink.unwrap_or(false);

        // Idempotent: if source is already a symlink to the vault entry, skip.
        let already_symlinked_to_dest = makes_symlink
            && source.is_symlink()
            && fs::read_link(&source).map(|t| t == vault_entry).unwrap_or(false);

        if already_symlinked_to_dest {
            print_dot_footer(had_header);
            if dry_run {
                println!("  {}: up-to-date (symlink → vault)", dot.name);
            }
            continue;
        }

        if dry_run {
            print_dot_footer(had_header);
            let total = count_files(&vault_entry, &[]);
            let sym = if makes_symlink { " [symlink]" } else { "" };
            println!("  {}: {} → {} ({} files){}", dot.name, vault_entry.display(), source.display(), total, sym);
            continue;
        }

        let result = if makes_symlink {
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
            let excludes: &[String] = &[];
            let total = count_files(&vault_entry, excludes);
            let show_file_progress = total > 5 && io::stderr().is_terminal();
            let counter = AtomicU64::new(0);
            let progress = show_file_progress.then_some((&counter, total));

            copy_recursive(&vault_entry, &source, excludes, progress)
                .with_context(|| {
                    format!("Failed to restore '{}' to {}", dot.name, source.display())
                })
        };

        print_dot_footer(had_header);

        if let Err(e) = result {
            errors.push(e.to_string());
        }
    }

    if !dry_run && !errors.is_empty() {
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
            let vault_entry = vault_dir(&vault, dot).join(filename);

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
        let vault_entry = vault_dir(&vault, dot).join(filename);
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
                    if always_excluded(&name) { continue; }
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
                    if always_excluded(&name) { continue; }
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
                        let name = e?.file_name();
                        if !always_excluded(&name) { names.insert(name); }
                    }
                }
                if vault.is_dir() {
                    for e in fs::read_dir(vault)? {
                        let name = e?.file_name();
                        if !always_excluded(&name) { names.insert(name); }
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
        OmahConfig { vault_path: vault.to_string(), dots, os: None, pkg_manager: None }
    }

    fn dot(name: &str, source: &str, symlink: Option<bool>) -> DotfileConfig {
        DotfileConfig {
            name: name.to_string(),
            source: source.to_string(),
            id: None,
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
        backup(&config, false).unwrap();

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
        backup(&config, false).unwrap();

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
        backup(&config, false).unwrap();

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
        backup(&config, false).unwrap();

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

        backup(&config, false).unwrap(); // first backup: copies file, creates symlink
        backup(&config, false).unwrap(); // second backup: must not zero out the vault

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
        assert!(backup(&config, false).is_err());
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
        backup(&config, false).unwrap();

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
        restore(&config, false).unwrap();

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
        restore(&config, false).unwrap();

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
        restore(&config, false).unwrap();

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
        assert!(restore(&config, false).is_err());
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
        backup(&config, false).unwrap();
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
        backup(&config, false).unwrap();
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
        backup(&config, false).unwrap();
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
        backup(&config, false).unwrap();
        fs::write(&source, "# zsh edited").unwrap();
        let changes = diff(&config).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].kind, ChangeKind::Modified);
    }

    // ── ensure_ids ──────────────────────────────────────────────────────────

    #[test]
    fn test_ensure_ids_generates_id_and_renames_vault() {
        let vault_dir = tempdir().unwrap();
        let config_dir = tempdir().unwrap();
        let config_path = config_dir.path().join("config.toml");

        // Create legacy vault dir: vault/Zsh/.zshrc
        let legacy = vault_dir.path().join("Zsh");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join(".zshrc"), "# legacy").unwrap();

        // Write config without IDs
        let mut config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Zsh", "/home/.zshrc", None)],
        );
        crate::config::save_toml_config(&config, &config_path).unwrap();

        // Run ensure_ids
        ensure_ids(&mut config, &config_path).unwrap();

        // ID was assigned
        assert!(config.dots[0].id.is_some());
        let id = config.dots[0].id.as_ref().unwrap();

        // Legacy vault was renamed
        let new_vault = vault_dir.path().join(format!("{id}_Zsh"));
        assert!(new_vault.is_dir());
        assert!(!legacy.exists());
        assert!(new_vault.join(".zshrc").is_file());

        // Config was saved to disk with ID
        let loaded = crate::config::load_toml_config(&config_path).unwrap();
        assert_eq!(loaded.dots[0].id, config.dots[0].id);
    }

    #[test]
    fn test_ensure_ids_no_vault_dir() {
        let vault_dir = tempdir().unwrap();
        let config_dir = tempdir().unwrap();
        let config_path = config_dir.path().join("config.toml");

        let mut config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![dot("Zsh", "/home/.zshrc", None)],
        );
        crate::config::save_toml_config(&config, &config_path).unwrap();

        ensure_ids(&mut config, &config_path).unwrap();

        // ID was assigned even though no vault dir existed
        assert!(config.dots[0].id.is_some());
    }

    #[test]
    fn test_ensure_ids_already_has_id() {
        let vault_dir = tempdir().unwrap();
        let config_dir = tempdir().unwrap();
        let config_path = config_dir.path().join("config.toml");

        let dot = DotfileConfig {
            name: "Zsh".into(),
            source: "/home/.zshrc".into(),
            id: Some("abc12345".into()),
            symlink: None,
            deps: None,
            setup: None,
            exclude: None,
        };
        let mut config = make_config(vault_dir.path().to_str().unwrap(), vec![dot]);
        crate::config::save_toml_config(&config, &config_path).unwrap();

        ensure_ids(&mut config, &config_path).unwrap();

        // ID unchanged
        assert_eq!(config.dots[0].id.as_deref(), Some("abc12345"));
    }

    #[test]
    fn test_ensure_ids_multiple_dots() {
        let vault_dir = tempdir().unwrap();
        let config_dir = tempdir().unwrap();
        let config_path = config_dir.path().join("config.toml");

        // Create two legacy vault dirs
        let old_a = vault_dir.path().join("A");
        fs::create_dir_all(&old_a).unwrap();
        fs::write(old_a.join("file"), "a").unwrap();
        let old_b = vault_dir.path().join("B");
        fs::create_dir_all(&old_b).unwrap();
        fs::write(old_b.join("file"), "b").unwrap();

        let mut config = make_config(
            vault_dir.path().to_str().unwrap(),
            vec![
                dot("A", "/a", None),
                dot("B", "/b", None),
            ],
        );
        crate::config::save_toml_config(&config, &config_path).unwrap();

        ensure_ids(&mut config, &config_path).unwrap();

        // Both have IDs
        assert!(config.dots[0].id.is_some());
        assert!(config.dots[1].id.is_some());

        let id_a = config.dots[0].id.as_ref().unwrap();
        let id_b = config.dots[1].id.as_ref().unwrap();

        // Both vault dirs renamed
        assert!(vault_dir.path().join(format!("{id_a}_A")).is_dir());
        assert!(vault_dir.path().join(format!("{id_b}_B")).is_dir());
        assert!(!old_a.exists());
        assert!(!old_b.exists());

        // Different IDs
        assert_ne!(id_a, id_b);
    }
}
