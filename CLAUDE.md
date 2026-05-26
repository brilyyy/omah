# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```sh
# Build
cargo build
cargo build --release

# Run CLI
cargo run -- <subcommand>           # e.g. cargo run -- init
cargo run -- --config path/to/config.toml backup

# Run desktop (dev)
bun run dev                         # starts Vite dev server
bun tauri dev                       # launches Tauri app (runs Vite internally)

# Test
cargo test --workspace --exclude omah_desktop   # all lib/bin tests
cargo test -p omah_lib                          # single crate
cargo test -p omah_lib config::tests::test_init_at_creates_dir_and_file

# Lint
cargo clippy --all-targets
cargo fmt --check

# Watch (bacon)
bacon                               # default: cargo check
bacon test                          # watch and re-run tests
bacon clippy-all                    # watch with clippy on all targets (also bound to 'c' in bacon)
```

## Architecture

Cargo workspace with four crates under `crates/` plus a Tauri desktop app at repo root:

```
omah_structs  →  omah_lib  →  omah_bin (binary: omah)
                          →  omah_core (re-exports for desktop)

src-tauri     →  Tauri v2 desktop backend
src/          →  React frontend (TanStack Router/Query, shadcn/ui)
```

**`omah_structs`** — pure data types. `OmahConfig` (`vault_path`, `os`, `pkg_manager`, `dots[]`) and `DotfileConfig` (`name`, `source`, `symlink`, `deps`, `setup`, `exclude`). Both derive `Serialize`/`Deserialize`.

**`omah_lib`** — all business logic:

- `config` — TOML load/save, default path (`~/.config/omah/omah-config.toml`), `init_setup` / `init_at` (auto-creates config on first run).
- `ops` — `backup`, `restore`, `status`, `diff`.
- `deps` — `is_installed` (checks PATH + interactive shell for functions like `nvm`), `missing_deps`, `pending_setup_steps`, `install_command`.
- `constants` — default paths.

**`omah_bin`** — thin CLI (clap). Commands: `init`, `backup`, `restore`, `status`, `list`, `diff`, `add`, `remove`.

**`src-tauri`** — Tauri v2 backend. Exposes `omah_core` via Tauri commands with streaming terminal output. Auto-inits config on first launch.

## Config file

Default location: `~/.config/omah/omah-config.toml`. Auto-created on first run.

```toml
vault_path = "~/Documents/OmahVault"
os = "auto"               # optional: "auto" | "macos" | "linux"
pkg_manager = "auto"      # optional: "auto" | "brew" | "apt-get" | "pacman" | "dnf" | "zypper"

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
  { install = "git clone ...", check = "dir:~/.local/share/nvim" }
]
```

### Setup step check types

| Value | Meaning |
|---|---|
| `bin:nvim` | binary or shell function in PATH (e.g. `bin:nvm`) |
| `file:~/.zshrc` | file must exist |
| `dir:~/.config/nvim` | directory must exist |
| `cmd:ls ... \| grep x` | shell command must exit 0 |
| `out:ok` | runs the install command; done when trimmed stdout == `ok` |
| `skip` | permanently mark as done |
| bare path (`~/…`, `/…`) | backward-compat path existence |
| bare name | backward-compat binary check |

## Releasing

Push a `v*` tag to master/main to trigger the release workflow:

```sh
bun run tag   # reads version from crates/omah_bin/Cargo.toml, creates tag, pushes
```

Builds CLI binaries (`linux-x86_64` musl, `macos-aarch64`, `macos-x86_64`) and desktop bundles (`.dmg`, `.AppImage`), then publishes a GitHub Release.

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
