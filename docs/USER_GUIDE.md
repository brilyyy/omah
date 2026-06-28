# omah — User Guide

## 1. Installation

### Install script (macOS / Linux)

```sh
# Quick install — downloads prebuilt binary
curl -fsSL https://raw.githubusercontent.com/brilyyy/omah/master/scripts/install.sh | bash

# Build from source (requires Rust toolchain)
curl -fsSL https://raw.githubusercontent.com/brilyyy/omah/master/scripts/install.sh | bash -s -- --source

# Custom install directory (default: ~/.local/bin)
curl -fsSL https://raw.githubusercontent.com/brilyyy/omah/master/scripts/install.sh | bash -s -- --prefix ~/.local/bin
```

### Manual binary

Download the latest release from [GitHub Releases](https://github.com/brilyyy/omah/releases):

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `omah-v*-macos-aarch64.tar.gz` |
| macOS (Intel) | `omah-v*-macos-x86_64.tar.gz` |
| Linux (x86_64) | `omah-v*-linux-x86_64.tar.gz` |

```sh
tar -xzf omah-*.tar.gz
install omah ~/.local/bin/
```

### Build from source

```sh
cargo install --path crates/omah_bin
```

### Verify

```sh
omah --version
omah --help
```

---

## 2. Quick Start

```sh
# 1. Scaffold the config
omah init

# 2. Edit the config
$EDITOR ~/.config/omah/omah.toml
```

Add your dotfiles:

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

```sh
# 3. Back up your dotfiles
omah backup

# 4. Check sync state
omah status

# 5. Restore on another machine
omah restore
```

---

## 3. Configuration

Default location: `~/.config/omah/omah.toml`

Override with `omah -c <path>` (global flag, works with any subcommand).

### Top-level fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `vault_path` | string | — (required) | Directory where dotfile copies are stored |
| `os` | string | `"auto"` | Target OS. `"macos"`, `"linux"`, or `"auto"` for runtime detection |
| `pkg_manager` | string | `"auto"` | Package manager for dependency install commands. `"brew"`, `"apt-get"`, `"pacman"`, `"dnf"`, `"zypper"`, or `"auto"` |

### `[[dots]]` entries

Each dotfile is one table in the `[[dots]]` array.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Label for the dotfile, used as CLI argument and vault subfolder |
| `source` | string | yes | Path to the file or directory on your machine |
| `id` | string | no | Unique identifier for vault directory (auto-assigned on backup if missing) |
| `symlink` | bool | no | If `true`, replaces source with symlink into vault after backup |
| `deps` | string[] | no | Binaries (or packages) that must be installed for this dotfile |
| `exclude` | string[] | no | Glob patterns to skip when backing up a directory |
| `setup` | table[] | no | Shell commands to run after restore, each with an optional check |

### Setup step format

Each entry in `setup` is an inline table with two fields:

```toml
[[dots]]
name = "Tmux"
source = "~/.tmux.conf"
setup = [
  { install = "git clone https://github.com/tmux-plugins/tpm ~/.tmux/plugins/tpm", check = "dir:~/.tmux/plugins/tpm" },
  { install = "~/.tmux/plugins/tpm/bin/install_plugins", check = "skip" },
]
```

### Setup check types

The `check` field determines whether a step is **pending** (needs to run) or **done** (skipped).

| Check | Skipped (done) when… | Example |
|-------|---------------------|---------|
| *(none / empty)* | never (always pending) | — |
| `bin:<name>` | binary exists in `$PATH` | `bin:nvim` |
| `file:<path>` | file exists | `file:~/.config/nvim/init.lua` |
| `dir:<path>` | directory exists | `dir:~/.tmux/plugins/tpm` |
| `app:<name>` | `.app` bundle exists in `/Applications` or `~/Applications` | `app:Kitty` |
| `cmd:<shell>` | shell command exits 0 | `cmd:test -d ~/.config/nvim` |
| `out:<text>` | install command's trimmed stdout equals text | `out:OK` |
| `skip` / `skip:<reason>` | always skipped (user-deferred) | `skip` |
| bare path | path exists | `~/.local/share/nvim/site/autoload` |
| bare word | binary exists in `$PATH` | `git` |

### Full config example

```toml
vault_path = "~/Documents/OmahVault"
os = "auto"
pkg_manager = "auto"

[[dots]]
name = "Zsh"
source = "~/.zshrc"
deps = ["zsh", "starship"]

[[dots]]
name = "Neovim"
source = "~/.config/nvim"
symlink = true
deps = ["nvim", "git", "ripgrep", "fd"]
exclude = ["*.log", ".git", "node_modules"]
setup = [
  { install = "nvim --headless '+Lazy! sync' +qa", check = "dir:~/.local/share/nvim/lazy" },
  { install = "brew install ripgrep fd", check = "bin:rg" },
]

[[dots]]
name = "Tmux"
source = "~/.tmux.conf"
deps = ["tmux"]
setup = [
  { install = "git clone https://github.com/tmux-plugins/tpm ~/.tmux/plugins/tpm", check = "dir:~/.tmux/plugins/tpm" },
  { install = "~/.tmux/plugins/tpm/bin/install_plugins", check = "skip" },
]

[[dots]]
name = "Kitty"
source = "~/.config/kitty"
exclude = ["*.bak"]
```

---

## 4. Command Reference

### `omah init`

Scaffold the config directory and write a default config file.

```sh
omah init
```

Creates `~/.config/omah/omah.toml` with a minimal template. If the file already exists, it is **not** overwritten.

---

### `omah backup [name]`

Copy dotfiles from their source paths into the vault.

```sh
# Back up all dotfiles
omah backup

# Only back up a single dotfile by name
omah backup nvim

# Ignore exclude patterns from config
omah backup --no-exclude

# Preview what would be backed up
omah backup --dry-run
```

Output (multi-file):

```
  ✓ Zsh           backed-up
  ✓ Neovim        backed-up
  ○ Kitty         backed up (symlink)
  ⚠ Tmux          source missing
```

Output (single-file with `--dry-run`):

```
  ✓ Zsh            would back up /Users/you/.zshrc
```

---

### `omah restore [name]`

Copy dotfiles from the vault back to their source paths.

```sh
# Restore all dotfiles
omah restore

# Restore a single dotfile
omah restore nvim

# Preview what would be restored
omah restore --dry-run
```

When a source file already exists, omah prompts for confirmation before overwriting.

---

### `omah status`

Show the sync state of every configured dotfile.

```sh
omah status
```

Output:

```
  ✓ Zsh     deployed
  ✓ Neovim  deployed (symlink)
  ○ Kitty   available in vault
  ⚠ Tmux    unbacked
  ✗ Ghost   missing
```

| State | Meaning |
|-------|---------|
| `deployed` | Source exists and vault copy exists |
| `deployed (symlink)` | Source is a symlink into the vault |
| `available in vault` | Vault copy exists, source doesn't (e.g. new machine before restore) |
| `unbacked` | Source exists, vault copy doesn't |
| `missing` | Neither source nor vault copy exist |

```sh
# Machine-readable output
omah status --json
```

---

### `omah diff`

Show what has changed between each source file and its vault copy.

```sh
omah diff
```

Output groups changes per dotfile:

```
Zsh:
  M .zshrc        (modified)

Neovim:
  A init.lua      (added to vault)
  D old-plugin.lua (removed from vault)
  M lazy-lock.json (modified)
```

```
omah diff --json
```

---

### `omah list`

List all configured dotfiles with their source paths.

```sh
omah list
```

Output:

```
  Zsh     ~/.zshrc
  Neovim  ~/.config/nvim
  Kitty   ~/.config/kitty
```

```sh
omah list --json
```

---

### `omah info [name]`

Show detailed information about a dotfile.

```sh
omah info zsh
```

Output:

```
Zsh
╭─────────┬──────────────────────────────────────────╮
│ source  │ /Users/you/.zshrc                        │
│ vault   │ /Users/you/Documents/OmahVault/Zsh/.zshrc │
│ state   │ ✓ deployed                               │
│ deps    │ zsh (installed) · starship (installed)    │
╰─────────┴──────────────────────────────────────────╯

Neovim
╭─────────┬──────────────────────────────────────────────╮
│ source  │ /Users/you/.config/nvim                       │
│ vault   │ …/OmahVault/abc_Neovim/nvim                   │
│ state   │ ✓ deployed (symlink)                          │
│ files   │ 47 (1.2 MB)                                   │
│ deps    │ nvim (installed) · git (installed) · …        │
│ setup   │ brew install ripgrep fd (done), … (pending)   │
│ exclude │ *.log · .git · node_modules                   │
╰─────────┴──────────────────────────────────────────────╯
```

Omit the name to show info for all dotfiles:

```sh
omah info
```

---

### `omah add <name> <source>`

Add a new dotfile entry to the config.

```sh
omah add nvim ~/.config/nvim
omah add kitty ~/.config/kitty --symlink
```

The `--symlink` flag sets `symlink = true`, so after the next backup the source path is replaced with a symlink into the vault.

This command appends to the config file in place — it does not back up files (run `omah backup` separately).

---

### `omah remove <name>`

Remove a dotfile entry from the config.

```sh
omah remove nvim
```

This only removes the entry from the config file. It does **not** delete the vault copy or the source file.

---

### `omah migrate`

Migrate legacy vault directories (pre-0.3.0) to the ID-based structure.

```sh
omah migrate
```

In 0.3.0+, each dotfile gets a unique ID, and vault directories are renamed from `{name}` to `{id}_{name}`. Run this after upgrading if you have an existing vault.

---

### Global flags

| Flag | Description |
|------|-------------|
| `-c`, `--config <FILE>` | Custom config path (default: `~/.config/omah/omah.toml`) |
| `--help` | Print help |
| `--version` | Print version |

```sh
omah -c ~/dotfiles/my-config.toml status
omah -c /etc/omah/omah.toml backup
```

---

## 5. Workflows

### First-time setup

```sh
omah init                      # create config
$EDITOR ~/.config/omah/omah.toml  # add your dotfiles
omah backup                    # copy to vault
omah status                    # verify everything is synced
```

### Restore on a new machine

```sh
# Install omah (see §1)
# Copy or sync your vault to the new machine, or point vault_path to a shared location
omah restore                   # copy from vault to source paths
omah status                    # verify
```

omah will prompt before overwriting any existing files at the source paths.

### Single dotfile operations

```sh
omah backup zsh                # back up just zsh
omah restore nvim              # restore just neovim
omah info tmux                 # show detail for tmux
```

Great for quick iteration — edit a dotfile, back it up, restore it on another machine.

### Symlink mode

```sh
# Add a dotfile with symlink enabled
omah add nvim ~/.config/nvim --symlink

# Or set it in the config
# [[dots]]
# name = "Neovim"
# source = "~/.config/nvim"
# symlink = true
```

After `omah backup`, the source path becomes a symlink pointing into the vault. The next time you open Neovim, it reads from the vault copy — edits go directly into the vault.

### Exclude patterns

When a dotfile source is a directory, exclude patterns skip matching files:

```toml
exclude = ["*.log", ".git", "node_modules", "*.swp"]
```

Glob patterns are matched against the file name relative to the source directory. Standard glob syntax applies (`*` matches within a single path component, `**` matches across directories).

### Dry-run preview

```sh
omah backup --dry-run
omah restore --dry-run
```

Shows what would be copied without modifying any files. Useful before running a potentially large backup or restore.

### Setup steps with check guards

Setup steps run after `omah restore`. Each step's `check` determines whether it needs to run at all:

```toml
setup = [
  # Only runs if nvim plugin dir doesn't exist
  { install = "nvim --headless '+Lazy! sync' +qa", check = "dir:~/.local/share/nvim/lazy" },
  # Runs every time (no check → always pending)
  { install = "echo 'restore complete'" },
  # Explicitly skipped
  { install = "long-running-setup", check = "skip" },
]
```

During restore, pending steps are shown and the user can choose to run or skip them.

### Migration from legacy vault

If you used omah before version 0.3.0, vault directories used bare dotfile names (`Zsh/`, `Neovim/`). Version 0.3.0+ prefixes vault directories with a unique ID (`abc123_Zsh/`).

```sh
# Check if migration is needed
omah migrate

# If any dotfiles lack IDs, this renames the vault directories
# and updates the config file
```

Migration is safe to run multiple times — it's idempotent.

---

## 6. FAQ / Troubleshooting

### Config file not found

```
Error: Failed to read config file: ~/.config/omah/omah.toml
```

Run `omah init` to scaffold the default config, or pass `-c /path/to/config.toml`.

### Permission errors on backup/restore

omah reads and writes files as the current user. If your dotfiles are owned by root (e.g. `/etc/`), run omah with `sudo` or change ownership:

```sh
sudo omah backup
```

### Path expansion

omah expands `~` in paths using the `expand-tilde` crate, which respects `$HOME`. Environment variables in paths (like `$XDG_CONFIG_HOME`) are **not** expanded — use `~` or absolute paths instead.

### Vault path on a shared drive

Set `vault_path` to a cloud-synced directory (Dropbox, iCloud Drive, Syncthing) to automatically sync your vault across machines:

```toml
vault_path = "~/Library/Mobile Documents/com~apple~CloudDocs/omah-vault"
```

### How do I see what a restore would do without running it?

```sh
omah restore --dry-run
```

### Does omah delete files?

- `omah remove` removes the config entry only — no file deletion.
- `omah restore` overwrites source files (with confirmation prompt) but does not delete files.
- `omah backup` copies files into the vault — it does not delete anything from the vault.

### How do I use omah with a dotfiles repo?

Keep your vault separate from a Git-tracked dotfiles directory. The vault is omah's internal storage — it uses hash-based IDs, symlinks, and a directory structure not designed for direct Git tracking. Instead, point `vault_path` to a cloud-synced folder or set up `omah backup` as a cron job.
