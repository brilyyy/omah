# 🏡 omah

A dotfile manager written in Rust. Backs up and restores your configuration files to a centralized vault.

> **omah** — Javanese for *home*

---

## Installation

Download the latest binary for your platform from the [Releases](../../releases) page:

| Platform | File |
| --- | --- |
| macOS (Apple Silicon) | `omah-macos-aarch64` |
| macOS (Intel) | `omah-macos-x86_64` |
| Linux (x86_64) | `omah-linux-x86_64` |

```sh
chmod +x omah-*
mv omah-* /usr/local/bin/omah
```

Or build from source:

```sh
# CLI only
cargo build --release
cp target/release/omah /usr/local/bin/omah

# CLI + TUI dashboard
cargo build --release --features tui
cp target/release/omah /usr/local/bin/omah
```

---

## Quick start

```sh
omah init       # create ~/.config/omah/omah-config.toml with a default template
# edit the config, then:
omah backup     # copy dotfiles into the vault
omah status     # see what's in sync
omah diff       # show what has changed between source and vault
omah restore    # copy dotfiles back from the vault
```

---

## Config file

Default location: `~/.config/omah/omah-config.toml`. Override with `--config <path>`.

```toml
vault_path = "~/Documents/OmahVault"
git = true                # auto-commit the vault after every backup (optional)
os = "auto"               # "auto" (detect) | "macos" | "linux" — optional
pkg_manager = "auto"      # "auto" (detect) | "brew" | "apt-get" | "pacman" | "dnf" | "zypper"

[[dots]]
name = "Zsh"
source = "~/.zshrc"
deps = ["zsh"]

[[dots]]
name = "Neovim"
source = "~/.config/nvim"
symlink = true                  # backup replaces source with a symlink into the vault
deps = ["nvim", "git", "ripgrep"]
exclude = ["*.log", ".git"]     # glob patterns — skipped when copying the directory
setup = [
  # skipped if ~/.local/share/nvim already exists
  { check = "~/.local/share/nvim", install = "git clone --depth 1 https://github.com/AstroNvim/template ~/.config/nvim" }
]

[[dots]]
name = "Custom"
source = "~/.my-custom-rc"
# no deps or setup = no requirements
```

### Fields

| Field | Type | Description |
| --- | --- | --- |
| `vault_path` | string | Where dotfiles are stored (supports `~`) |
| `git` | bool | When `true`, `omah backup` auto-commits the vault (including config file) after copying |
| `os` | string | Target OS. `"auto"` (default) detects at runtime. Accepts `"macos"` or `"linux"` |
| `pkg_manager` | string | Package manager for installing deps. `"auto"` detects from PATH. Accepts `"brew"`, `"apt-get"`, `"pacman"`, `"dnf"`, `"zypper"` |
| `name` | string | Human-readable label, also used as the vault subfolder name |
| `source` | string | Path to the dotfile/directory on your machine (supports `~`) |
| `symlink` | bool | When `true`, backup moves the source into the vault and leaves a symlink behind |
| `deps` | string[] | Binaries that must be in PATH (checked at restore time) |
| `setup` | array | Shell commands to run before restore; each entry has `install` (required) and optional `check` (path — step is skipped if it already exists) |
| `exclude` | string[] | Glob patterns for files/dirs to skip when copying a source directory (e.g. `*.log`, `.git`) |

Each dotfile is stored at `vault/{name}/{filename}` — the original filename is preserved inside a named folder.

---

## Commands

### `omah init`

Creates `~/.config/omah/` and scaffolds a default `omah-config.toml` if one does not exist. Safe to run multiple times — will not overwrite an existing config.

### `omah backup`

Copies every dotfile in `[[dots]]` from its `source` into the vault, skipping any paths matched by `exclude` patterns.

If `git = true` is set in the config (and `--no-git` is not passed), omah will copy `omah-config.toml` into the vault as `.omah-config.toml` and then auto-commit everything together.

If any dotfile has `symlink = true`, omah will list those entries and ask for confirmation before replacing the source with a symlink:

```text
The following dotfiles will have their source replaced with a symlink:
  - Neovim

Continue? [y/N]
```

**Flags:**

| Flag | Description |
| --- | --- |
| `--no-git` | Skip the git auto-commit even if `git = true` in config |
| `--no-exclude` | Ignore all `exclude` patterns (copy everything) |

