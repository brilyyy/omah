use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

pub fn is_git_repo(vault: &Path) -> bool {
    vault.join(".git").exists()
}

/// Stage all changes in `vault` and commit them. Initialises the repo first if needed.
/// Returns silently if there is nothing to commit.
pub fn auto_commit_vault(vault: &Path) -> Result<()> {
    let vault_str = vault
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("vault path contains non-UTF-8 characters"))?;

    if !is_git_repo(vault) {
        Command::new("git")
            .args(["-C", vault_str, "init"])
            .output()
            .context("Failed to run `git init` in vault")?;
    }

    let add = Command::new("git")
        .args(["-C", vault_str, "add", "-A"])
        .output()
        .context("Failed to run `git add -A`")?;
    if !add.status.success() {
        let stderr = String::from_utf8_lossy(&add.stderr);
        anyhow::bail!("git add failed: {}", stderr.trim());
    }

    // Check if there is anything staged
    let porcelain = Command::new("git")
        .args(["-C", vault_str, "status", "--porcelain"])
        .output()
        .context("Failed to run `git status`")?;

    if porcelain.stdout.is_empty() {
        return Ok(()); // nothing to commit
    }

    let out = Command::new("git")
        .args(["-C", vault_str, "commit", "-m", "omah backup"])
        .output()
        .context("Failed to run `git commit`")?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("git commit failed: {}", stderr.trim());
    }

    Ok(())
}
