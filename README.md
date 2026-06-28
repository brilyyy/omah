<div align="center">

```

    ╔╦╗╔═╗╦═╗╔═╗╔╦╗  ╦╔═╗╔═╗╦═╗╔╦╗
     ║ ║╣ ╠╦╝╠═╣ ║   ║╚═╗║╣ ╠╦╝║║║
     ╩ ╚═╝╩╚═╩ ╩ ╩   ╩╚═╝╚═╝╩╚═╩ ╩

```

### *Your dotfiles' home.*

**omah** is a dotfile manager that keeps your config files safe in a vault and restores them on any machine with one command.

[![CI](https://github.com/brilyyy/omah/actions/workflows/test.yml/badge.svg)](https://github.com/brilyyy/omah/actions)
[![Release](https://img.shields.io/github/v/release/brilyyy/omah)](https://github.com/brilyyy/omah/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

</div>

---

## ✨ What can omah do?

<details open>
<summary><strong>Features</strong></summary>

- [x] **Backup & Restore** — copy dotfiles into a vault, restore them on any machine
- [x] **Per-dotfile operations** — backup or restore a single dotfile by name
- [x] **Symlink mode** — replace source with a symlink into the vault
- [x] **Diff viewer** — see what changed between your live files and the vault
- [x] **Status dashboard** — live sync state for every configured dotfile
- [x] **Exclude patterns** — glob-based file filtering (skip `.log`, `.git`, etc.)
- [x] **Dependency checking** — verify required binaries and shell functions are available before restore
- [x] **Setup steps** — run shell commands after restore, with smart skip checks
- [x] **OS & package manager detection** — works on macOS and Linux, auto-detects brew/apt/pacman

</details>
<details>
<summary><strong>CLI Commands</strong></summary>

- [x] `omah init` — scaffold config on first run
- [x] `omah backup` — back up all dotfiles (or just one)
- [x] `omah restore` — restore all dotfiles (or just one)
- [x] `omah status` — show sync state
- [x] `omah diff` — show what changed
- [x] `omah list` — list configured dotfiles
- [x] `omah info` — show detailed dotfile info with state, deps, setup
- [x] `omah add` — add a dotfile entry
- [x] `omah remove` — remove a dotfile entry

</details>

---

## 🚀 Quick Start

### Install

**Prebuilt binaries** — grab the latest from [Releases](https://github.com/brilyyy/omah/releases):

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `omah-v*-macos-aarch64.tar.gz` · `omah_*.aarch64.dmg` |
| macOS (Intel) | `omah-v*-macos-x86_64.tar.gz` · `omah_*.x64.dmg` |
| Linux (x86_64) | `omah-v*-linux-x86_64.tar.gz` · `omah_*.AppImage` |

```sh
# Quick install (macOS / Linux)
curl -fsSL https://raw.githubusercontent.com/brilyyy/omah/master/install.sh | bash

# Or manually
chmod +x omah-* && mv omah-* /usr/local/bin/omah
```

**Build from source:**

```sh
cargo install --path crates/omah_bin
```

### Set it up

```sh
omah init          # creates ~/.config/omah/omah-config.toml
```

Edit the config to add your dotfiles:

```toml
vault_path = "~/Documents/OmahVault"

[[dots]]
name = "Zsh"
source = "~/.zshrc"
deps = ["zsh"]

[[dots]]
name = "Neovim"
source = "~/.config/nvim"
deps = ["nvim", "git", "ripgrep"]
exclude = ["*.log", ".git"]
```

### Use it

```sh
omah backup              # save your dotfiles to the vault
omah status              # check what's synced
omah diff                # see what changed since last backup
omah restore             # put dotfiles back from the vault
omah backup zsh          # backup a single dotfile
omah info zsh            # show detail for one dotfile
omah add nvim ~/.config/nvim --symlink  # add dotfile with symlink
omah remove nvim         # remove dotfile from config
omah -c path.toml        # use a custom config file
```

> **Tip:** Use `omah backup --no-exclude` to ignore exclude patterns.  
> Use `omah add --symlink` to create a symlink from source into the vault on backup.

---

## 📖 Config

Default location: `~/.config/omah/omah-config.toml`

| Field | What it does |
|-------|-------------|
| `vault_path` | Where your dotfiles are stored |
| `os` | `"auto"` detects at runtime, or set `"macos"` / `"linux"` |
| `pkg_manager` | `"auto"` detects, or set `"brew"` / `"apt-get"` / `"pacman"` / `"dnf"` / `"zypper"` |
| `name` | Label for the dotfile (also the vault subfolder) |
| `source` | Path to the file or directory on your machine |
| `symlink` | `true` = replace source with a symlink after backup |
| `deps` | Binaries that must be installed (e.g. `["nvim", "git"]`) |
| `setup` | Shell commands to run after restore |
| `exclude` | Glob patterns to skip (e.g. `["*.log", ".git"]`) |

### Setup step checks

Each setup step can have a `check` value that determines when it's considered done:

| Check | Skipped when... |
|-------|-----------------|
| `bin:name` | Binary exists in PATH |
| `file:path` | File exists |
| `dir:path` | Directory exists |
| `app:name` | macOS app exists in /Applications |
| `cmd:...` | Shell command exits 0 |
| `out:text` | Install command stdout matches |
| `skip` | Always skipped |

---

## 🤝 Contributing

```sh
git clone https://github.com/brilyyy/omah.git
cd omah
cargo test           # run tests
cargo clippy         # lint
```

<details>
<summary><strong>Project structure</strong></summary>

```
crates/
├── omah_structs/   # Core data types
├── omah_lib/       # Business logic (backup, restore, diff, deps)
├── omah_core/      # Re-exports for the desktop app
└── omah_bin/       # CLI entry point
```

</details>

---

## 📄 License

MIT © [brilyyy](https://github.com/brilyyy)
