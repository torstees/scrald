//! Live reload: watches each window's document and tells that window when
//! the file changes on disk (DESIGN.md §9.3).

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::Context;
use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};
use tauri::{AppHandle, Emitter, EventTarget};

/// Event sent to a window when its document changed on disk. Payload: the path.
pub const DOCUMENT_CHANGED: &str = "document-changed";

/// Editors often save in several steps (write temp file, rename, touch);
/// wait this long after the last event before reloading.
const DEBOUNCE: Duration = Duration::from_millis(300);

/// One watcher per window, for the document that window shows.
#[derive(Default)]
pub struct DocumentWatchers {
    inner: Mutex<HashMap<String, Debouncer<RecommendedWatcher>>>,
}

impl DocumentWatchers {
    /// Starts watching `path` for `window`, replacing that window's previous
    /// watcher (dropping a debouncer stops it).
    pub fn watch(&self, app: &AppHandle, window: &str, path: &Path) -> anyhow::Result<()> {
        let dir = path
            .parent()
            .context("document has no parent directory")?
            .to_path_buf();
        let file_name = path
            .file_name()
            .context("document path has no file name")?
            .to_os_string();

        let app = app.clone();
        let label = window.to_string();
        let changed_path = path.to_path_buf();
        // Rust note: `move` closures take ownership of what they use, so the
        // watcher thread can call this long after `watch` has returned.
        let mut debouncer =
            new_debouncer(DEBOUNCE, move |result: DebounceEventResult| match result {
                Ok(events) => {
                    if events.iter().any(|e| same_file_name(&e.path, &file_name)) {
                        notify_window(&app, &label, &changed_path);
                    }
                }
                Err(error) => tracing::warn!(%error, "file watcher error"),
            })
            .context("could not create file watcher")?;

        // Watch the directory, not the file: editors that save by writing a
        // new file and renaming it over the old one would otherwise end the
        // watch after the first save.
        debouncer
            .watcher()
            .watch(&dir, RecursiveMode::NonRecursive)
            .with_context(|| format!("could not watch {}", dir.display()))?;

        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.insert(window.to_string(), debouncer);
        Ok(())
    }

    /// Stops watching for a window (when it closes).
    pub fn unwatch(&self, window: &str) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.remove(window);
    }
}

fn notify_window(app: &AppHandle, label: &str, path: &Path) {
    tracing::debug!(path = %path.display(), window = label, "document changed on disk");
    let target = EventTarget::webview_window(label);
    if let Err(error) = app.emit_to(target, DOCUMENT_CHANGED, path.display().to_string()) {
        tracing::warn!(%error, "could not notify window of document change");
    }
}

/// Whether `path` names the watched file. Compared case-insensitively, since
/// Windows and macOS file systems usually are; on a case-sensitive system a
/// sibling differing only in case just causes a harmless extra reload.
fn same_file_name(path: &Path, file_name: &OsString) -> bool {
    path.file_name().is_some_and(|n| {
        n.to_string_lossy()
            .eq_ignore_ascii_case(&file_name.to_string_lossy())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn matches_file_names_case_insensitively() {
        let name = OsString::from("Notes.md");
        assert!(same_file_name(
            &PathBuf::from("dir").join("notes.MD"),
            &name
        ));
        assert!(!same_file_name(
            &PathBuf::from("dir").join("other.md"),
            &name
        ));
        assert!(!same_file_name(Path::new(""), &name));
    }
}
