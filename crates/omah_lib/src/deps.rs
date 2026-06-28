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
/// | `out:ok`                 | runs install cmd; done when stdout == `ok`  |
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
            } else if let Some(app_name) = raw.strip_prefix("app:") {
                let name = app_name.trim();
                let bundle = if name.ends_with(".app") {
                    name.to_string()
                } else {
                    format!("{name}.app")
                };
                let in_system = std::path::Path::new("/Applications").join(&bundle).is_dir();
                let in_home = format!("~/Applications/{bundle}")
                    .expand_tilde()
                    .map(|p| p.is_dir())
                    .unwrap_or(false);
                !(in_system || in_home)
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
            } else if let Some(expected) = raw.strip_prefix("out:") {
                // out:<expected> — runs the install command, done when trimmed stdout == expected.
                let out = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(step.install.trim())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::null())
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok());
                out.map(|s| s.trim() != expected.trim()).unwrap_or(true)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn dot(deps: Option<Vec<&str>>) -> DotfileConfig {
        DotfileConfig {
            name: "test".into(),
            source: "/dev/null".into(),
            symlink: None,
            deps: deps.map(|d| d.into_iter().map(String::from).collect()),
            setup: None,
            exclude: None,
        }
    }

    fn step(check: Option<&str>, install: &str) -> SetupStep {
        SetupStep {
            check: check.map(String::from),
            install: install.into(),
        }
    }

    // ── declared_deps ────────────────────────────────────────────────────────

    #[test]
    fn test_declared_deps_some() {
        let d = dot(Some(vec!["nvim", "git"]));
        assert_eq!(declared_deps(&d), &["nvim", "git"]);
    }

    #[test]
    fn test_declared_deps_none() {
        let d = dot(None);
        assert!(declared_deps(&d).is_empty());
    }

    #[test]
    fn test_declared_deps_empty_vec() {
        let d = dot(Some(vec![]));
        assert!(declared_deps(&d).is_empty());
    }

    // ── pkg_to_bin ───────────────────────────────────────────────────────────

    #[test]
    fn test_pkg_to_bin_known() {
        assert_eq!(pkg_to_bin("neovim"), "nvim");
    }

    #[test]
    fn test_pkg_to_bin_case_insensitive() {
        assert_eq!(pkg_to_bin("Neovim"), "nvim");
        assert_eq!(pkg_to_bin("NEOVIM"), "nvim");
    }

    #[test]
    fn test_pkg_to_bin_unknown_falls_back() {
        assert_eq!(pkg_to_bin("xyzzy_nope_12345"), "xyzzy_nope_12345");
    }

    #[test]
    fn test_pkg_to_bin_empty() {
        assert_eq!(pkg_to_bin(""), "");
    }

    // ── command_available ────────────────────────────────────────────────────

    #[test]
    fn test_command_available_sh() {
        assert!(command_available("sh"));
    }

    #[test]
    fn test_command_available_echo() {
        assert!(command_available("echo"));
    }

    #[test]
    fn test_command_available_nonexistent() {
        assert!(!command_available("xyzzy_nope_12345_does_not_exist"));
    }

    #[test]
    fn test_command_available_empty() {
        assert!(!command_available(""));
    }

    // ── is_installed ─────────────────────────────────────────────────────────

    #[test]
    fn test_is_installed_known_binary() {
        assert!(is_installed("sh"));
        assert!(is_installed("echo"));
    }

    #[test]
    fn test_is_installed_nonexistent() {
        assert!(!is_installed("xyzzy_nope_12345_does_not_exist"));
    }

    #[test]
    fn test_is_installed_mapped_pkg() {
        // PKG_TO_BIN maps neovim→nvim — both should resolve
        assert!(is_installed("neovim"));
    }

    #[test]
    fn test_is_installed_empty() {
        assert!(!is_installed(""));
    }

    // ── missing_deps ─────────────────────────────────────────────────────────

    #[test]
    fn test_missing_deps_none_when_all_installed() {
        let d = dot(Some(vec!["sh", "echo"]));
        assert!(missing_deps(&d).is_empty());
    }

    #[test]
    fn test_missing_deps_filters_missing() {
        let d = dot(Some(vec!["sh", "xyzzy_nope_12345_does_not_exist"]));
        let missing = missing_deps(&d);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], "xyzzy_nope_12345_does_not_exist");
    }

    #[test]
    fn test_missing_deps_none_when_no_declared() {
        let d = dot(None);
        assert!(missing_deps(&d).is_empty());
    }

    // ── resolve_pkg_manager ──────────────────────────────────────────────────

    #[test]
    fn test_resolve_pkg_manager_explicit_brew() {
        assert_eq!(resolve_pkg_manager(Some("brew")).unwrap(), "brew");
    }

    #[test]
    fn test_resolve_pkg_manager_explicit_custom() {
        assert_eq!(resolve_pkg_manager(Some("nix-env")).unwrap(), "nix-env");
    }

    #[test]
    fn test_resolve_pkg_manager_none_delegates_to_detect() {
        // Non-deterministic — depends on CI/dev environment.
        // Just verify it returns Option<String> without panicking.
        let result = resolve_pkg_manager(None);
        let _: Option<String> = result;
    }

    #[test]
    fn test_resolve_pkg_manager_auto_delegates_to_detect() {
        let result = resolve_pkg_manager(Some("auto"));
        let _: Option<String> = result;
    }

    #[test]
    fn test_resolve_pkg_manager_empty_delegates_to_detect() {
        let result = resolve_pkg_manager(Some(""));
        let _: Option<String> = result;
    }

    // ── step_is_pending: None / "" ───────────────────────────────────────────

    #[test]
    fn test_step_pending_none() {
        assert!(step_is_pending(&step(None, "")));
    }

    #[test]
    fn test_step_pending_empty_string() {
        assert!(step_is_pending(&step(Some(""), "")));
    }

    // ── step_is_pending: skip ────────────────────────────────────────────────

    #[test]
    fn test_step_pending_skip() {
        assert!(!step_is_pending(&step(Some("skip"), "")));
    }

    #[test]
    fn test_step_pending_skip_with_reason() {
        assert!(!step_is_pending(&step(Some("skip:already have it"), "")));
    }

    // ── step_is_pending: bin: ────────────────────────────────────────────────

    #[test]
    fn test_step_pending_bin_exists() {
        assert!(!step_is_pending(&step(Some("bin:sh"), "")));
    }

    #[test]
    fn test_step_pending_bin_missing() {
        assert!(step_is_pending(&step(Some("bin:xyzzy_nope_12345"), "")));
    }

    #[test]
    fn test_step_pending_bin_trimmed() {
        assert!(!step_is_pending(&step(Some("bin:  sh  "), "")));
    }

    #[test]
    fn test_step_pending_bin_empty() {
        assert!(step_is_pending(&step(Some("bin:"), "")));
    }

    // ── step_is_pending: file: ───────────────────────────────────────────────

    #[test]
    fn test_step_pending_file_exists() {
        let dir = tempdir().unwrap();
        let f = dir.path().join("test.txt");
        std::fs::write(&f, "content").unwrap();
        assert!(!step_is_pending(&step(Some(&format!("file:{}", f.display())), "")));
    }

    #[test]
    fn test_step_pending_file_missing() {
        assert!(step_is_pending(&step(Some("file:/xyzzy_nope_12345_file"), "")));
    }

    #[test]
    fn test_step_pending_file_empty_path() {
        assert!(step_is_pending(&step(Some("file:"), "")));
    }

    // ── step_is_pending: dir: ────────────────────────────────────────────────

    #[test]
    fn test_step_pending_dir_exists_via_tempdir() {
        let dir = tempdir().unwrap();
        assert!(!step_is_pending(&step(Some(&format!("dir:{}", dir.path().display())), "")));
    }

    #[test]
    fn test_step_pending_dir_root() {
        assert!(!step_is_pending(&step(Some("dir:/"), "")));
    }

    #[test]
    fn test_step_pending_dir_missing() {
        assert!(step_is_pending(&step(Some("dir:/xyzzy_nope_12345_dir"), "")));
    }

    #[test]
    fn test_step_pending_dir_empty() {
        assert!(step_is_pending(&step(Some("dir:"), "")));
    }

    // ── step_is_pending: cmd: ────────────────────────────────────────────────

    #[test]
    fn test_step_pending_cmd_success() {
        assert!(!step_is_pending(&step(Some("cmd:true"), "")));
    }

    #[test]
    fn test_step_pending_cmd_failure() {
        assert!(step_is_pending(&step(Some("cmd:false"), "")));
    }

    #[test]
    fn test_step_pending_cmd_echo_success() {
        assert!(!step_is_pending(&step(Some("cmd:echo hi"), "")));
    }

    // ── step_is_pending: out: ────────────────────────────────────────────────

    #[test]
    fn test_step_pending_out_matches() {
        assert!(!step_is_pending(&step(Some("out:ok"), "echo ok")));
    }

    #[test]
    fn test_step_pending_out_mismatch() {
        assert!(step_is_pending(&step(Some("out:ok"), "echo fail")));
    }

    #[test]
    fn test_step_pending_out_empty_expected() {
        assert!(!step_is_pending(&step(Some("out:"), "true")));
    }

    // ── step_is_pending: bare binary (backward compat) ───────────────────────

    #[test]
    fn test_step_pending_bare_binary_exists() {
        assert!(!step_is_pending(&step(Some("sh"), "")));
    }

    #[test]
    fn test_step_pending_bare_binary_missing() {
        assert!(step_is_pending(&step(Some("xyzzy_nope_12345"), "")));
    }

    // ── step_is_pending: bare path (backward compat) ─────────────────────────

    #[test]
    fn test_step_pending_bare_path_root() {
        assert!(!step_is_pending(&step(Some("/"), "")));
    }

    #[test]
    fn test_step_pending_bare_path_tmp() {
        assert!(!step_is_pending(&step(Some("/tmp"), "")));
    }

    #[test]
    fn test_step_pending_bare_path_missing() {
        assert!(step_is_pending(&step(Some("/xyzzy_nope_12345_path"), "")));
    }

    #[test]
    fn test_step_pending_bare_path_home_tilde() {
        assert!(!step_is_pending(&step(Some("~"), "")));
    }

    // ── install_command ──────────────────────────────────────────────────────

    #[test]
    fn test_install_command_brew() {
        assert_eq!(install_command("brew", &["nvim".into()]), "brew install nvim");
    }

    #[test]
    fn test_install_command_apt_get() {
        assert_eq!(
            install_command("apt-get", &["neovim".into()]),
            "sudo apt-get install -y neovim"
        );
    }

    #[test]
    fn test_install_command_pacman() {
        assert_eq!(
            install_command("pacman", &["neovim".into()]),
            "sudo pacman -S --noconfirm neovim"
        );
    }

    #[test]
    fn test_install_command_dnf() {
        assert_eq!(
            install_command("dnf", &["neovim".into()]),
            "sudo dnf install -y neovim"
        );
    }

    #[test]
    fn test_install_command_zypper() {
        assert_eq!(
            install_command("zypper", &["neovim".into()]),
            "sudo zypper install -y neovim"
        );
    }

    #[test]
    fn test_install_command_fallback_unknown() {
        assert_eq!(
            install_command("nix-env", &["neovim".into()]),
            "nix-env install neovim"
        );
    }

    #[test]
    fn test_install_command_multiple_deps() {
        assert_eq!(
            install_command("brew", &["nvim".into(), "git".into(), "zsh".into()]),
            "brew install nvim git zsh"
        );
    }

    #[test]
    fn test_install_command_empty_deps() {
        let result = install_command("brew", &[]);
        assert_eq!(result, "brew install ");
    }

    // ── pending_setup_steps ──────────────────────────────────────────────────

    #[test]
    fn test_pending_setup_steps_mixed() {
        let dot = DotfileConfig {
            name: "test".into(),
            source: "/dev/null".into(),
            symlink: None,
            deps: None,
            setup: Some(vec![
                step(Some("skip"), ""),
                step(Some("bin:sh"), ""),
                step(Some("bin:xyzzy_nope_12345"), ""),
                step(Some("bin:echo"), ""),
            ]),
            exclude: None,
        };
        let pending = pending_setup_steps(&dot);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].check.as_deref(), Some("bin:xyzzy_nope_12345"));
    }

    #[test]
    fn test_pending_setup_steps_all_done() {
        let dot = DotfileConfig {
            name: "test".into(),
            source: "/dev/null".into(),
            symlink: None,
            deps: None,
            setup: Some(vec![
                step(Some("skip"), ""),
                step(Some("bin:sh"), ""),
                step(Some("bin:echo"), ""),
            ]),
            exclude: None,
        };
        assert!(pending_setup_steps(&dot).is_empty());
    }

    #[test]
    fn test_pending_setup_steps_no_setup() {
        let d = dot(None);
        assert!(pending_setup_steps(&d).is_empty());
    }
}