### `omah restore`

Copies dotfiles from the vault back to their `source` paths.

Before copying, omah checks for missing deps and pending setup steps across all dotfiles. If anything is needed, it shows a numbered action list and asks once:

```text
The following steps are required before restore:

  [1]  install deps:    brew install nvim git ripgrep
  [2]  setup  Neovim:  git clone --depth 1 https://... ~/.config/nvim

Run all? [y/N]
```

If a vault entry is missing for one dotfile, that entry is skipped with a warning — the rest are still restored.

### `omah diff`

Shows what has changed between your live source files and the vault snapshot:

```text
Zsh:
  ~ .zshrc

Neovim:
  + init.lua
  ~ lua/plugins.lua
  - lua/old-module.lua
```

`+` = added in source (not in vault), `~` = modified, `-` = removed from source (still in vault).

### `omah status`

Shows sync state for every configured dotfile:

```text
Vault: ~/Documents/OmahVault

  Zsh                  ~/.zshrc              backed up
  Neovim               ~/.config/nvim        backed up  [symlinked]
  Custom               ~/.my-custom-rc       NOT backed up
                       missing deps:  curl
                       pending setup: git clone ...
```

### `omah list`

Lists all configured dotfiles with their source paths and symlink flag.

---

## TUI dashboard

Build and run with the `tui` feature:

```sh
cargo build --release --features tui
omah tui
# or with a custom config:
omah --config ~/work/omah.toml tui
```

### Splash screen

A brief animated splash is shown on launch. Any key skips it immediately.

### List screen

The main view shows all configured dotfiles with their current sync status.

```text
┌─ omah ─ vault: ~/Documents/OmahVault ─────────────────────────────────────┐
│ Name                Source                    Status        Flags          │
│ ────────────────────────────────────────────────────────────────────────── │
│ ▶ Zsh               ~/.zshrc                  backed up                    │
│   Neovim            ~/.config/nvim            backed up     symlink        │
│   Custom            ~/.my-custom-rc           not backed up                │
└────────────────────────────────────────────────────────────────────────────┘
 j/k: navigate  e: edit  b: backup  B: backup all  r: restore  R: restore all  n: new  q: quit
```

| Key | Action |
| --- | --- |
| `j` / `↓` | move down |
| `k` / `↑` | move up |
| `e` | open edit screen for selected dotfile |
| `n` | open add-dotfile form |
| `b` | backup selected dotfile |
| `B` | backup all dotfiles |
| `r` | restore selected dotfile (asks for confirmation) |
| `R` | restore all dotfiles (asks for confirmation) |
| `S` | open global settings (OS, package manager) |
| `q` / `Esc` | quit |

### Settings (`S`)

Edit global config fields that apply to all dotfiles:

```text
┌─ omah — settings ──────────────────────────────────────────────────────────┐
│  Global Settings                                                             │
├─ Configuration ─────────────────────────────────────────────────────────────┤
│ ▶ OS               auto                                                      │
│                    values: auto | macos | linux                              │
│   Package Manager  auto                                                      │
│                    values: auto | brew | apt-get | pacman | dnf | zypper     │
└─────────────────────────────────────────────────────────────────────────────┘
  Tab: switch field   s: save   Esc: cancel
```

- `auto` removes the key from the config (falls back to runtime detection).
- Setting an explicit value locks it regardless of what is installed on the system.

| Key | Action |
| --- | --- |
| `Tab` / `Enter` | switch between OS and Package Manager fields |
| type / `Backspace` | edit the active field |
| `s` | save and return to list |
| `Esc` | cancel |

### Add dotfile (`n`)

A modal form for quickly adding a new entry to the config:

| Key | Action |
| --- | --- |
| `Tab` | next field |
| `Shift+Tab` | previous field |
| `Space` | toggle symlink (on the symlink field) |
| `Enter` | advance to next field / save on last field |
| `Esc` | cancel |

### Edit dotfile (`e`)

A full-screen editor for an existing dotfile entry. Changes are saved back to the config file using `toml_edit`, preserving all comments and formatting in the rest of the file.

