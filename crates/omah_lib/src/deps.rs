use expand_tilde::ExpandTilde;
use omah_structs::{DotfileConfig, SetupStep};

use crate::constants::PKG_TO_BIN;

/// Returns the declared dep list (empty if field omitted).
pub fn declared_deps(dot: &DotfileConfig) -> &[String] {
    dot.deps.as_deref().unwrap_or(&[])
}

/// Maps a package name to the binary it installs.
/// Falls back to the package name itself when no mapping is found.
fn pkg_to_bin(pkg: &str) -> &str {
    let lower = pkg.to_lowercase();
    PKG_TO_BIN
        .iter()
        .find(|(p, _)| *p == lower.as_str())
        .map(|(_, b)| *b)
        .unwrap_or(pkg)
}

/// True if `name` resolves — binary in PATH or shell function/alias sourced
/// in rc files (e.g. nvm). Fast path: `which`. Slow path: interactive
/// `$SHELL -i -c "command -v <name>"`, only runs when `which` misses.
fn command_available(name: &str) -> bool {
    if which::which(name).is_ok() {
        return true;
    }
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
    std::process::Command::new(&shell)
        .args(["-i", "-c", &format!("command -v {name}")])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn is_installed(dep: &str) -> bool {
    command_available(pkg_to_bin(dep))
}

/// Returns declared deps that are not currently installed.
pub fn missing_deps(dot: &DotfileConfig) -> Vec<String> {
    declared_deps(dot)
        .iter()
        .filter(|d| !is_installed(d))
        .cloned()
        .collect()
}

/// Detect the system package manager (first match wins).
pub fn detect_package_manager() -> Option<&'static str> {
    ["brew", "apt-get", "pacman", "dnf", "zypper"]
        .iter()
        .find(|&pm| which::which(pm).is_ok())
        .map(|v| v as _)
}

/// Resolve the effective package manager from a config value.
/// `None` or `"auto"` → auto-detect; any other value is used as-is.
pub fn resolve_pkg_manager(configured: Option<&str>) -> Option<String> {
    match configured {
        None | Some("auto") | Some("") => detect_package_manager().map(|s| s.to_string()),
        Some(pm) => Some(pm.to_string()),
    }
}

/// Returns true when a setup step still needs to run.
///
/// The `check` field supports explicit prefixes and bare values:
///
/// | Stored value          | Meaning                                      |
/// |---------------------- |--------------------------------------------- |
/// | `bin:nvim`               | `nvim` must be found in PATH / shell        |
/// | `file:~/.zshrc`          | the file must exist                         |
/// | `dir:~/.config/nvim`     | the directory must exist                    |
/// | `cmd:ls … \| grep …`     | shell command must exit 0                   |
/// | `out:ok:node -e "…"`     | trimmed stdout must equal `ok`              |
/// | bare `nvim`              | backward-compat: binary check               |
/// | bare `/…` or `~/…`       | backward-compat: path existence check       |
/// | missing / empty          | always pending (no way to verify)           |
fn step_is_pending(step: &SetupStep) -> bool {
    match step.check.as_deref() {
        None | Some("") => true,
        Some(raw) => {
            let raw = raw.trim();
            if let Some(bin) = raw.strip_prefix("bin:") {
                !command_available(bin.trim())
            } else if let Some(path) = raw.strip_prefix("file:") {
                path.trim()
                    .expand_tilde()
                    .map(|p| !p.is_file())
                    .unwrap_or(true)
            } else if let Some(path) = raw.strip_prefix("dir:") {
                path.trim()
                    .expand_tilde()
                    .map(|p| !p.is_dir())
                    .unwrap_or(true)
            } else if let Some(cmd) = raw.strip_prefix("cmd:") {
                // Run the shell snippet; step is done when it exits 0.
                std::process::Command::new("sh")
                    .arg("-c")
                    .arg(cmd.trim())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .map(|s| !s.success())
                    .unwrap_or(true)
            } else if let Some(rest) = raw.strip_prefix("out:") {
                // out:<expected>:<cmd> — step done when trimmed stdout == expected.
                // Example: out:ok:node -e "..." | echo "ok"
                if let Some((expected, cmd)) = rest.split_once(':') {
                    let out = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(cmd.trim())
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::null())
                        .output()
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok());
                    out.map(|s| s.trim() != expected.trim()).unwrap_or(true)
                } else {
                    true // malformed — treat as pending
                }
            } else if raw == "skip" || raw.starts_with("skip:") {
                // User explicitly skipped this step — never pending
                false
            } else {
                // Backward-compat: bare path or bare binary name
                if raw.starts_with('/') || raw.starts_with('~') {
                    raw.expand_tilde().map(|p| !p.exists()).unwrap_or(true)
                } else {
                    !command_available(raw)
                }
            }
        }
    }
}

/// Returns setup steps that are pending.
pub fn pending_setup_steps(dot: &DotfileConfig) -> Vec<&SetupStep> {
    dot.setup
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .filter(|step| step_is_pending(step))
        .collect()
}

/// Build the install command for a list of packages.
pub fn install_command(pm: &str, deps: &[String]) -> String {
    let pkgs = deps.join(" ");
    match pm {
        "brew" => format!("brew install {pkgs}"),
        "apt-get" => format!("sudo apt-get install -y {pkgs}"),
        "pacman" => format!("sudo pacman -S --noconfirm {pkgs}"),
        "dnf" => format!("sudo dnf install -y {pkgs}"),
        "zypper" => format!("sudo zypper install -y {pkgs}"),
        _ => format!("{pm} install {pkgs}"),
    }
}
