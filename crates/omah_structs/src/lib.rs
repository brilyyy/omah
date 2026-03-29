use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OmahConfig {
    pub vault_path: String,
    pub dots: Vec<DotfileConfig>,
    /// Target OS. `"auto"` (default) detects at runtime. Accepts `"macos"` or `"linux"`.
    pub os: Option<String>,
    /// Package manager to use when installing deps. `"auto"` (default) detects from PATH.
    /// Accepts `"brew"`, `"apt-get"`, `"pacman"`, `"dnf"`, `"zypper"`.
    pub pkg_manager: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SetupStep {
    /// Path to check for existence; if it exists, skip this step.
    pub check: Option<String>,
    /// Shell command to run.
    pub install: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DotfileConfig {
    pub name: String,
    pub source: String,
    pub symlink: Option<bool>,
    pub deps: Option<Vec<String>>,
    pub setup: Option<Vec<SetupStep>>,
    /// Glob patterns to skip when recursively copying a directory (e.g. `["*.log", ".git"]`).
    pub exclude: Option<Vec<String>>,
}
