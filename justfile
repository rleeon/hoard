# Hoard — task runner
#
# `just` is a `make`-alike with saner syntax (https://github.com/casey/just).
# Install: `cargo install just` or `brew install just`.
#
# Every recipe here is also documented in docs/dev.md. The two should not
# drift — if you add a recipe, mention it in dev.md.

set shell := ["bash", "-cu"]

# Repo paths used in multiple recipes.
ui_dir := "crates/hoard-desktop/ui"

# Default target: show the list of recipes.
default:
    @just --list

# ----- Day-to-day -----------------------------------------------------------

# Run the desktop app in dev mode (Tauri + Vite HMR).
dev:
    pnpm --dir {{ui_dir}} tauri dev

# Run only the UI dev server (no Tauri shell). Handy for pure CSS / Svelte work.
ui:
    pnpm --dir {{ui_dir}} dev

# Build the production desktop bundle for the current platform.
build:
    pnpm --dir {{ui_dir}} tauri build

# ----- Verification (mirror of CI) ------------------------------------------

# Fast pre-commit gate: format check + clippy + svelte-check. Run before pushing.
check: fmt-check clippy ui-check

# Run cargo fmt across the workspace.
fmt:
    cargo fmt --all

# Verify formatting without modifying files (CI mode).
fmt-check:
    cargo fmt --all -- --check

# Workspace clippy with warnings denied — same flag CI uses.
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# svelte-check on the desktop UI. Catches type + template errors.
ui-check:
    pnpm --dir {{ui_dir}} check

# Run cargo tests across the workspace.
test:
    SQLX_OFFLINE=true cargo test --workspace

# Compile-only check (no tests). Quick way to validate after a refactor.
compile:
    cargo check --workspace

# i18n locales lint: every key in en.json must exist in the other 7 locales,
# every locale file must parse as JSON, and unused keys are flagged.
i18n-check:
    node scripts/check-i18n.mjs

# ----- Release helpers ------------------------------------------------------

# Regenerate the SQLx offline cache. Needs a running server schema at the
# DATABASE_URL you pass in.
sqlx-prepare DATABASE_URL:
    DATABASE_URL="{{DATABASE_URL}}" cargo sqlx prepare --workspace

# Install git hooks from .githooks/ into .git/hooks/. Run once after clone.
install-hooks:
    git config core.hooksPath .githooks
    @echo "hooks installed (core.hooksPath -> .githooks)"

# Clean the heavy target/ dir. Useful when cargo complains about disk space.
clean:
    cargo clean
    rm -rf {{ui_dir}}/dist {{ui_dir}}/node_modules/.vite
