.PHONY: all build build-cli test lint fmt fmt-check check install uninstall clean hooks

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

# ── Hooks ──────────────────────────────────────────────────────────────────

## Configure git to use hooks from .githooks/
hooks:
	git config core.hooksPath .githooks
	@echo "Git hooks enabled (core.hooksPath = .githooks)"

# ── Clean ──────────────────────────────────────────────────────────────────

## Remove build artefacts
clean:
	cargo clean
