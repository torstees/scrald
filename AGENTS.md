# AGENTS.md

Guidance for coding agents (Claude Code and others) working on Scrald. Read `DESIGN.md` for architecture and decisions, and the GitHub issues (indexed in `TODO.md`) for the current build order.

## Project in one paragraph

Scrald is a Tauri 2 desktop Markdown reader with lightweight editing. A Rust workspace holds `scrald-core` (pure logic: parsing, flavors, transforms, rendering, themes, editing) and `scrald-app` (the thin Tauri layer: commands, custom protocols, persistence, file watching). The frontend in `ui/` is Svelte 5 + TypeScript + Vite. The headline requirement is that 100K+ word documents open fast and scroll smoothly.

## The human you are working with

The maintainer is an experienced developer (10+ years C++, 10 years Python, some Ruby/Rails) who is **learning Rust through this project**. This affects how you write code:

- **Prefer simple, idiomatic Rust over clever Rust.** Use owned types (`String`, `Vec<T>`, `PathBuf`) in structs and public APIs. Avoid explicit lifetime annotations unless they're clearly necessary, and avoid complex generic bounds and trait-object gymnastics where a concrete type works.
- **Add short `// Rust note:` comments** the first time a non-obvious Rust concept appears in a file (for example `?` propagation, `impl Trait`, `Arc<Mutex<_>>`, pattern matching with bindings, iterator adaptors that move vs borrow). One or two lines, relating it to C++ or Python where helpful. Don't repeat the same note throughout a file.
- **When you finish a task**, briefly list any Rust concepts that appeared for the first time, so the maintainer can read up on them.
- Do not hide complexity behind macros you wrote yourself. Derive macros from common crates (`serde`, `thiserror`) are fine.

## Repository layout

```
crates/scrald-core/   # pure Rust library, no Tauri or GUI dependencies
crates/scrald-app/    # Tauri application (tauri.conf.json, capabilities/, icons/ live here)
ui/                   # Svelte + TypeScript frontend (npm workspace)
assets/               # source artwork, e.g. icon-source.png (regenerate icons with
                      #   `npm run tauri -- icon assets/icon-source.png -o crates/scrald-app/icons`,
                      #   then delete the android/, ios/, and Square*/StoreLogo outputs)
.github/workflows/    # CI
package.json          # root npm workspace + Tauri CLI
justfile              # task shortcuts (`just` lists them)
DESIGN.md  TODO.md  AGENTS.md  CLAUDE.md
```

Logic belongs in `scrald-core` whenever it can be tested without a window. `scrald-app` should mostly translate between Tauri and core.

## Environment

- Development happens on **native Windows 11** in a normal NTFS directory, not WSL. The shell is PowerShell or Git Bash. Write commands that work in both where practical, and never assume Linux-only tools.
- Toolchain: Rust stable via rustup (MSVC target), Node.js LTS, Visual Studio Build Tools (C++ workload). WebView2 is part of Windows 11.
- The maintainer's editor is Zed. Don't add VS Code-specific config files.
- Keep code free of Windows-only APIs unless isolated behind a clear module boundary, so macOS and Linux builds remain possible.
- `scrald-core` must pass its tests on Windows, macOS, and Linux at all times (CI checks this). Use `std::path` APIs, never string-concatenated `\` separators, and don't assume a case-insensitive file system.

## Commands

Run these from the repository root. The `justfile` wraps the common ones (run `just` to list recipes; needs [just](https://github.com/casey/just), e.g. `winget install Casey.Just`). The underlying commands work in both PowerShell and Git Bash. The root `package.json` is an npm workspace containing `ui/`, so one `npm install` at the root installs everything.

| Task | `just` recipe | Underlying command |
|---|---|---|
| Install frontend deps | `just install` | `npm install` |
| Run app in dev mode | `just dev` | `npm run tauri dev` |
| Run app with a file | `just open path/to/file.md` | `npm run tauri -- dev -- -- /absolute/path/to/file.md` (the app's working directory is `crates/scrald-app`, so pass an absolute path) |
| Open a large fixture | `just large 250k` | generates fixtures if needed, then opens `target/fixtures/large-250k.md` |
| Everything CI checks | `just check` | fmt check, clippy, Rust tests, typecheck, frontend tests |
| All tests | `just test` | `cargo test --workspace` and `npm test` |
| Rust format | `just fmt` | `cargo fmt --all` |
| Rust lint | `just clippy` | `cargo clippy --workspace --all-targets -- -D warnings` |
| Rust tests | `just test-rust` | `cargo test --workspace` |
| Update snapshots | `just snapshots` | `INSTA_UPDATE=always cargo test -p scrald-core --test snapshots`, then review the diff in git (or use `cargo insta review` with `cargo install cargo-insta`) |
| Benchmarks | `just bench` | `cargo bench -p scrald-core` |
| Write large fixtures | `just fixtures` | `cargo run -p scrald-core --example generate_fixtures` (writes `target/fixtures/large-{100k,250k,500k}.md`) |
| Frontend typecheck | `just typecheck` | `npm run check` |
| Frontend tests | `just test-ui` | `npm test` |
| Release build (no installer) | `just build` | `npm run tauri -- build --no-bundle` |
| Release build + log to file | `just run-release file.md` | builds, then runs the release exe with `SCRALD_LOG_FILE=target/scrald.log` |
| Release build + installers | `just bundle` | `npm run tauri build` (verified in M7) |

When adding a common task, add a recipe to the `justfile` and a row here. Keep recipes to plain `cargo`/`npm` commands (one per line, no `&&`), so they run under both PowerShell (`windows-shell`) and sh.

Before `cargo clippy` or `cargo build` on a fresh checkout, build the frontend once (`npm run build --workspace ui`, or any `tauri dev`/`build` run): Tauri's `generate_context!` needs `ui/dist` to exist.

Logging: set `RUST_LOG` to override the default filter (`info,scrald=debug,scrald_core=debug`), e.g. `RUST_LOG=trace`. Log output appears in the terminal for debug builds. Release builds have no console on Windows; set `SCRALD_LOG_FILE=path/to/log.txt` to write the log to a file instead.

Performance checks (DESIGN.md §4): the frontend reports `ipc_open_document`, `first_screen`, and `full_mount` timings into the backend log via the `report_timing` command. Measure with a release build (`npm run tauri -- build --no-bundle`), `SCRALD_LOG_FILE` set, and the generated fixtures in `target/fixtures/`.

## Work tracking

Work is tracked in GitHub issues on `torstees/scrald`, not in `TODO.md`:

- Each milestone (M0–M10, plus "Later / ideas") is a **parent issue** with a matching GitHub **Milestone**. Individual work items are **sub-issues** of that parent, assigned to the same Milestone.
- `TODO.md` is only an index of the milestone issues plus the **decisions log**. Record notable implementation decisions there, and update `DESIGN.md` to match.
- To add work: `gh issue create -R torstees/scrald --milestone "<milestone>" ...`, then attach it to the parent with `gh api --method POST repos/torstees/scrald/issues/<parent>/sub_issues -F sub_issue_id=<issue id>` (the numeric `id`, not the issue number).
- Only ever run `gh` against `torstees/scrald`. The maintainer's account is linked to work organizations that must never be touched, so never run account-wide or unscoped queries (`gh repo list`, `gh search` without `repo:`, org APIs).

## Pull requests

Each completed chunk of work (usually a few related sub-issues) goes up as a pull request on `torstees/scrald`, not as a direct push to `main`:

1. Work on a branch named after the chunk, e.g. `m3/state-store` or `chore/justfile`.
2. When `just check` passes, push the branch and open a PR with `gh pr create -R torstees/scrald`. The body follows `.github/pull_request_template.md`.
3. The PR body has checkbox lists for **what's added** and for **manual tests** the maintainer can run (with `just` recipes and fixtures). Tick the manual tests you already verified yourself, and say how (screenshot, UI Automation, release build).
4. Close issues from the PR body (`Closes #12`), not only from commits.
5. Don't merge; the maintainer reviews and merges. Start the next chunk from up-to-date `main`, or stack on the open branch if it depends on it and say so in the PR.

