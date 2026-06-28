# Roadmap

## Core

- [x] `init` — scaffold default config
- [x] Backup — copy dotfiles into vault
- [x] Restore — copy dotfiles back to source
- [x] Symlink support — backup replaces source with symlink when `symlink = true`
- [x] Restore confirms before overwriting; continues past missing vault entries
- [x] Backup confirms before replacing sources with symlinks
- [x] Exclude patterns — glob-based file filtering during backup
- [ ] Git integration — auto-commit vault after backup (`git = true`), includes config file
- [x] Diff — compare source vs vault, show added/modified/removed
- [x] OS and package manager config — explicit override or auto-detect

## CLI

- [x] `init`, `backup`, `restore`, `status`, `list`, `info`, `diff` subcommands
- [x] `--config` flag for custom config path
- [x] `--no-exclude` flag on `backup`
- [x] Error messages with context
- [x] `omah add <name> <source>` / `omah remove <name>` subcommands
- [x] `--dry-run` flag on backup/restore
- [x] Per-dotfile granularity: `omah backup [name]` / `omah restore [name]`
- [x] Colored status output (green/yellow/red labels)
- [ ] Shell completion generation (bash, zsh, fish)
- [ ] `--quiet` / `--verbose` flags
- [ ] Error summary at end of backup/restore

## Enhancements

- [ ] Multiple profiles — named profiles pointing to different vault paths (e.g. work vs personal)
- [ ] `omah watch` — monitor source paths and auto-backup on change
- [ ] Encryption — optionally encrypt sensitive dotfiles at rest in the vault
- [ ] `omah import` — bootstrap config from an existing dotfile repository
- [ ] Config validation — catch invalid paths, missing binaries, and malformed globs before operations run
- [ ] Colored diff output in CLI
