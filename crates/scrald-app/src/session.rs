//! The text of each window's document, including unsaved edits (DESIGN.md
//! §9.3). Edits change this text, never the file; `save` writes it back
//! atomically with the BOM it was read with. The text keeps the file's line
//! endings, because edits only splice into it.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::Context;
use serde::Serialize;

/// One window's document.
#[derive(Debug, Clone)]
struct Session {
    path: PathBuf,
    /// The document's text without its BOM, with any unsaved edits.
    text: String,
    has_bom: bool,
    /// Whether `text` differs from what was last read or saved.
    dirty: bool,
    /// Hash of the file's bytes as last read or written, to tell our own
    /// saves (and touches that change nothing) from real external edits.
    disk_hash: u64,
}

/// How the file on disk compares with what Scrald last read or saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DiskState {
    /// Same bytes: Scrald's own save, or a touch that changed nothing.
    Unchanged,
    /// Different bytes: someone else edited the file.
    Changed,
    /// The file can't be read any more (deleted, renamed, locked).
    Missing,
}

#[derive(Default)]
pub struct Sessions {
    inner: Mutex<HashMap<String, Session>>,
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

impl Sessions {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Session>> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Reads `path` from disk as `window`'s document, replacing any earlier
    /// one (the frontend asks about unsaved changes first). Returns the bytes.
    pub fn open(&self, window: &str, path: &Path) -> anyhow::Result<Vec<u8>> {
        let bytes =
            std::fs::read(path).with_context(|| format!("could not read {}", path.display()))?;
        let decoded = scrald_core::source::decode(&bytes)?;
        self.lock().insert(
            window.to_string(),
            Session {
                path: path.to_path_buf(),
                text: decoded.text,
                has_bom: decoded.has_bom,
                dirty: false,
                disk_hash: hash_bytes(&bytes),
            },
        );
        Ok(bytes)
    }

    /// The document's path and current bytes (with unsaved edits), to parse.
    pub fn current(&self, window: &str) -> Option<(PathBuf, Vec<u8>)> {
        self.lock().get(window).map(|s| {
            (
                s.path.clone(),
                scrald_core::save::encode(&s.text, s.has_bom),
            )
        })
    }

    /// Replaces the text with `edit(text)`. Marks the document unsaved if
    /// the text changed. Returns the path and new bytes, to parse.
    // Rust note: `impl FnOnce(&str) -> Result<String, E>` accepts any closure
    // that can be called once with the text, like a callable parameter in
    // Python; `E` is whatever error type that closure returns.
    pub fn edit<E>(
        &self,
        window: &str,
        edit: impl FnOnce(&str) -> Result<String, E>,
    ) -> anyhow::Result<(PathBuf, Vec<u8>)>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let mut sessions = self.lock();
        let session = sessions
            .get_mut(window)
            .context("no document is open in this window")?;
        let edited = edit(&session.text)?;
        if edited != session.text {
            session.text = edited;
            session.dirty = true;
        }
        Ok((
            session.path.clone(),
            scrald_core::save::encode(&session.text, session.has_bom),
        ))
    }

    /// Writes the document to disk atomically and marks it saved.
    pub fn save(&self, window: &str) -> anyhow::Result<PathBuf> {
        let mut sessions = self.lock();
        let session = sessions
            .get_mut(window)
            .context("no document is open in this window")?;
        let bytes = scrald_core::save::encode(&session.text, session.has_bom);
        scrald_core::save::write_atomic(&session.path, &bytes)
            .with_context(|| format!("could not save {}", session.path.display()))?;
        session.disk_hash = hash_bytes(&bytes);
        session.dirty = false;
        tracing::info!(path = %session.path.display(), bytes = bytes.len(), "saved document");
        Ok(session.path.clone())
    }

    pub fn is_dirty(&self, window: &str) -> bool {
        self.lock().get(window).is_some_and(|s| s.dirty)
    }

    /// Compares the file on disk with what was last read or saved.
    pub fn disk_state(&self, window: &str) -> DiskState {
        let Some((path, expected)) = self
            .lock()
            .get(window)
            .map(|s| (s.path.clone(), s.disk_hash))
        else {
            return DiskState::Missing;
        };
        match std::fs::read(&path) {
            Ok(bytes) if hash_bytes(&bytes) == expected => DiskState::Unchanged,
            Ok(_) => DiskState::Changed,
            Err(_) => DiskState::Missing,
        }
    }

    pub fn forget(&self, window: &str) {
        self.lock().remove(window);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str, contents: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scrald-session-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }

    fn upper(text: &str) -> Result<String, std::io::Error> {
        Ok(text.to_uppercase())
    }

    #[test]
    fn edits_stay_in_memory_until_saved() {
        let path = temp_file("edit.md", b"\xEF\xBB\xBFhello\r\n");
        let sessions = Sessions::default();
        sessions.open("main", &path).unwrap();
        assert!(!sessions.is_dirty("main"));

        let (_, bytes) = sessions.edit("main", upper).unwrap();
        assert_eq!(bytes, b"\xEF\xBB\xBFHELLO\r\n");
        assert!(sessions.is_dirty("main"));
        // The file hasn't changed yet.
        assert_eq!(std::fs::read(&path).unwrap(), b"\xEF\xBB\xBFhello\r\n");
        assert_eq!(sessions.disk_state("main"), DiskState::Unchanged);

        sessions.save("main").unwrap();
        assert!(!sessions.is_dirty("main"));
        // BOM and CRLF kept.
        assert_eq!(std::fs::read(&path).unwrap(), b"\xEF\xBB\xBFHELLO\r\n");
        // Our own save doesn't count as an external change.
        assert_eq!(sessions.disk_state("main"), DiskState::Unchanged);
    }

    #[test]
    fn external_changes_are_detected() {
        let path = temp_file("external.md", b"one");
        let sessions = Sessions::default();
        sessions.open("w", &path).unwrap();
        std::fs::write(&path, b"two").unwrap();
        assert_eq!(sessions.disk_state("w"), DiskState::Changed);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(sessions.disk_state("w"), DiskState::Missing);
    }

    #[test]
    fn an_edit_that_changes_nothing_is_not_unsaved() {
        let path = temp_file("noop.md", b"SAME");
        let sessions = Sessions::default();
        sessions.open("w", &path).unwrap();
        sessions.edit("w", upper).unwrap();
        assert!(!sessions.is_dirty("w"));
    }
}
