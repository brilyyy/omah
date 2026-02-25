use anyhow::{Context, Result};
use expand_tilde::ExpandTilde;
use omah_structs::OmahConfig;
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::constants::{DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILE};

pub fn load_toml_config(path: &Path) -> Result<OmahConfig> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    let config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
    Ok(config)
}

pub fn get_default_dir() -> Result<PathBuf> {
    DEFAULT_CONFIG_DIR
        .expand_tilde()
        .map(|p| p.to_path_buf())
        .context("Failed to determine home directory for default config path")
}

pub fn get_default_config_path() -> Result<PathBuf> {
    Ok(get_default_dir()?.join(DEFAULT_CONFIG_FILE))
}

pub fn check_dir_exists() -> Result<bool> {
    Ok(get_default_dir()?.is_dir())
}

pub fn check_file_exists() -> Result<bool> {
    Ok(get_default_config_path()?.is_file())
}

pub fn init_setup() -> Result<()> {
    init_at(get_default_dir()?)
}

pub(crate) fn init_at(config_dir: PathBuf) -> Result<()> {
    let config_path = config_dir.join(DEFAULT_CONFIG_FILE);

    if !config_dir.is_dir() {
        fs::create_dir_all(&config_dir).with_context(|| {
            format!(
                "Failed to create config directory: {}",
                config_dir.display()
            )
        })?;
    }

    if !config_path.is_file() {
        let default_config = concat!(
            "# Panggonan kanggo nyimpen backup (The Vault)\n",
            "vault_path = \"~/.config/omah/vault\"\n",
            "\n",
            "# [[dots]]\n",
            "# name = \"Example\"\n",
            "# source = \"~/.zshrc\"\n",
            "# symlink = false\n",
        );
        fs::write(&config_path, default_config).with_context(|| {
            format!("Failed to write default config: {}", config_path.display())
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_load_toml_config_valid() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r#"
vault_path = "/tmp/vault"

[[dots]]
name = "Zsh"
source = "/home/user/.zshrc"

[[dots]]
name = "Nvim"
source = "/home/user/.config/nvim"
symlink = true
"#,
        )
        .unwrap();

        let config = load_toml_config(&path).unwrap();
        assert_eq!(config.vault_path, "/tmp/vault");
        assert_eq!(config.dots.len(), 2);
        assert_eq!(config.dots[0].name, "Zsh");
        assert_eq!(config.dots[0].source, "/home/user/.zshrc");
        assert_eq!(config.dots[0].symlink, None);
        assert_eq!(config.dots[1].name, "Nvim");
        assert_eq!(config.dots[1].symlink, Some(true));
    }

    #[test]
    fn test_load_toml_config_invalid_toml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "invalid }{ toml").unwrap();
        assert!(load_toml_config(&path).is_err());
    }

    #[test]
    fn test_load_toml_config_missing_required_field() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        // missing vault_path
        fs::write(&path, "[[dots]]\nname = \"x\"\nsource = \"/tmp/x\"\n").unwrap();
        assert!(load_toml_config(&path).is_err());
    }

    #[test]
    fn test_load_toml_config_missing_file() {
        let path = Path::new("/nonexistent/path/config.toml");
        assert!(load_toml_config(path).is_err());
    }

    #[test]
    fn test_get_default_dir_is_absolute() {
        let dir = get_default_dir().unwrap();
        assert!(dir.is_absolute());
        assert!(dir.to_str().unwrap().ends_with(".config/omah"));
    }

    #[test]
    fn test_get_default_config_path_ends_with_filename() {
        let path = get_default_config_path().unwrap();
        assert!(path.to_str().unwrap().ends_with("omah-config.toml"));
    }

    #[test]
    fn test_init_at_creates_dir_and_file() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("omah");

        init_at(config_dir.clone()).unwrap();

        assert!(config_dir.is_dir());
        let config_path = config_dir.join(DEFAULT_CONFIG_FILE);
        assert!(config_path.is_file());
        let contents = fs::read_to_string(&config_path).unwrap();
        assert!(contents.contains("vault_path"));
    }

    #[test]
    fn test_init_at_idempotent() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("omah");

        init_at(config_dir.clone()).unwrap();

        // Write custom content to simulate user-edited config
        let config_path = config_dir.join(DEFAULT_CONFIG_FILE);
        fs::write(&config_path, "vault_path = \"/custom/vault\"\ndots = []\n").unwrap();

        // Second call must not overwrite
        init_at(config_dir.clone()).unwrap();
        let contents = fs::read_to_string(&config_path).unwrap();
        assert!(contents.contains("/custom/vault"));
    }
}