## Definition of done for any change

1. `cargo fmt`, `cargo clippy` (no warnings), and `cargo test` all pass.
2. Frontend typecheck and tests pass if `ui/` changed.
3. New core behavior has tests. Rendering changes have snapshot tests with reviewed snapshots.
4. The work is in a pull request (see above) that closes its sub-issues (`Closes #12`), and new follow-up work is filed as sub-issues of the right milestone issue.
5. If a change contradicts or extends `DESIGN.md`, update `DESIGN.md` in the same change and call it out in your summary.

## Coding conventions

### Rust

- Error handling: `thiserror` for error types in `scrald-core`, `anyhow` in `scrald-app`. No `unwrap()` or `expect()` outside tests, except where an invariant truly cannot fail; then use `expect("reason")` with a reason.
- Keep functions small and pure in core where possible: input in, output out, no global state.
- Byte offsets into the original file are `usize` wrapped in a `SourceRange { start, end }` type. Never mix up line/column positions and byte offsets; convert in exactly one place.
- Logging: `tracing`, not `println!`.
- Do not add a new crate dependency without stating why in your summary. Dependencies already chosen in `DESIGN.md` are pre-approved.

### Frontend

- TypeScript strict mode. No `any` without a comment explaining why.
- Theme values reach the page only through CSS custom properties (`--sk-*`). Components must not hard-code colors or fonts.
- Heavy libraries (KaTeX, Mermaid, CodeMirror) are loaded with dynamic `import()` only when needed.
- Never insert HTML from anywhere except the sanitized `Block.html` produced by core.

### Tauri boundary

- Commands are defined in `crates/scrald-app/src/commands.rs` and return serializable structs from core. Keep command signatures simple and documented.
- Mirror command payload types in TypeScript in `ui/src/lib/types.ts`, and keep them in sync when the Rust types change.

## Things to be careful about

- **Line endings and encoding.** Files may be CRLF or LF, with or without a UTF-8 BOM. Preserve both on save. Source-range code must be tested with CRLF and multi-byte characters.
- **Never corrupt user documents.** Saves are atomic (temp file + rename). Edits only touch the edited byte range. A no-op edit must round-trip byte-for-byte. When unsure, don't write.
- **YAML front matter** is edited with minimal text edits, never by parsing and re-serializing the whole block.
- **Security.** All rendered HTML is sanitized. The asset protocol serves only resolved document assets and theme assets. No remote scripts, ever.
- **Performance.** Before and after changes to parsing, rendering, or the reader's DOM structure, check the large fixtures. Don't introduce whole-document work on the frontend's main thread.
- **Don't modify files outside the repository** and never run the app against the maintainer's real documents in automated steps. Use fixtures.

## When in doubt

Ask before making architectural changes, adding significant dependencies, or deviating from `DESIGN.md`. For small judgment calls, make a reasonable choice and mention it in your summary.
