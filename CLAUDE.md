# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```sh
# Build
cargo build
cargo build --release

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

This is a Cargo workspace with three crates under `crates/` and a Tauri desktop app:

```
omah_structs  →  omah_lib  →  omah_bin (binary: omah)
                          →  omah_core (re-exports for desktop)

src-tauri     →  Tauri v2 desktop app
```

**`omah_structs`** — pure data types, no logic. Defines `OmahConfig` (top-level config with `vault_path` and a `dots` array) and `DotfileConfig` (per-dotfile entry with `name`, `source`, optional `symlink`). Both derive `Serialize`/`Deserialize`.

**`omah_lib`** — all business logic, split across three modules:

- `config` — TOML loading (`load_toml_config`), default path resolution (`get_default_config_path` → `~/.config/omah/omah-config.toml`), and `init_setup` / `init_at` for scaffolding the config directory on first run.
- `ops` — filesystem operations: `backup` (copies source → vault, then optionally replaces source with a symlink), `restore` (copies vault → source or re-creates symlink), and `status` (returns `Vec<DotStatus>` describing sync state per dotfile).
- `constants` — `DEFAULT_CONFIG_DIR`, `DEFAULT_CONFIG_FILE`, `DEFAULT_VAULT_PATH`.

**`omah_bin`** — thin CLI layer using `clap`. `cli.rs` defines the `Cli` struct and `Commands` enum (`init`, `backup`, `restore`, `status`, `list`, `diff`). Each command lives in its own file under `commands/` and delegates immediately to `omah_lib`.

**`src-tauri`** — Tauri v2 desktop GUI. Frontend (React + TanStack Router/Query + shadcn/ui) lives at repo root alongside `src-tauri/`. Backend exposes `omah_core` via Tauri commands with streaming terminal support.

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

Tags matching `v*` trigger the CI release workflow, which builds CLI binaries for `linux-x86_64` (musl), `macos-aarch64`, and `macos-x86_64`, and desktop bundles (`.dmg` for macOS, `.AppImage` for Linux), then publishes a GitHub Release with auto-generated notes.

<!-- rtk-instructions v2 -->
# RTK (Rust Token Killer) - Token-Optimized Commands

## Golden Rule

**Always prefix commands with `rtk`**. If RTK has a dedicated filter, it uses it. If not, it passes through unchanged. This means RTK is always safe to use.

**Important**: Even in command chains with `&&`, use `rtk`:
```bash
# ❌ Wrong
git add . && git commit -m "msg" && git push

# ✅ Correct
rtk git add . && rtk git commit -m "msg" && rtk git push
```

## RTK Commands by Workflow

### Build & Compile (80-90% savings)
```bash
rtk cargo build         # Cargo build output
rtk cargo check         # Cargo check output
rtk cargo clippy        # Clippy warnings grouped by file (80%)
rtk tsc                 # TypeScript errors grouped by file/code (83%)
rtk lint                # ESLint/Biome violations grouped (84%)
rtk prettier --check    # Files needing format only (70%)
rtk next build          # Next.js build with route metrics (87%)
```

### Test (60-99% savings)
```bash
rtk cargo test          # Cargo test failures only (90%)
rtk go test             # Go test failures only (90%)
rtk jest                # Jest failures only (99.5%)
rtk vitest              # Vitest failures only (99.5%)
rtk playwright test     # Playwright failures only (94%)
rtk pytest              # Python test failures only (90%)
rtk rake test           # Ruby test failures only (90%)
rtk rspec               # RSpec test failures only (60%)
rtk test <cmd>          # Generic test wrapper - failures only
```

### Git (59-80% savings)
```bash
rtk git status          # Compact status
rtk git log             # Compact log (works with all git flags)
rtk git diff            # Compact diff (80%)
rtk git show            # Compact show (80%)
rtk git add             # Ultra-compact confirmations (59%)
rtk git commit          # Ultra-compact confirmations (59%)
rtk git push            # Ultra-compact confirmations
rtk git pull            # Ultra-compact confirmations
rtk git branch          # Compact branch list
rtk git fetch           # Compact fetch
rtk git stash           # Compact stash
rtk git worktree        # Compact worktree
```

Note: Git passthrough works for ALL subcommands, even those not explicitly listed.

### GitHub (26-87% savings)
```bash
rtk gh pr view <num>    # Compact PR view (87%)
rtk gh pr checks        # Compact PR checks (79%)
rtk gh run list         # Compact workflow runs (82%)
rtk gh issue list       # Compact issue list (80%)
rtk gh api              # Compact API responses (26%)
```

### JavaScript/TypeScript Tooling (70-90% savings)
```bash
rtk pnpm list           # Compact dependency tree (70%)
rtk pnpm outdated       # Compact outdated packages (80%)
rtk pnpm install        # Compact install output (90%)
rtk npm run <script>    # Compact npm script output
rtk npx <cmd>           # Compact npx command output
rtk prisma              # Prisma without ASCII art (88%)
```

### Files & Search (60-75% savings)
```bash
rtk ls <path>           # Tree format, compact (65%)
rtk read <file>         # Code reading with filtering (60%)
rtk grep <pattern>      # Search grouped by file (75%). Format flags (-c, -l, -L, -o, -Z) run raw.
rtk find <pattern>      # Find grouped by directory (70%)
```

### Analysis & Debug (70-90% savings)
```bash
rtk err <cmd>           # Filter errors only from any command
rtk log <file>          # Deduplicated logs with counts
rtk json <file>         # JSON structure without values
rtk deps                # Dependency overview
rtk env                 # Environment variables compact
rtk summary <cmd>       # Smart summary of command output
rtk diff                # Ultra-compact diffs
```

### Infrastructure (85% savings)
```bash
rtk docker ps           # Compact container list
rtk docker images       # Compact image list
rtk docker logs <c>     # Deduplicated logs
rtk kubectl get         # Compact resource list
rtk kubectl logs        # Deduplicated pod logs
```

### Network (65-70% savings)
```bash
rtk curl <url>          # Compact HTTP responses (70%)
rtk wget <url>          # Compact download output (65%)
```

### Meta Commands
```bash
rtk gain                # View token savings statistics
rtk gain --history      # View command history with savings
rtk discover            # Analyze Claude Code sessions for missed RTK usage
rtk proxy <cmd>         # Run command without filtering (for debugging)
rtk init                # Add RTK instructions to CLAUDE.md
rtk init --global       # Add RTK to ~/.claude/CLAUDE.md
```

## Token Savings Overview

| Category | Commands | Typical Savings |
|----------|----------|-----------------|
| Tests | vitest, playwright, cargo test | 90-99% |
| Build | next, tsc, lint, prettier | 70-87% |
| Git | status, log, diff, add, commit | 59-80% |
| GitHub | gh pr, gh run, gh issue | 26-87% |
| Package Managers | pnpm, npm, npx | 70-90% |
| Files | ls, read, grep, find | 60-75% |
| Infrastructure | docker, kubectl | 85% |
| Network | curl, wget | 65-70% |

Overall average: **60-90% token reduction** on common development operations.
<!-- /rtk-instructions -->