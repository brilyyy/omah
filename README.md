# omah

A dotfile manager written in Rust with a Tauri desktop app.

> **omah** — Javanese for *home*

Back up your configuration files to a centralized vault. Restore them on any machine with one command.

---

## Install

### Download binary

Grab the latest release from the [Releases](../../releases) page:

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `omah-macos-aarch64` |
| macOS (Intel) | `omah-macos-x86_64` |
| Linux (x86_64) | `omah-linux-x86_64` |

```sh
chmod +x omah-*
mv omah-* /usr/local/bin/omah
```

### Build from source

```sh
cargo build --release
cp target/release/omah /usr/local/bin/omah
```

---

## Quick start

```sh
omah init       # create ~/.config/omah/omah-config.toml
# edit the config to add your dotfiles, then:
omah backup     # copy dotfiles into the vault
omah status     # see what's in sync
omah diff       # show what changed since last backup
omah restore    # copy dotfiles back from the vault
```

---

## Config

Default location: `~/.config/omah/omah-config.toml`. Override with `--config <path>`.

```toml
vault_path = "~/Documents/OmahVault"
os = "auto"            # "auto" | "macos" | "linux"
pkg_manager = "auto"   # "auto" | "brew" | "apt-get" | "pacman" | "dnf" | "zypper"

[[dots]]
name = "Zsh"
source = "~/.zshrc"
deps = ["zsh"]

[[dots]]
name = "Neovim"
source = "~/.config/nvim"
deps = ["nvim", "git", "ripgrep"]
exclude = ["*.log", ".git"]
setup = [
  { check = "dir:~/.local/share/nvim", install = "git clone --depth 1 https://github.com/AstroNvim/template ~/.config/nvim" }
]
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `vault_path` | string | Where dotfiles are stored (supports `~`) |
| `os` | string | Target OS — `"auto"` detects at runtime |
| `pkg_manager` | string | Package manager for installing deps — `"auto"` detects from PATH |
| `name` | string | Label for the dotfile; also used as the vault subfolder name |
| `source` | string | Path to the dotfile or directory on your machine (supports `~`) |
| `symlink` | bool | When `true`, backup moves the source into the vault and leaves a symlink |
| `deps` | string[] | Binaries or shell functions that must be available (checked via PATH and interactive shell) |
| `setup` | array | Shell commands to run after restore — each entry has `install` (required) and optional `check` |
| `exclude` | string[] | Glob patterns for files/dirs to skip when copying a source directory |

### Setup step `check` values

Controls when a setup step is considered done and skipped:

| Value | Skipped when |
|-------|-------------|
| `bin:<name>` | Binary or shell function is in PATH |
| `file:<path>` | File exists |
| `dir:<path>` | Directory exists |
| `cmd:<shell>` | Shell command exits 0 |
| `out:<expected>` | `install` command's stdout matches `expected` |
| `skip` | Always skipped |
| *(empty)* | Never skipped — runs every time |

Each dotfile is stored at `vault/{name}/{filename}`.

---

## Commands

### `omah init`

Creates `~/.config/omah/` and scaffolds a default `omah-config.toml`. Safe to run multiple times — will not overwrite an existing config.

### `omah backup`

Copies every configured dotfile from its `source` into the vault.

| Flag | Description |
|------|-------------|
| `--no-exclude` | Ignore all `exclude` patterns |

### `omah restore`

Copies dotfiles from the vault back to their `source` paths.

Before copying, omah checks for missing deps and pending setup steps. If anything is needed, it shows a numbered action list and asks for confirmation:

```
The following steps are required before restore:

  [1]  install deps:    brew install nvim git ripgrep
  [2]  setup  Neovim:  git clone --depth 1 https://... ~/.config/nvim

Run all? [y/N]
```

### `omah diff`

Shows what has changed between your live source files and the vault snapshot:

```
Zsh:
  ~ .zshrc

Neovim:
  + init.lua
  ~ lua/plugins.lua
  - lua/old-module.lua
```

`+` added in source, `~` modified, `-` removed from source (still in vault).

### `omah status`

Shows sync state for every configured dotfile:

```
Vault: ~/Documents/OmahVault

  Zsh       ~/.zshrc              backed up
  Neovim    ~/.config/nvim        backed up  [symlinked]
  Custom    ~/.my-custom-rc       NOT backed up
            missing deps:  curl
            pending setup: git clone ...
```

### `omah list`

Lists all configured dotfiles with their source paths and symlink flag.

---

## Desktop app

Download `.dmg` (macOS) or `.AppImage` (Linux) from the [Releases](../../releases) page.

The desktop app provides a visual interface for everything the CLI does — plus streaming terminal output for setup steps and an inline diff viewer.

---

## Development

### Setup

```sh
git clone <repo>
cd omah
bun run hooks   # activate commit-msg hook (enforces Conventional Commits)
```

### Commands

| Command | Description |
|---------|-------------|
| `bun run cargo:check` | Fast compile check |
| `bun run cargo:test` | Run all workspace tests |
| `bun run cargo:lint` | Clippy (warnings as errors) |
| `bun run cargo:fmt` | Auto-format Rust code |
| `bun run cargo:build` | Build release binary |
| `bun run cli:install` | Build + copy binary to `/usr/local/bin/omah` |
| `bun run dev` | Vite dev server (frontend only) |
| `bun run tauri dev` | Tauri desktop app in dev mode |
| `bun run build` | Build frontend for production |
| `bun run tauri build` | Build desktop app for release |
| `bun run check` | Biome lint + format check on `src/` |
| `bacon` | Watch: re-runs `cargo check` on save |
| `bacon test` | Watch: re-runs tests on save |

### Commit messages

Follows [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add shell completion generation
fix(backup): skip unreadable symlink targets
docs: update README installation section
```

Allowed types: `feat`, `fix`, `hotfix`, `docs`, `chore`, `refactor`, `test`, `style`, `ci`, `perf`, `build`

The `commit-msg` hook validates this after `bun run hooks`.

### CI

| Trigger | Jobs |
|---------|------|
| Every push | `cargo test --workspace --exclude omah_desktop --locked` |
| Tag `v*` on `master`/`main` | Build CLI (3 platforms) + Desktop bundles (3 platforms) → GitHub Release |

### Releasing

```sh
# 1. Bump version in crates/omah_bin/Cargo.toml
git commit -m "chore: bump version to 1.4.0"
bun run tag   # reads version from Cargo.toml, creates + pushes tag
```

#### Release targets

| Platform | CLI binary | Desktop bundle |
|----------|------------|----------------|
| Linux x86_64 (musl) | `omah-v{ver}-linux-x86_64.tar.gz` | `omah_{ver}_amd64.AppImage` |
| macOS Apple Silicon | `omah-v{ver}-macos-aarch64.tar.gz` | `omah_{ver}_aarch64.dmg` |
| macOS Intel | `omah-v{ver}-macos-x86_64.tar.gz` | `omah_{ver}_x64.dmg` |

---

## Project structure

```
crates/
├── omah_structs/   # Core data types
├── omah_lib/       # Business logic: config, backup, restore, status, diff, deps
├── omah_core/      # Re-exports omah_lib + omah_structs
└── omah_bin/       # CLI entry point (clap)

src/                # React frontend (TanStack Router/Query, shadcn/ui)
src-tauri/          # Tauri v2 backend — exposes omah_core via Tauri commands
```

---

## docs/

- [TODO.md](docs/TODO.md) — feature roadmap
- [PLAN.md](docs/PLAN.md) — UX improvement plan
