# Common Scrald tasks. Run `just` to list them, `just <recipe>` to run one.
# Recipes are plain cargo/npm commands, so they work in PowerShell on Windows
# and in sh on macOS and Linux.

set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

# Executable produced by `just build` (PowerShell finds scrald.exe from this too).
release_exe := "./target/release/scrald"

# List recipes
default:
    @just --list --unsorted

# --- Setup -----------------------------------------------------------------

# Install frontend dependencies (root npm workspace, includes ui/)
install:
    npm install

# --- Run the app -----------------------------------------------------------

# Run the app in dev mode with no document
dev:
    npm run tauri dev

# Run the app in dev mode with a Markdown file
open file:
    npm run tauri -- dev -- -- "{{ absolute_path(file) }}"

# Write the generated 100k/250k/500k-word fixtures to target/fixtures/
fixtures:
    cargo run -p scrald-core --example generate_fixtures

# Open a generated large fixture in dev mode: just large 250k
large size="100k": fixtures
    npm run tauri -- dev -- -- "{{ absolute_path('target/fixtures/large-' + size + '.md') }}"

# --- Tests and checks ------------------------------------------------------

# Run all tests (Rust and frontend)
test: test-rust test-ui

# Run Rust tests
test-rust:
    cargo test --workspace

# Run frontend tests
test-ui:
    npm test

# Format Rust code
fmt:
    cargo fmt --all

# Lint Rust code (warnings are errors, as in CI)
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Typecheck the frontend
typecheck:
    npm run check

# Everything CI checks, plus the frontend: run before committing
check: typecheck
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    npm test

# Re-run snapshot tests, writing new snapshots in place; review with `git diff`
snapshots $INSTA_UPDATE="always":
    cargo test -p scrald-core --test snapshots

# Run the parse benchmarks
bench:
    cargo bench -p scrald-core

# --- Release builds --------------------------------------------------------

# Build an optimized app without installers
build:
    npm run tauri -- build --no-bundle

# Build the app and its installers
bundle:
    npm run tauri build

# Run the release build on a file, logging to a file (release builds have no console on Windows)
run-release file $SCRALD_LOG_FILE="target/scrald.log": build
    {{ release_exe }} "{{ absolute_path(file) }}"
