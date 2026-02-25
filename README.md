# 🏡 omah

A dotfile manager written in Rust. Backs up and restores your configuration files to a centralized vault.

> **omah** — Javanese for *home*

## Usage

```toml
# ~/.config/omah/omah-config.toml

vault_path = "~/Documents/OmahVault"

[[dots]]
name = "Zsh Config"
source = "~/.zshrc"

[[dots]]
name = "Neovim"
source = "~/.config/nvim"
symlink = true
```

## Project Structure

```text
crates/
├── omah_structs/   # Core data structures (OmahConfig, DotfileConfig)
├── omah_lib/       # Config loading, path resolution
└── omah_bin/       # CLI entry point
```

## TODO

### Core

- [x] Implement `init_setup()` — create `~/.config/omah/` and scaffold default config if missing
- [x] Implement backup logic — copy dotfiles from `source` into the vault
- [x] Implement restore logic — copy dotfiles from vault back to `source`
- [x] Implement symlink support — create symlinks from `source` to vault when `symlink = true`

### CLI

- [x] Wire up `clap` for argument parsing
- [x] Add subcommands: `init`, `backup`, `restore`, `status`, `list`
- [x] Load config from default path (`~/.config/omah/omah-config.toml`) instead of hardcoded `./assets/config.toml`
- [x] Add `--config` flag to specify a custom config file path

### Quality

- [x] Add error messages with context (file not found, permission denied, etc.)
- [x] Add a `status` command showing which dotfiles are in sync vs. out of date
- [x] Write unit tests for config loading and path resolution
- [ ] Add CI (GitHub Actions)

### Enhancements

#### Vault

- [ ] Git integration — auto-commit vault changes after each backup with a timestamp message
- [ ] Diff support — show what changed between source and vault before backing up
- [ ] Dry-run mode (`--dry-run`) — preview backup/restore operations without touching the filesystem
- [ ] Selective backup — back up a single dotfile by name instead of all at once

#### Config

- [ ] `exclude` field on `[[dots]]` — glob patterns for files/dirs to skip inside a source directory
- [ ] Multiple profiles — support named profiles (e.g. `work`, `home`) pointing to different vault paths
- [ ] `tags` field on `[[dots]]` — group dotfiles and operate on a subset with `--tag`

#### Watch mode

- [ ] `omah watch` — monitor source paths with `inotify`/`FSEvents` and auto-backup on change

#### Portability

- [ ] Windows symlink support via `std::os::windows::fs::symlink_file` / `symlink_dir`
- [ ] Shell completion generation (`clap` completions for bash, zsh, fish)

## Building

```sh
cargo build
cargo run
```
