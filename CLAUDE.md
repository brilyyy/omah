# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```sh
# Build
cargo build
cargo build --features tui          # include the optional TUI feature

# Run
cargo run -- <subcommand>           # e.g. cargo run -- init
cargo run -- --config path/to/config.toml backup

# Test
cargo test --workspace              # all tests
cargo test -p omah_lib              # single crate
cargo test -p omah_lib config::tests::test_init_at_creates_dir_and_file  # single test

# Lint
cargo clippy --all-targets
cargo fmt --check

# Watch (bacon)
bacon                               # default: cargo check
bacon test                          # watch and re-run tests
bacon clippy-all                    # watch with clippy on all targets (also bound to 'c' in bacon)
```

## Architecture

This is a Cargo workspace with four crates under `crates/`:

```
omah_structs  →  omah_lib  →  omah_bin (binary: omah)
                          →  omah_tui  (optional, feature = "tui")
```

**`omah_structs`** — pure data types, no logic. Defines `OmahConfig` (top-level config with `vault_path` and a `dots` array) and `DotfileConfig` (per-dotfile entry with `name`, `source`, optional `symlink`). Both derive `Serialize`/`Deserialize`.

**`omah_lib`** — all business logic, split across three modules:
- `config` — TOML loading (`load_toml_config`), default path resolution (`get_default_config_path` → `~/.config/omah/omah-config.toml`), and `init_setup` / `init_at` for scaffolding the config directory on first run.
- `ops` — filesystem operations: `backup` (copies source → vault, then optionally replaces source with a symlink), `restore` (copies vault → source or re-creates symlink), and `status` (returns `Vec<DotStatus>` describing sync state per dotfile).
- `constants` — `DEFAULT_CONFIG_DIR`, `DEFAULT_CONFIG_FILE`, `DEFAULT_VAULT_PATH`.

**`omah_bin`** — thin CLI layer using `clap`. `cli.rs` defines the `Cli` struct and `Commands` enum (`init`, `backup`, `restore`, `status`, `list`; `tui` gated on `#[cfg(feature = "tui")]`). Each command lives in its own file under `commands/` and delegates immediately to `omah_lib`.

**`omah_tui`** — stub crate using `ratatui` + `crossterm`; the `run()` function is a `todo!()` placeholder.

## Config file

Default location: `~/.config/omah/omah-config.toml`. Override with `--config <path>`.

```toml
vault_path = "~/Documents/OmahVault"

[[dots]]
name = "Zsh Config"
source = "~/.zshrc"

[[dots]]
name = "Neovim"
source = "~/.config/nvim"
symlink = true   # backup moves source into vault and replaces it with a symlink
```

## Releasing

Tags matching `v*` trigger the CI release workflow, which builds for `linux-x86_64` (musl), `macos-aarch64`, and `macos-x86_64` with `--features tui --release`, then publishes a GitHub Release with auto-generated notes.
