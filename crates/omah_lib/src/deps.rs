use expand_tilde::ExpandTilde;
use omah_structs::{DotfileConfig, SetupStep};

/// Returns the declared dep list (empty if field omitted).
pub fn declared_deps(dot: &DotfileConfig) -> &[String] {
    dot.deps.as_deref().unwrap_or(&[])
}

/// True if the binary exists in PATH.
pub fn is_installed(dep: &str) -> bool {
    which::which(dep).is_ok()
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

/// Returns setup steps that are pending (check path absent or no check).
pub fn pending_setup_steps(dot: &DotfileConfig) -> Vec<&SetupStep> {
    dot.setup.as_deref().unwrap_or(&[])
        .iter()
        .filter(|step| {
            step.check.as_ref().map_or(true, |check| {
                check.expand_tilde().map(|p| !p.exists()).unwrap_or(true)
            })
        })
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