```text
┌─ omah — edit ──────────────────────────────────────────────────────────────┐
│  Editing: Neovim                                                            │
├─ Fields ───────────────────────────────────────────────────────────────────┤
│ ▶ Name    Neovim                                                            │
│   Source  ~/.config/nvim                                                    │
│   Symlink [x] replace source with symlink                                   │
│   Deps    nvim git ripgrep                                                  │
├─ Setup Steps — [a] add  [d] delete ────────────────────────────────────────┤
│ ▶ check: ~/.local/share/nvim  →  git clone --depth 1 https://... nvim      │
│   (no check)                  →  pip install pynvim                         │
├─ Exclude Patterns — [a] add  [d] delete ───────────────────────────────────┤
│   *.log                                                                     │
│   .git                                                                      │
└────────────────────────────────────────────────────────────────────────────┘
  Tab: switch focus   j/k: navigate   s: save   Esc: cancel
```

**Fields section** (focus 0–3):

| Key | Action |
| --- | --- |
| `Tab` / `Shift+Tab` | cycle between Name → Source → Symlink → Deps → Steps → Excludes |
| type / `Backspace` | edit the active text field |
| `Space` | toggle symlink (when Symlink field is focused) |
| `Enter` | advance focus to next field |

**Setup steps section** (focus 4, reached via `Tab`):

| Key | Action |
| --- | --- |
| `j` / `↓` | select next step |
| `k` / `↑` | select previous step |
| `a` | open add-step form |
| `d` | delete selected step |

**Add-step form** (appears when pressing `a` in the steps section):

| Key | Action |
| --- | --- |
| `Tab` / `Enter` | move from Check field to Install field |
| `Enter` (on Install) | save the step |
| `Esc` | cancel |

The **Check path** field is optional. If filled in, the step is skipped during restore when that path already exists on disk.

**Exclude patterns section** (focus 5, reached via `Tab`):

| Key | Action |
| --- | --- |
| `j` / `↓` | select next pattern |
| `k` / `↑` | select previous pattern |
| `a` | open input to add a new glob pattern |
| `d` | delete selected pattern |

Patterns follow standard glob syntax (e.g. `*.log`, `.git`, `node_modules`). They are matched against filenames when copying a source directory.

**Saving:**

| Key | Action |
| --- | --- |
| `s` | save all changes to the config file |
| `Esc` | cancel and return to the list without saving |

---

## Project structure

```text
crates/
├── omah_structs/   # Core data types (OmahConfig, DotfileConfig, SetupStep)
├── omah_lib/       # Business logic: config loading, backup, restore, status, diff, git
├── omah_core/      # Re-exports omah_lib + omah_structs as a single crate
├── omah_bin/       # CLI entry point (clap)
└── omah_tui/       # Optional TUI dashboard (feature = "tui", ratatui + crossterm)
```

---

## TODO

### Core

- [x] `init` — scaffold default config
- [x] Backup — copy dotfiles into vault
- [x] Restore — copy dotfiles back to source
- [x] Symlink support — backup replaces source with symlink when `symlink = true`
- [x] Restore confirms before overwriting; continues past missing vault entries
- [x] Backup confirms before replacing sources with symlinks
- [x] Exclude patterns — glob-based file filtering during backup
- [x] Git integration — auto-commit vault after backup (`git = true`), includes config file
- [x] Diff — compare source vs vault, show added/modified/removed
- [x] OS and package manager config — explicit override or auto-detect

### CLI

- [x] `init`, `backup`, `restore`, `status`, `list`, `diff` subcommands
- [x] `--config` flag for custom config path
- [x] `--no-git` / `--no-exclude` flags on `backup`
- [x] Error messages with context

### TUI (`--features tui`)

- [x] Animated splash screen
- [x] Dotfile list with live status indicators
- [x] Backup selected / backup all
- [x] Restore selected / restore all (with confirmation dialog)
- [x] Add new dotfile (name, source, symlink)
- [x] Edit existing dotfile (name, source, symlink, deps, setup steps, exclude patterns)
- [x] Config saved with `toml_edit` — preserves comments and formatting
- [x] Settings screen (`S`) — edit OS and package manager globally

### Enhancements

- [ ] `--dry-run` flag — preview operations without touching the filesystem
- [ ] Selective backup — back up a single dotfile by name
- [ ] Multiple profiles — named profiles pointing to different vault paths
- [ ] `omah watch` — monitor source paths and auto-backup on change
- [ ] Shell completion generation (bash, zsh, fish)
