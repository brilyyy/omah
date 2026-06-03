# Roadmap

## Core

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

## CLI

- [x] `init`, `backup`, `restore`, `status`, `list`, `diff` subcommands
- [x] `--config` flag for custom config path
- [x] `--no-git` / `--no-exclude` flags on `backup`
- [x] Error messages with context
- [ ] `omah add <name> <source>` / `omah remove <name>` subcommands
- [ ] `--dry-run` flag on backup/restore
- [ ] Per-dotfile granularity: `omah backup [name]` / `omah restore [name]`
- [ ] Shell completion generation (bash, zsh, fish)
- [ ] `--quiet` / `--verbose` flags
- [ ] Colored status output (green/yellow/red labels)
- [ ] Error summary at end of backup/restore

## Desktop app (Tauri)

- [x] v1.0.0 — full visual interface with streaming terminal, batik theme
- [x] Dotfile list with live sync status
- [x] Backup / restore per dotfile or all at once
- [x] Inline diff viewer
- [x] Add / edit dotfile (name, source, symlink, deps, setup steps, exclude patterns)
- [x] Setup step runner with streaming terminal output
- [x] Donation dialog
- [ ] Auto-update — notify and apply new releases in-app
- [ ] Tray icon / menubar mode (macOS)
- [ ] Vault browser — explore backed-up files and their history
- [ ] Onboarding wizard — guided first-run setup for new users
- [ ] Drag-and-drop to add dotfiles from Finder / file manager
- [ ] Stale backup notifications

## Enhancements

- [ ] Multiple profiles — named profiles pointing to different vault paths (e.g. work vs personal)
- [ ] `omah watch` — monitor source paths and auto-backup on change
- [ ] Encryption — optionally encrypt sensitive dotfiles at rest in the vault
- [ ] Remote vault — push/pull vault to a Git remote, S3 bucket, or rsync target
- [ ] `omah import` — bootstrap config from an existing dotfile repository
- [ ] Config validation — catch invalid paths, missing binaries, and malformed globs before operations run
- [ ] Windows support — native path handling and package manager detection (`winget`, `scoop`, `choco`)
- [ ] Colored diff output in CLI
