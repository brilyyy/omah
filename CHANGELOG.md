# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- `--dry-run` flag — preview backup/restore operations without touching the filesystem
- Multiple profiles — named profiles pointing to different vault paths
- `omah watch` — monitor source paths and auto-backup on change
- Shell completion generation (bash, zsh, fish)
- Encryption support for sensitive dotfiles at rest in the vault
- Remote vault — push/pull to a Git remote, S3 bucket, or rsync target
- `omah import` — bootstrap config from an existing dotfile repository
- Windows support (native path handling, winget/scoop/choco detection)
- Desktop: prebuilt `.dmg` and `.AppImage` in GitHub Releases
- Desktop: auto-update — notify and apply new releases in-app
- Desktop: tray icon / menubar mode (macOS)
- Desktop: vault browser — explore backed-up files
- Desktop: drag-and-drop to add dotfiles

---

## [1.0.0] - 2026-03-19

### Added

#### Core library (`omah_lib`)
- `init` / `init_at` — scaffold `~/.config/omah/omah-config.toml` on first run; safe to re-run
- `backup` — copy dotfiles from `source` into the vault, respecting `exclude` glob patterns
- `restore` — copy dotfiles from the vault back to `source`; skips missing entries with a warning
- `diff` — compare live source files against the vault snapshot (added / modified / removed)
- `status` — return per-dotfile sync state (`Vec<DotStatus>`)
- Symlink support — backup replaces `source` with a symlink into the vault when `symlink = true`
- Exclude patterns — glob-based file filtering applied during backup of directory sources
- Git integration — auto-commit the vault (including config) after every backup when `git = true`
- OS and package manager detection — `auto` mode at runtime; explicit override via config
- Deps checking — verify required binaries are in `PATH` before restore
- Setup steps — per-dotfile shell commands run before restore, with optional `check` path to skip if already present
- Expanded `PKG_TO_BIN` lookup table with 100+ package-name → binary-name mappings covering editors, shell tools, TUI apps, and language runtimes

#### CLI (`omah_bin`)
- `omah init` — scaffold config directory and default config file
- `omah backup` — back up all configured dotfiles; `--no-git` and `--no-exclude` flags
- `omah restore` — restore all dotfiles; prompts for deps/setup before proceeding
- `omah status` — show sync state for every configured dotfile
- `omah list` — list all configured dotfiles with source paths and flags
- `omah diff` — show added / modified / removed files vs the vault
- `--config <path>` — override the default config location globally

#### TUI dashboard (`--features tui`, ratatui + crossterm)
- Animated splash screen on launch (any key to skip)
- Dotfile list with live status indicators
- Backup selected dotfile or all at once (`b` / `B`)
- Restore selected dotfile or all with confirmation dialog (`r` / `R`)
- Add new dotfile via modal form (`n`)
- Edit existing dotfile — name, source, symlink, deps, setup steps, exclude patterns (`e`)
- Config written back with `toml_edit` — preserves comments and original formatting
- Settings screen (`S`) — edit OS and package manager globally

#### Desktop app (`apps/desktop`, Tauri v2)
- Full visual interface with batik-themed dark UI
- Dotfile list with live sync status badges
- Backup / restore per dotfile or all at once with toast notifications
- Inline diff viewer — shows added / modified / removed files per dotfile
- Dotfile detail view with setup step runner and streaming terminal output
- Symlink toggle in the dotfile detail view
- Add / edit dotfile dialog — name, source, symlink, deps, setup steps, exclude patterns
- Donation dialog
- Reusable React hooks: `use-config`, `use-status`, `use-backup-restore`, `use-diff`, `use-symlink-mutation`, `use-streaming-terminal`, `use-delete-dotfile`
- Centralized query key factory (`query-keys.ts`) for consistent cache invalidation

#### Tooling & CI
- GitHub Actions CI — `cargo test --workspace` and `cargo build --features tui` on every push and PR
- GitHub Actions release — builds CLI binaries for Linux x86_64 (musl), macOS arm64, and macOS x86_64; publishes a GitHub Release on `v*` tags
- Auto-release workflow — detects version bumps on `master` pushes and creates releases automatically
- `install.sh` one-liner installer — prebuilt binary download or build-from-source (CLI and Desktop)
- Git commit-msg hook enforcing [Conventional Commits](https://www.conventionalcommits.org/)
- Root `package.json` scripts for all common Cargo and Tauri tasks

[Unreleased]: https://github.com/brilyyy/omah/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/brilyyy/omah/releases/tag/v1.0.0
