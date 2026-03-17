.PHONY: all build build-cli test lint fmt fmt-check check install uninstall clean hooks desktop desktop-build

BIN        := target/release/omah
INSTALL    := /usr/local/bin/omah

# ── Default ────────────────────────────────────────────────────────────────

all: build

# ── Build ──────────────────────────────────────────────────────────────────

## Build release binary with TUI dashboard (default)
build:
	cargo build --release --features tui

## Build release binary without TUI (CLI only)
build-cli:
	cargo build --release

# ── Quality ────────────────────────────────────────────────────────────────

## Run all tests
test:
	cargo test --workspace

## Run Clippy linter (warnings become errors)
lint:
	cargo clippy --all-targets --features tui -- -D warnings

## Check code without emitting artifacts (fast)
check:
	cargo check --workspace --features tui

## Apply rustfmt to all code
fmt:
	cargo fmt

## Check formatting without modifying files
fmt-check:
	cargo fmt --check

# ── Install ────────────────────────────────────────────────────────────────

## Install binary to /usr/local/bin (builds with TUI if not already built)
install: build
	cp $(BIN) $(INSTALL)
	@echo "Installed → $(INSTALL)"

## Remove installed binary
uninstall:
	rm -f $(INSTALL)
	@echo "Removed $(INSTALL)"

# ── Release ────────────────────────────────────────────────────────────────

## Create and push a git tag from the version in crates/omah_bin/Cargo.toml
tag:
	@VERSION=$$(grep -m1 '^version' crates/omah_bin/Cargo.toml | sed 's/.*"\(.*\)".*/\1/'); \
	echo "Tagging v$$VERSION ..."; \
	git tag "v$$VERSION" && git push origin "v$$VERSION"; \
	echo "Pushed tag v$$VERSION → triggers release workflow"

# ── Hooks ──────────────────────────────────────────────────────────────────

## Configure git to use hooks from .githooks/
hooks:
	git config core.hooksPath .githooks
	@echo "Git hooks enabled (core.hooksPath = .githooks)"

# ── Desktop (Tauri) ────────────────────────────────────────────────────────

## Run the Tauri desktop app in development mode
desktop:
	bun run desktop

## Build the Tauri desktop app for release
desktop-build:
	bun run desktop:build

# ── Clean ──────────────────────────────────────────────────────────────────

## Remove build artefacts
clean:
	cargo clean
