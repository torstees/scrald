//! Tauri commands exposed to the frontend.
//!
//! Payload types here are mirrored in TypeScript in `ui/src/lib/types.ts`;
//! keep the two in sync.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::Context;
use scrald_core::DocumentModel;
use serde::Serialize;

/// An error returned to the frontend, where it arrives as a rejected promise
/// with this message.
// Rust note: Tauri needs command errors to be `Serialize`. `anyhow::Error`
// isn't, so this wrapper carries the message text across the boundary.
#[derive(Debug, Serialize)]
pub struct CommandError(String);

// Rust note: implementing `From<E>` lets `?` convert errors automatically:
// any error type that `anyhow::Error` accepts becomes a `CommandError`.
impl<E: Into<anyhow::Error>> From<E> for CommandError {
    fn from(error: E) -> Self {
        let error: anyhow::Error = error.into();
        // `{:#}` includes the chain of `.context()` messages.
        CommandError(format!("{error:#}"))
    }
}

/// Reads and parses a Markdown file, and sets the window title from it.
///
/// Parsing runs on a background thread so the window stays responsive.
// Rust note: an `async fn` command runs on Tauri's async runtime instead of
// the main thread. `spawn_blocking` then moves the CPU-bound parse onto a
// thread pool, and `.await` waits for it without blocking anything.
#[tauri::command]
pub async fn open_document(
    window: tauri::WebviewWindow,
    path: PathBuf,
) -> Result<DocumentModel, CommandError> {
    let started = Instant::now();
    let load_path = path.clone();
    // Rust note: `move` makes the closure take ownership of `load_path`, so it
    // can run on another thread after this function's locals are gone. The
    // double `??` unwraps two layers: the thread's result, then the parse's.
    let doc = tauri::async_runtime::spawn_blocking(move || scrald_core::load_document(&load_path))
        .await
        .context("document loading task failed")??;
    tracing::info!(
        path = %path.display(),
        blocks = doc.blocks.len(),
        words = doc.word_count,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "opened document"
    );

    let title = doc
        .front_matter
        .as_ref()
        .and_then(|fm| fm.title.clone())
        .unwrap_or_else(|| scrald_core::title_from_path(&path));
    window.set_title(&format!("{title} \u{2014} Scrald"))?;
    Ok(doc)
}

/// Logs a timing measured in the frontend, so performance numbers end up in
/// the same log as the backend's (DESIGN.md §4 budget checks).
#[tauri::command]
pub fn report_timing(name: String, ms: f64) {
    tracing::info!(name, ms = (ms * 10.0).round() / 10.0, "frontend timing");
}

/// What the app was launched with. Built once at startup and stored as Tauri
/// managed state.
// Rust note: `#[derive(...)]` asks the compiler to generate trait impls:
// `Clone` gives `.clone()` (an explicit deep copy, like a C++ copy ctor), and
// serde's `Serialize` generates the JSON conversion Tauri needs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchInfo {
    /// Absolute path from the command line, if one was given.
    pub path: Option<PathBuf>,
    /// Display title for the window and page.
    pub title: String,
    /// Whether `path` existed when the app started.
    pub exists: bool,
}

impl LaunchInfo {
    /// Builds launch info from the first command-line argument, if any.
    pub fn from_arg(arg: Option<PathBuf>) -> Self {
        // Rust note: `map` transforms the value inside an Option only if it's
        // present, much like Python's `f(x) if x is not None else None`.
        let path = arg.map(|p| std::path::absolute(&p).unwrap_or(p));
        let title = match &path {
            Some(p) => scrald_core::title_from_path(p),
            None => "Scrald".to_string(),
        };
        let exists = path.as_ref().is_some_and(|p| p.is_file());
        LaunchInfo {
            path,
            title,
            exists,
        }
    }
}

/// Returns the path and title the app was launched with.
// Rust note: `State<'_, T>` borrows Tauri's managed state for the duration of
// this call; `'_` tells the compiler "infer the lifetime", so you rarely need
// to name it. We return a clone because the frontend needs an owned value.
#[tauri::command]
pub fn launch_info(state: tauri::State<'_, LaunchInfo>) -> LaunchInfo {
    state.inner().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_argument_means_no_document() {
        let info = LaunchInfo::from_arg(None);
        assert_eq!(info.path, None);
        assert_eq!(info.title, "Scrald");
        assert!(!info.exists);
    }

    #[test]
    fn relative_argument_becomes_absolute() {
        let info = LaunchInfo::from_arg(Some(PathBuf::from("does-not-exist.md")));
        let path = info.path.expect("path should be set");
        assert!(path.is_absolute());
        assert_eq!(info.title, "does-not-exist");
        assert!(!info.exists);
    }

    #[test]
    fn existing_file_is_detected() {
        // This crate's own manifest is a file that always exists.
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let info = LaunchInfo::from_arg(Some(manifest));
        assert!(info.exists);
        assert_eq!(info.title, "Cargo");
    }
}
