# AGENTS.md

Guidance for coding agents (Claude Code and others) working on Scrald. Read `DESIGN.md` for architecture and decisions, and `TODO.md` for the current build order.

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
crates/scrald-app/    # Tauri application
ui/                   # Svelte + TypeScript frontend
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

Run these from the repository root unless noted. Adjust this section as scripts are added.

| Task | Command |
|---|---|
| Run app in dev mode | `npm run tauri dev` (from `ui/` or root, per final setup) |
| Rust format | `cargo fmt --all` |
| Rust lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Rust tests | `cargo test --workspace` |
| Review snapshots | `cargo insta review` |
| Benchmarks | `cargo bench -p scrald-core` |
| Frontend typecheck | `npm run check` (in `ui/`) |
| Frontend tests | `npm run test` (in `ui/`) |
| Release build | `npm run tauri build` |

## Definition of done for any change

1. `cargo fmt`, `cargo clippy` (no warnings), and `cargo test` all pass.
2. Frontend typecheck and tests pass if `ui/` changed.
3. New core behavior has tests. Rendering changes have snapshot tests with reviewed snapshots.
4. Relevant `TODO.md` items are checked off, and new follow-up work is added there.
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
