use expand_tilde::ExpandTilde;
use omah_structs::{DotfileConfig, SetupStep};

/// Returns the declared dep list (empty if field omitted).
pub fn declared_deps(dot: &DotfileConfig) -> &[String] {
    dot.deps.as_deref().unwrap_or(&[])
}

/// Package name → binary name mapping for packages whose installed binary
/// differs from the package name. Lookup is case-insensitive.
///
/// Format: `("package-name", "binary-name")` — multiple package aliases may
/// map to the same binary (add them as separate entries).
pub const PKG_TO_BIN: &[(&str, &str)] = &[
    // ── Editors ──────────────────────────────────────────────────────────────
    ("neovim", "nvim"),
    ("vim-nox", "vim"),
    ("gvim", "vim"),
    ("emacs-nox", "emacs"),
    ("emacs-gtk", "emacs"),
    ("code", "code"),              // VS Code (snap/apt name)
    ("visual-studio-code", "code"),
    ("helix", "hx"),
    ("micro-editor", "micro"),
    ("kakoune", "kak"),
    // ── Search / file tools ──────────────────────────────────────────────────
    ("ripgrep", "rg"),
    ("fd-find", "fd"),             // Debian/Ubuntu apt name
    ("fd-rs", "fd"),
    ("bat-cat", "bat"),            // some distros
    ("the_silver_searcher", "ag"),
    ("silversearcher-ag", "ag"),   // Debian apt name
    ("ugrep", "ugrep"),
    ("hypergrep", "hgrep"),
    ("fzf", "fzf"),
    ("skim", "sk"),
    ("broot", "br"),
    ("lsd", "lsd"),
    ("eza", "eza"),
    ("exa", "eza"),                // deprecated alias
    ("tre-command", "tre"),
    ("dust", "dust"),
    ("dua-cli", "dua"),
    ("ncdu", "ncdu"),
    ("tokei", "tokei"),
    ("loc", "loc"),
    // ── Shell & navigation ───────────────────────────────────────────────────
    ("nushell", "nu"),
    ("zoxide", "zoxide"),
    ("fish", "fish"),
    ("zsh", "zsh"),
    ("bash", "bash"),
    ("dash", "dash"),
    ("elvish", "elvish"),
    ("xonsh", "xonsh"),
    ("oil-shell", "osh"),
    ("carapace-bin", "carapace"),
    ("atuin", "atuin"),
    ("mcfly", "mcfly"),
    ("direnv", "direnv"),
    ("starship", "starship"),
    ("oh-my-posh", "oh-my-posh"),
    // ── Terminal multiplexers ─────────────────────────────────────────────────
    ("tmux", "tmux"),
    ("zellij", "zellij"),
    ("screen", "screen"),
    ("byobu", "byobu"),
    // ── TUI system tools ──────────────────────────────────────────────────────
    ("bottom", "btm"),
    ("btop", "btop"),
    ("htop", "htop"),
    ("gtop", "gtop"),
    ("glances", "glances"),
    ("procs", "procs"),
    ("bandwhich", "bandwhich"),
    ("nethogs", "nethogs"),
    ("iftop", "iftop"),
    ("gping", "gping"),
    ("dog", "dog"),
    ("xh", "xh"),
    ("curlie", "curlie"),
    ("httpie", "http"),            // httpie → http binary
    ("http", "http"),
    // ── Diff / VCS ───────────────────────────────────────────────────────────
    ("difftastic", "difft"),
    ("delta", "delta"),
    ("git-delta", "delta"),
    ("diff-so-fancy", "diff-so-fancy"),
    ("lazygit", "lazygit"),
    ("gitui", "gitui"),
    ("tig", "tig"),
    ("gh", "gh"),                  // GitHub CLI
    ("glab", "glab"),              // GitLab CLI
    ("hub", "hub"),
    // ── Language runtimes & package managers ─────────────────────────────────
    ("nodejs", "node"),
    ("node-js", "node"),
    ("nodejs-lts", "node"),
    ("python", "python3"),
    ("python3-pip", "python3"),
    ("python3", "python3"),
    ("pyenv", "pyenv"),
    ("pipx", "pipx"),
    ("poetry", "poetry"),
    ("uv", "uv"),
    ("rye", "rye"),
    ("ruby", "ruby"),
    ("rbenv", "rbenv"),
    ("rvm", "rvm"),
    ("rustup", "rustup"),
    ("cargo", "cargo"),
    ("go", "go"),
    ("golang", "go"),
    ("java", "java"),
    ("openjdk", "java"),
    ("temurin", "java"),
    ("deno", "deno"),
    ("bun", "bun"),
    ("pnpm", "pnpm"),
    ("yarn", "yarn"),
    ("lua", "lua"),
    ("luarocks", "luarocks"),
    ("php", "php"),
    ("composer", "composer"),
    ("perl", "perl"),
    ("elixir", "elixir"),
    ("erlang", "erl"),
    ("erlang-solutions", "erl"),
    ("scala", "scala"),
    ("kotlin", "kotlin"),
    ("swift", "swift"),
    ("dart", "dart"),
    ("flutter", "flutter"),
    ("zig", "zig"),
    ("nim", "nim"),
    ("crystal", "crystal"),
    ("haskell-platform", "ghc"),
    ("ghc", "ghc"),
    ("stack", "stack"),
    ("cabal-install", "cabal"),
    ("ocaml", "ocaml"),
    ("opam", "opam"),
    ("dotnet", "dotnet"),
    ("dotnet-sdk", "dotnet"),
    // ── Build tools ──────────────────────────────────────────────────────────
    ("cmake", "cmake"),
    ("make", "make"),
    ("ninja-build", "ninja"),
    ("ninja", "ninja"),
    ("meson", "meson"),
    ("autoconf", "autoconf"),
    ("automake", "automake"),
    ("pkg-config", "pkg-config"),
    ("pkgconf", "pkg-config"),
    // ── Containers & cloud ───────────────────────────────────────────────────
    ("docker", "docker"),
    ("docker-ce", "docker"),
    ("podman", "podman"),
    ("kubectl", "kubectl"),
    ("kubernetes-cli", "kubectl"), // Homebrew name
    ("helm", "helm"),
    ("k9s", "k9s"),
    ("terraform", "terraform"),
    ("pulumi", "pulumi"),
    ("ansible", "ansible"),
    ("aws-cli", "aws"),
    ("awscli", "aws"),
    ("azure-cli", "az"),
    ("google-cloud-sdk", "gcloud"),
    ("gcloud", "gcloud"),
    ("flyctl", "flyctl"),
    ("wrangler", "wrangler"),
    // ── Fonts / misc ─────────────────────────────────────────────────────────
    ("stow", "stow"),
    ("chezmoi", "chezmoi"),
    ("mackup", "mackup"),
    ("antibody", "antibody"),
    ("antigen", "antigen"),
    ("zinit", "zinit"),
    ("sheldon", "sheldon"),
    ("topgrade", "topgrade"),
    ("mas", "mas"),               // Mac App Store CLI
    ("mas-cli", "mas"),
];

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

