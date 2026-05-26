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
cargo build --release
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
os = "auto"               # "auto" (detect) | "macos" | "linux" — optional
pkg_manager = "auto"      # "auto" (detect) | "brew" | "apt-get" | "pacman" | "dnf" | "zypper"

[[dots]]
name = "Zsh"
source = "~/.zshrc"
deps = ["zsh"]

[[dots]]
name = "Neovim"
source = "~/.config/nvim"
deps = ["nvim", "git", "ripgrep"]
exclude = ["*.log", ".git"]     # glob patterns — skipped when copying the directory
setup = [
  # skipped if ~/.local/share/nvim already exists
  { check = "dir:~/.local/share/nvim", install = "git clone --depth 1 https://github.com/AstroNvim/template ~/.config/nvim" }
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
| `os` | string | Target OS. `"auto"` (default) detects at runtime. Accepts `"macos"` or `"linux"` |
| `pkg_manager` | string | Package manager for installing deps. `"auto"` detects from PATH. Accepts `"brew"`, `"apt-get"`, `"pacman"`, `"dnf"`, `"zypper"` |
| `name` | string | Human-readable label, also used as the vault subfolder name |
| `source` | string | Path to the dotfile/directory on your machine (supports `~`) |
| `symlink` | bool | When `true`, backup moves the source into the vault and leaves a symlink behind |
| `deps` | string[] | Binaries or shell functions that must be available (e.g. `nvm`). Checked via PATH and interactive shell. |
| `setup` | array | Shell commands to run before restore; each entry has `install` (required) and optional `check` (see below) |
| `exclude` | string[] | Glob patterns for files/dirs to skip when copying a source directory (e.g. `*.log`, `.git`) |

#### `check` field

Controls when a setup step is considered done and skipped:

| Value | Meaning |
| --- | --- |
| `bin:<name>` | Binary or shell function must be available (PATH + interactive shell) |
| `file:<path>` | File must exist (supports `~`) |
| `dir:<path>` | Directory must exist (supports `~`) |
| `cmd:<shell>` | Shell snippet must exit 0 |
| `out:<expected>` | Runs the `install` command; done when trimmed stdout equals `expected` |
| `skip` | Always considered done (never runs) |
| bare `nvim` | Backward-compat: binary check |
| bare `/…` or `~/…` | Backward-compat: path existence check |
| missing / empty | Always pending (runs every time) |

Each dotfile is stored at `vault/{name}/{filename}` — the original filename is preserved inside a named folder.

---

## Commands

### `omah init`

Creates `~/.config/omah/` and scaffolds a default `omah-config.toml` if one does not exist. Safe to run multiple times — will not overwrite an existing config.

### `omah backup`

Copies every dotfile in `[[dots]]` from its `source` into the vault, skipping any paths matched by `exclude` patterns.

**Flags:**

| Flag | Description |
| --- | --- |
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

## Development

### Setup

```sh
git clone <repo>
cd omah
bun run hooks   # activate commit-msg hook (enforces Conventional Commits)
```

### Common tasks

| Command | Description |
| --- | --- |
| `bun run cargo:check` | Fast compile check (all workspace crates) |
| `bun run cargo:test` | Run all workspace tests |
| `bun run cargo:lint` | Run Clippy (warnings as errors) |
| `bun run cargo:fmt` | Auto-format Rust code |
| `bun run cargo:build` | Build release binary |
| `bun run cli:install` | Build + copy binary to `/usr/local/bin/omah` |
| `bun run cargo:clean` | Remove build artifacts |
| `bun run dev` | Run Vite dev server (frontend only) |
| `bun run tauri dev` | Run the Tauri desktop app in dev mode |
| `bun run build` | Build frontend for production |
| `bun run tauri build` | Build the Tauri desktop app for release |
| `bun run check` | Biome lint + format check on `src/` |
| `bacon` | Watch mode: re-runs `cargo check` on save |
| `bacon test` | Watch mode: re-runs tests on save |

### Commit messages

Commits must follow [Conventional Commits](https://www.conventionalcommits.org/):

```text
<type>[optional scope]: <description>
```

Allowed types: `feat`, `fix`, `hotfix`, `docs`, `chore`, `refactor`, `test`, `style`, `ci`, `perf`, `build`

```sh
git commit -m "feat: add shell completion generation"
git commit -m "fix(backup): skip unreadable symlink targets"
git commit -m "docs: update README installation section"
```

The `commit-msg` hook validates this automatically after `bun run hooks`.

### CI

| Trigger | Jobs |
| --- | --- |
| Every push (any branch) | `cargo test --workspace --exclude omah_desktop --locked` |
| Tag `v*` on `master`/`main` | Build CLI (3 platforms) + Desktop bundles (3 platforms) → GitHub Release |

### Releasing

Bump the version in `crates/omah_bin/Cargo.toml`, commit, then push a `v*` tag:

```sh
# 1. Edit crates/omah_bin/Cargo.toml  →  version = "1.4.0"
git commit -m "chore: bump version to 1.4.0"
bun run tag   # reads version from Cargo.toml, creates + pushes v1.4.0 tag
```

GitHub Actions builds CLI binaries and desktop bundles for all platforms and publishes a GitHub Release with auto-generated notes.

#### Release targets

| Platform | CLI binary | Desktop bundle |
| --- | --- | --- |
| Linux x86_64 (musl, static) | `omah-v{version}-linux-x86_64.tar.gz` | `omah_{version}_amd64.AppImage` |
| macOS Apple Silicon | `omah-v{version}-macos-aarch64.tar.gz` | `omah_{version}_aarch64.dmg` |
| macOS Intel | `omah-v{version}-macos-x86_64.tar.gz` | `omah_{version}_x64.dmg` |

---

## Project structure

```text
crates/
├── omah_structs/   # Core data types (OmahConfig, DotfileConfig, SetupStep)
├── omah_lib/       # Business logic: config loading, backup, restore, status, diff, deps
├── omah_core/      # Re-exports omah_lib + omah_structs as a single crate
└── omah_bin/       # CLI entry point (clap)

src/                # React frontend (TanStack Router/Query, shadcn/ui)
src-tauri/          # Tauri v2 backend — exposes omah_core via Tauri commands
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

### Desktop app (Tauri)

- [x] v1.0.0 — full visual interface with streaming terminal, batik theme
- [x] Dotfile list with live sync status
- [x] Backup / restore per dotfile or all at once
- [x] Inline diff viewer
- [x] Add / edit dotfile (name, source, symlink, deps, setup steps, exclude patterns)
- [x] Setup step runner with streaming terminal output
- [x] Symlink toggle in dotfile detail view (backend supported; UI temporarily disabled)
- [x] Donation dialog
- [ ] Prebuilt `.dmg` (macOS) and `.AppImage` (Linux) in GitHub Releases
- [ ] Auto-update — notify and apply new releases in-app
- [ ] Tray icon / menubar mode (macOS) — keep omah running in the background
- [ ] Vault browser — explore backed-up files and their history
- [ ] Onboarding wizard — guided first-run setup for new users
- [ ] Drag-and-drop to add dotfiles from Finder / file manager
- [ ] Stale backup notifications — alert when source has changed since last backup

### Enhancements

- [ ] `--dry-run` flag — preview backup/restore operations without touching the filesystem
- [ ] Multiple profiles — named profiles pointing to different vault paths (e.g. work vs personal)
- [ ] `omah watch` — monitor source paths and auto-backup on change using filesystem events
- [ ] Shell completion generation (bash, zsh, fish)
- [ ] Encryption — optionally encrypt sensitive dotfiles (e.g. SSH keys, `.env` files) at rest in the vault
- [ ] Remote vault — push/pull vault to a Git remote, S3 bucket, or rsync target for cross-machine sync
- [ ] `omah import` — bootstrap config from an existing dotfile repository or bare vault directory
- [ ] Config validation — catch invalid paths, missing binaries, and malformed globs before operations run
- [ ] Windows support — native path handling and package manager detection (`winget`, `scoop`, `choco`)
- [ ] Colored diff output in CLI — highlight added/modified/removed lines with ANSI colors