/// True if the binary exists in PATH.
pub fn is_installed(dep: &str) -> bool {
    which::which(pkg_to_bin(dep)).is_ok()
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
/// | `bin:nvim`            | `nvim` must be found in PATH                 |
/// | `file:~/.zshrc`       | the file must exist                          |
/// | `dir:~/.config/nvim`  | the directory must exist                     |
/// | `cmd:ls ... \| grep …` | shell command must exit 0                   |
/// | bare `nvim`           | backward-compat: binary check                |
/// | bare `/…` or `~/…`    | backward-compat: path existence check        |
/// | missing / empty       | always pending (no way to verify)            |
fn step_is_pending(step: &SetupStep) -> bool {
    match step.check.as_deref() {
        None | Some("") => true,
        Some(raw) => {
            let raw = raw.trim();
            if let Some(bin) = raw.strip_prefix("bin:") {
                which::which(bin.trim()).is_err()
            } else if let Some(path) = raw.strip_prefix("file:") {
                path.trim().expand_tilde().map(|p| !p.is_file()).unwrap_or(true)
            } else if let Some(path) = raw.strip_prefix("dir:") {
                path.trim().expand_tilde().map(|p| !p.is_dir()).unwrap_or(true)
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
            } else if raw == "skip" || raw.starts_with("skip:") {
                // User explicitly skipped this step — never pending
                false
            } else {
                // Backward-compat: bare path or bare binary name
                if raw.starts_with('/') || raw.starts_with('~') {
                    raw.expand_tilde().map(|p| !p.exists()).unwrap_or(true)
                } else {
                    which::which(raw).is_err()
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
