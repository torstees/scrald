//! The text of each window's document, including unsaved edits (DESIGN.md
//! §9.3). Edits change this text, never the file; `save` writes it back
//! atomically with the BOM it was read with. The text keeps the file's line
//! endings, because edits only splice into it.
//!
//! While a document has unsaved changes, `write_recovery` keeps a copy of its
//! text in a recovery file (`scrald_core::recovery`), deleted again once the
//! changes are saved or discarded, so a crash loses at most a few seconds.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Duration;

use anyhow::Context;
use scrald_core::delta::{self, ShownBlocks};
use scrald_core::edit::TextDiff;
use scrald_core::recovery::{self, RecoveryEntry, RecoveryHeader};
use scrald_core::search::SearchIndex;
use scrald_core::{Block, RenderCache};
use serde::Serialize;

/// One window's document.
#[derive(Debug)]
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
    /// Changes that can be undone, most recent last, and changes undone
    /// that can be redone. Each is a small diff, not a copy of the text.
    undo: Vec<TextDiff>,
    redo: Vec<TextDiff>,
    /// Goes up with every change to `text`, so raw mode can tell whether the
    /// text it holds is still the current one.
    version: u64,
    /// The changed file as last found on disk while there were unsaved edits:
    /// what the conflict banner offers to load or compare (DESIGN.md §9.3).
    disk_snapshot: Option<DiskSnapshot>,
    /// Hash of a disk version the user chose to overwrite ("Keep mine"), so
    /// saving over it is allowed. Any other external change blocks the save.
    overwrite_hash: Option<u64>,
    /// The name of this session's recovery file, and the `version` of the
    /// text it holds; `None` while there's no file.
    recovery_id: String,
    recovery_written: Option<u64>,
    /// Sanitized HTML and highlighted code from the last parse, so the next
    /// one only redoes changed blocks (`scrald_core::RenderCache`).
    render_cache: RenderCache,
    /// The blocks last sent to the window, and that model's number, so the
    /// next model can leave out the HTML the window already has.
    shown_blocks: ShownBlocks,
    shown_model: u64,
    /// The text of the model last sent, for searching it (DESIGN.md §10).
    /// `None` while a search has it.
    search: Option<SearchIndex>,
}

/// A model about to be sent to a window (see `Sessions::send_blocks`).
#[derive(Debug)]
pub struct BlocksSent {
    /// This model's number.
    pub model: u64,
    /// The model the window must be showing to fill in the left-out HTML,
    /// or `None` if every block has its HTML.
    pub base: Option<u64>,
    /// For each block with its HTML left out, the id of the block in the
    /// `base` model to copy it from.
    pub html_from: Vec<Option<u32>>,
}

/// A version of the file found on disk.
#[derive(Debug, Clone)]
struct DiskSnapshot {
    hash: u64,
    bytes: Vec<u8>,
}

/// How many changes undo remembers.
const UNDO_LIMIT: usize = 500;

/// How often unsaved text is copied to its recovery file (if it changed).
pub const RECOVERY_INTERVAL: Duration = Duration::from_secs(3);

impl Session {
    fn bytes(&self) -> Vec<u8> {
        scrald_core::save::encode(&self.text, self.has_bom)
    }

    /// Unsaved means different from the file as last read or saved, so
    /// undoing back to the saved text clears it.
    fn update_dirty(&mut self) {
        self.dirty = hash_bytes(&self.bytes()) != self.disk_hash;
    }
}

/// What happened to a save.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SaveOutcome {
    Saved,
    /// The file changed on disk and the user hasn't chosen what to keep;
    /// nothing was written.
    Conflict,
}

/// Which way to step through the edit history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Undo,
    Redo,
}

/// How the file on disk compares with what Scrald last read or saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DiskState {
    /// Same bytes: Scrald's own save, a touch that changed nothing, or the
    /// version the user already chose to overwrite.
    Unchanged,
    /// Different bytes: someone else edited the file.
    Changed,
    /// The file can't be read any more (deleted, renamed, locked).
    Missing,
}

#[derive(Default)]
pub struct Sessions {
    inner: Mutex<HashMap<String, Session>>,
    /// The folder for recovery files; unset (no recovery) if the app has no
    /// data folder.
    // Rust note: a `OnceLock` can be set once, from any thread, and read
    // freely afterwards: a thread-safe "initialized later" value.
    recovery_dir: OnceLock<PathBuf>,
    /// Held while recovery files are written or deleted, so a write that
    /// started before a save or discard can't bring a file back after it.
    /// Always taken before `inner`, never while holding it.
    recovery_files: Mutex<()>,
}

/// A stable hash, since recovery files store it (DESIGN.md §12).
fn hash_bytes(bytes: &[u8]) -> u64 {
    scrald_core::hash::fnv1a(bytes)
}

/// A recovery file name no other session, in this run or another, uses.
fn new_recovery_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:x}-{:x}-{n:x}", now_millis(), std::process::id())
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// Whether two paths name the same document (as the state database sees it).
fn same_document(a: &Path, b: &Path) -> bool {
    a == b || crate::state::document_key(a) == crate::state::document_key(b)
}

/// One recovery file to bring up to date, collected under the lock.
enum RecoveryJob {
    Write {
        window: String,
        id: String,
        version: u64,
        header: RecoveryHeader,
        text: String,
    },
    Remove {
        window: String,
        id: String,
    },
}

impl Sessions {
    fn lock(&self) -> MutexGuard<'_, HashMap<String, Session>> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    // Rust note: `Mutex<()>` guards no data, only a stretch of code; the
    // returned guard keeps it locked until it's dropped at the end of the
    // caller's scope, like `std::lock_guard`.
    fn lock_recovery_files(&self) -> MutexGuard<'_, ()> {
        self.recovery_files
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Where recovery files go. Set once at startup; without it, nothing is
    /// written.
    pub fn set_recovery_dir(&self, dir: PathBuf) {
        let _ = self.recovery_dir.set(dir);
    }

    /// Deletes a session's recovery file, if it has one. Logged, not
    /// returned: a leftover file is offered again later, never lost.
    fn remove_recovery_file(&self, session: &Session) {
        if session.recovery_written.is_none() {
            return;
        }
        if let Some(dir) = self.recovery_dir.get()
            && let Err(error) = recovery::remove(dir, &session.recovery_id)
        {
            tracing::warn!(error = %error, "could not delete a recovery file");
        }
    }

    /// Reads `path` from disk as `window`'s document, replacing any earlier
    /// one (the frontend asks about unsaved changes first). Returns the bytes.
    pub fn open(&self, window: &str, path: &Path) -> anyhow::Result<Vec<u8>> {
        let bytes =
            std::fs::read(path).with_context(|| format!("could not read {}", path.display()))?;
        let decoded = scrald_core::source::decode(&bytes)?;
        let _files = self.lock_recovery_files();
        // Replacing the window's document discards its unsaved changes
        // (the user was asked), so their recovery file goes too.
        let previous = self.lock().insert(
            window.to_string(),
            Session {
                path: path.to_path_buf(),
                text: decoded.text,
                has_bom: decoded.has_bom,
                dirty: false,
                disk_hash: hash_bytes(&bytes),
                undo: Vec::new(),
                redo: Vec::new(),
                version: self.next_version(),
                disk_snapshot: None,
                overwrite_hash: None,
                recovery_id: new_recovery_id(),
                recovery_written: None,
                render_cache: RenderCache::default(),
                shown_blocks: ShownBlocks::new(),
                shown_model: 0,
                search: None,
            },
        );
        if let Some(previous) = previous {
            self.remove_recovery_file(&previous);
        }
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
            session.undo.push(TextDiff::between(&session.text, &edited));
            if session.undo.len() > UNDO_LIMIT {
                session.undo.remove(0);
            }
            session.redo.clear();
            session.text = edited;
            session.version += 1;
            session.update_dirty();
        }
        Ok((session.path.clone(), session.bytes()))
    }

    /// Undoes or redoes one change. `None` if there's nothing to step to.
    pub fn step(&self, window: &str, step: Step) -> anyhow::Result<Option<(PathBuf, Vec<u8>)>> {
        let mut sessions = self.lock();
        let session = sessions
            .get_mut(window)
            .context("no document is open in this window")?;
        let popped = match step {
            Step::Undo => session.undo.pop(),
            Step::Redo => session.redo.pop(),
        };
        let Some(diff) = popped else {
            return Ok(None);
        };
        let stepped = match step {
            Step::Undo => diff.revert(&session.text),
            Step::Redo => diff.apply(&session.text),
        };
        // Diffs always fit: every change to the text goes through `edit`.
        let Some(text) = stepped else {
            session.undo.clear();
            session.redo.clear();
            // Rust note: `bail!` returns early with an error, like `raise`.
            anyhow::bail!("the edit history no longer matches the document");
        };
        session.text = text;
        session.version += 1;
        match step {
            Step::Undo => session.redo.push(diff),
            Step::Redo => session.undo.push(diff),
        }
        session.update_dirty();
        Ok(Some((session.path.clone(), session.bytes())))
    }

    /// The text in a block's range (original-file offsets, BOM counted).
    pub fn source(&self, window: &str, start: usize, end: usize) -> Option<String> {
        let sessions = self.lock();
        let session = sessions.get(window)?;
        let bom = usize::from(session.has_bom) * 3;
        session
            .text
            .get(start.checked_sub(bom)?..end.checked_sub(bom)?)
            .map(str::to_string)
    }

    /// The whole text (without BOM) and its version, for raw mode.
    pub fn text(&self, window: &str) -> Option<(String, u64)> {
        self.lock().get(window).map(|s| (s.text.clone(), s.version))
    }

    /// Replaces the whole text (raw mode), if `version` is still current:
    /// otherwise the text changed meanwhile (a reload, an undo) and nothing is
    /// replaced. Recorded for undo like any edit. Returns the new version and
    /// whether the document now has unsaved changes.
    pub fn replace_text(
        &self,
        window: &str,
        version: u64,
        text: String,
    ) -> anyhow::Result<(u64, bool)> {
        let current = self
            .lock()
            .get(window)
            .map(|s| s.version)
            .context("no document is open in this window")?;
        if current != version {
            anyhow::bail!("the document changed while it was being edited in raw mode");
        }
        self.edit(window, |_| Ok::<String, std::io::Error>(text))?;
        let sessions = self.lock();
        let session = sessions
            .get(window)
            .context("no document is open in this window")?;
        Ok((session.version, session.dirty))
    }

    /// Versions keep rising across documents too, so text from a document
    /// that was replaced can never match the new one's version.
    fn next_version(&self) -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        NEXT.fetch_add(1_000_000, Ordering::Relaxed)
    }

    /// Takes the window's render cache for a parse; `put_render_cache` gives
    /// it back. Empty if there's no document (or another parse has it).
    pub fn take_render_cache(&self, window: &str) -> RenderCache {
        self.lock()
            .get_mut(window)
            .map(|s| std::mem::take(&mut s.render_cache))
            .unwrap_or_default()
    }

    pub fn put_render_cache(&self, window: &str, cache: RenderCache) {
        if let Some(session) = self.lock().get_mut(window) {
            session.render_cache = cache;
        }
    }

    /// Records `blocks` as sent to the window, leaving out the HTML of the
    /// blocks it already shows (all of it is kept with `full`, or when the
    /// window hasn't been sent a model for this document yet).
    pub fn send_blocks(&self, window: &str, blocks: &mut [Block], full: bool) -> BlocksSent {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_MODEL: AtomicU64 = AtomicU64::new(1);
        let mut sessions = self.lock();
        let Some(session) = sessions.get_mut(window) else {
            return BlocksSent {
                model: 0,
                base: None,
                html_from: Vec::new(),
            };
        };
        let model = NEXT_MODEL.fetch_add(1, Ordering::Relaxed);
        let base = (!full && session.shown_model != 0).then_some(session.shown_model);
        let html_from = if base.is_some() {
            let (from, shown) = delta::reuse_html(blocks, &session.shown_blocks);
            session.shown_blocks = shown;
            from
        } else {
            session.shown_blocks = delta::shown_blocks(blocks);
            Vec::new()
        };
        session.shown_model = model;
        BlocksSent {
            model,
            base,
            html_from,
        }
    }

    /// Keeps `index` for searching the model about to be sent (call before
    /// `send_blocks`, which leaves HTML out of the blocks).
    pub fn set_search(&self, window: &str, index: SearchIndex) {
        if let Some(session) = self.lock().get_mut(window) {
            session.search = Some(index);
        }
    }

    /// Takes the search index of the model the window shows, with that
    /// model's number, so a search can run without holding the lock;
    /// `put_search` gives it back. `None` while another search has it.
    pub fn take_search(&self, window: &str) -> Option<(u64, SearchIndex)> {
        let mut sessions = self.lock();
        let session = sessions.get_mut(window)?;
        let index = session.search.take()?;
        Some((session.shown_model, index))
    }

    /// Gives back an index from `take_search`, unless a newer model has
    /// been sent meanwhile (it has its own index).
    pub fn put_search(&self, window: &str, model: u64, index: SearchIndex) {
        if let Some(session) = self.lock().get_mut(window)
            && session.shown_model == model
            && session.search.is_none()
        {
            session.search = Some(index);
        }
    }

    /// 3 if the document has a BOM, else 0: block ranges count it.
    pub fn bom_len(&self, window: &str) -> usize {
        self.lock()
            .get(window)
            .map_or(0, |s| usize::from(s.has_bom) * 3)
    }

    /// Writes the document to disk atomically and marks it saved, unless
    /// the file changed on disk in a way the user hasn't seen: then nothing
    /// is written and the change is kept for the conflict banner. (A missing
    /// file is written again.)
    pub fn save(&self, window: &str) -> anyhow::Result<SaveOutcome> {
        let _files = self.lock_recovery_files();
        let mut sessions = self.lock();
        let session = sessions
            .get_mut(window)
            .context("no document is open in this window")?;
        if let Ok(on_disk) = std::fs::read(&session.path) {
            let hash = hash_bytes(&on_disk);
            if hash != session.disk_hash && Some(hash) != session.overwrite_hash {
                tracing::info!(path = %session.path.display(), "save refused: the file changed on disk");
                session.disk_snapshot = Some(DiskSnapshot {
                    hash,
                    bytes: on_disk,
                });
                return Ok(SaveOutcome::Conflict);
            }
        }
        let bytes = session.bytes();
        scrald_core::save::write_atomic(&session.path, &bytes)
            .with_context(|| format!("could not save {}", session.path.display()))?;
        session.disk_hash = hash_bytes(&bytes);
        session.dirty = false;
        session.disk_snapshot = None;
        session.overwrite_hash = None;
        tracing::info!(path = %session.path.display(), bytes = bytes.len(), "saved document");
        self.remove_recovery_file(session);
        session.recovery_written = None;
        Ok(SaveOutcome::Saved)
    }

    pub fn is_dirty(&self, window: &str) -> bool {
        self.lock().get(window).is_some_and(|s| s.dirty)
    }

    /// Compares the file on disk with what was last read or saved. A
    /// changed file is remembered, so "load" and "compare" use exactly the
    /// version the user was told about.
    pub fn disk_state(&self, window: &str) -> DiskState {
        let Some(path) = self.lock().get(window).map(|s| s.path.clone()) else {
            return DiskState::Missing;
        };
        // Read without holding the lock; a slow disk mustn't block edits.
        let read = std::fs::read(&path);
        let mut sessions = self.lock();
        let Some(session) = sessions.get_mut(window) else {
            return DiskState::Missing;
        };
        match read {
            Ok(bytes) => {
                let hash = hash_bytes(&bytes);
                if hash == session.disk_hash || Some(hash) == session.overwrite_hash {
                    session.disk_snapshot = None;
                    DiskState::Unchanged
                } else {
                    session.disk_snapshot = Some(DiskSnapshot { hash, bytes });
                    DiskState::Changed
                }
            }
            Err(_) => {
                session.disk_snapshot = None;
                DiskState::Missing
            }
        }
    }

    /// The text of the changed disk version (without BOM), for comparing.
    pub fn disk_text(&self, window: &str) -> anyhow::Result<String> {
        let sessions = self.lock();
        let snapshot = sessions
            .get(window)
            .and_then(|s| s.disk_snapshot.as_ref())
            .context("there is no changed disk version to compare with")?;
        Ok(scrald_core::source::decode(&snapshot.bytes)?.text)
    }

    /// The text of the file as it is on disk now (without BOM), to compare
    /// recovered changes with.
    pub fn file_text(&self, window: &str) -> anyhow::Result<String> {
        let path = self
            .lock()
            .get(window)
            .map(|s| s.path.clone())
            .context("no document is open in this window")?;
        // Read without holding the lock, as `disk_state` does.
        let bytes =
            std::fs::read(&path).with_context(|| format!("could not read {}", path.display()))?;
        Ok(scrald_core::source::decode(&bytes)?.text)
    }

    /// "Keep mine": the user has seen the changed disk version and chose
    /// their own text, so the next save may replace it.
    pub fn keep_local(&self, window: &str) {
        // Rust note: a "let chain" (Rust 2024): `if let A && let B` runs the
        // body only if both patterns match, without nesting two `if let`s.
        // `Option::take()` moves the value out and leaves `None` behind, so
        // the snapshot is used once (like `std::exchange(x, nullopt)`).
        if let Some(session) = self.lock().get_mut(window)
            && let Some(snapshot) = session.disk_snapshot.take()
        {
            session.overwrite_hash = Some(snapshot.hash);
        }
    }

    /// "Load disk version": replaces the text with the changed disk version,
    /// which becomes the saved state. Recorded for undo like any edit, so
    /// Ctrl+Z brings the user's text back (as unsaved changes). Returns the
    /// path and new bytes, to parse.
    pub fn load_disk_version(&self, window: &str) -> anyhow::Result<(PathBuf, Vec<u8>)> {
        let snapshot = self
            .lock()
            .get_mut(window)
            .and_then(|s| s.disk_snapshot.take())
            .context("there is no changed disk version to load")?;
        let decoded = scrald_core::source::decode(&snapshot.bytes)?;
        self.edit(window, |_| Ok::<String, std::io::Error>(decoded.text))?;
        let mut sessions = self.lock();
        let session = sessions
            .get_mut(window)
            .context("no document is open in this window")?;
        session.has_bom = decoded.has_bom;
        session.disk_hash = snapshot.hash;
        session.overwrite_hash = None;
        session.update_dirty();
        Ok((session.path.clone(), session.bytes()))
    }

    /// The window closed. Its unsaved changes were saved or discarded (the
    /// user was asked), so its recovery file goes too.
    pub fn forget(&self, window: &str) {
        let _files = self.lock_recovery_files();
        let removed = self.lock().remove(window);
        if let Some(session) = removed {
            self.remove_recovery_file(&session);
        }
    }

    /// Brings every recovery file up to date: writes the text of documents
    /// with unsaved changes that changed since the last write, and deletes
    /// the files of documents that no longer have unsaved changes (say,
    /// after undoing back to the saved text). Runs every `RECOVERY_INTERVAL`.
    pub fn write_recovery(&self) {
        let Some(dir) = self.recovery_dir.get() else {
            return;
        };
        let _files = self.lock_recovery_files();
        // Copy what's needed under the lock, then write without it, so a
        // slow disk never holds up editing.
        let jobs: Vec<RecoveryJob> = self
            .lock()
            .iter()
            .filter_map(|(window, s)| {
                let window = window.clone();
                let id = s.recovery_id.clone();
                if s.dirty && s.recovery_written != Some(s.version) {
                    Some(RecoveryJob::Write {
                        window,
                        id,
                        version: s.version,
                        header: RecoveryHeader::new(s.path.clone(), s.disk_hash, now_millis()),
                        text: s.text.clone(),
                    })
                } else if !s.dirty && s.recovery_written.is_some() {
                    Some(RecoveryJob::Remove { window, id })
                } else {
                    None
                }
            })
            .collect();
        for job in jobs {
            let (window, written, result) = match job {
                RecoveryJob::Write {
                    window,
                    id,
                    version,
                    header,
                    text,
                } => {
                    let result = recovery::write(dir, &id, &header, &text);
                    (window, Some(version), result)
                }
                RecoveryJob::Remove { window, id } => (window, None, recovery::remove(dir, &id)),
            };
            match result {
                Ok(()) => {
                    if let Some(session) = self.lock().get_mut(&window) {
                        session.recovery_written = written;
                    }
                }
                // Tried again next time, since `recovery_written` is unchanged.
                Err(error) => tracing::warn!(error = %error, "could not update a recovery file"),
            }
        }
    }

    /// Recovery files that no open window is writing: left behind when
    /// Scrald stopped before the changes were saved or discarded. Newest
    /// first, and only those for `path` if given.
    pub fn orphaned_recoveries(&self, path: Option<&Path>) -> Vec<RecoveryEntry> {
        let Some(dir) = self.recovery_dir.get() else {
            return Vec::new();
        };
        let live: Vec<String> = self
            .lock()
            .values()
            .map(|s| s.recovery_id.clone())
            .collect();
        recovery::list(dir)
            .into_iter()
            .filter(|entry| !live.contains(&entry.id))
            .filter(|entry| path.is_none_or(|path| same_document(&entry.header.path, path)))
            .collect()
    }

    /// The header and text of an orphaned recovery file.
    pub fn recovery_text(&self, id: &str) -> anyhow::Result<(RecoveryHeader, String)> {
        let dir = self
            .recovery_dir
            .get()
            .context("crash recovery is not available")?;
        Ok(recovery::read(dir, id)?)
    }

    /// Restores an orphaned recovery file into `window`, which must have its
    /// document open: the text replaces the document's as an ordinary edit
    /// (unsaved, and Ctrl+Z goes back to the file's text). The window then
    /// keeps that recovery file up to date as its own. Returns the path and
    /// new bytes, to parse.
    pub fn restore_recovery(&self, window: &str, id: &str) -> anyhow::Result<(PathBuf, Vec<u8>)> {
        let _files = self.lock_recovery_files();
        let path = self
            .lock()
            .get(window)
            .map(|s| s.path.clone())
            .context("no document is open in this window")?;
        if !self
            .orphaned_recoveries(Some(&path))
            .iter()
            .any(|e| e.id == id)
        {
            anyhow::bail!("those recovered changes are not for this document, or are in use");
        }
        let (_, text) = self.recovery_text(id)?;
        self.edit(window, |_| Ok::<String, std::io::Error>(text))?;
        let mut sessions = self.lock();
        let session = sessions
            .get_mut(window)
            .context("no document is open in this window")?;
        self.remove_recovery_file(session);
        session.recovery_id = id.to_string();
        // The file holds exactly this text now.
        session.recovery_written = Some(session.version);
        Ok((session.path.clone(), session.bytes()))
    }

    /// Deletes an orphaned recovery file: the user chose not to restore it.
    pub fn discard_recovery(&self, id: &str) -> anyhow::Result<()> {
        let _files = self.lock_recovery_files();
        if !self.orphaned_recoveries(None).iter().any(|e| e.id == id) {
            anyhow::bail!("those recovered changes are in use");
        }
        let dir = self
            .recovery_dir
            .get()
            .context("crash recovery is not available")?;
        Ok(recovery::remove(dir, id)?)
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

    /// Sessions writing recovery files into a fresh folder.
    fn with_recovery(name: &str) -> (Sessions, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "scrald-session-recovery-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let sessions = Sessions::default();
        sessions.set_recovery_dir(dir.clone());
        (sessions, dir)
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
    fn a_search_index_is_given_back_only_to_its_own_model() {
        use scrald_core::search::SearchOptions;
        let path = temp_file(
            "search.md",
            b"Saga text.
",
        );
        let sessions = Sessions::default();
        sessions.open("main", &path).unwrap();
        let send = |text: &str| {
            let mut doc = scrald_core::parse_document(path.clone(), text.as_bytes()).unwrap();
            sessions.set_search("main", SearchIndex::new(&doc));
            sessions.send_blocks("main", &mut doc.blocks, false).model
        };
        let first = send(
            "Saga text.
",
        );

        let (model, mut index) = sessions.take_search("main").unwrap();
        assert_eq!(model, first);
        assert_eq!(index.search("saga", SearchOptions::default()).total, 1);
        // Only one search at a time.
        assert!(sessions.take_search("main").is_none());
        sessions.put_search("main", model, index);

        // A newer model arrives while a search of the old one runs: the old
        // index isn't put back over the new one.
        let (old, index) = sessions.take_search("main").unwrap();
        send(
            "Saga, saga.
",
        );
        sessions.put_search("main", old, index);
        let (_, mut index) = sessions.take_search("main").unwrap();
        assert_eq!(index.search("saga", SearchOptions::default()).total, 2);
    }

    #[test]
    fn models_after_the_first_leave_out_shown_html() {
        let path = temp_file(
            "models.md",
            b"One.

Two.
",
        );
        let sessions = Sessions::default();
        let parse =
            |text: &str| scrald_core::parse_document(path.clone(), text.as_bytes()).unwrap();
        // No document: nothing to build on.
        let mut blocks = parse(
            "One.
",
        )
        .blocks;
        assert_eq!(sessions.send_blocks("main", &mut blocks, false).base, None);

        sessions.open("main", &path).unwrap();
        let mut blocks = parse(
            "One.

Two.
",
        )
        .blocks;
        let first = sessions.send_blocks("main", &mut blocks, false);
        // The first model for a document is sent whole.
        assert_eq!(first.base, None);
        assert!(blocks.iter().all(|b| !b.html.is_empty()));

        let mut blocks = parse(
            "One.

Two, edited.
",
        )
        .blocks;
        let second = sessions.send_blocks("main", &mut blocks, false);
        assert_eq!(second.base, Some(first.model));
        assert_eq!(second.html_from, vec![Some(0), None]);
        assert_eq!(blocks[0].html, "");

        // `full` sends everything again (the window lost track).
        let mut blocks = parse(
            "One.

Two, edited.
",
        )
        .blocks;
        let third = sessions.send_blocks("main", &mut blocks, true);
        assert_eq!(third.base, None);
        assert!(third.model > second.model);
        assert!(blocks.iter().all(|b| !b.html.is_empty()));

        // Opening the document again starts over.
        sessions.open("main", &path).unwrap();
        let mut blocks = parse(
            "One.

Two.
",
        )
        .blocks;
        assert_eq!(sessions.send_blocks("main", &mut blocks, false).base, None);
    }

    #[test]
    fn undo_and_redo_step_through_changes() {
        let path = temp_file("undo.md", b"one");
        let sessions = Sessions::default();
        sessions.open("w", &path).unwrap();
        let append = |s: &str| -> Result<String, std::io::Error> { Ok(format!("{s} two")) };
        sessions.edit("w", append).unwrap();
        sessions.edit("w", upper).unwrap();
        assert_eq!(sessions.current("w").unwrap().1, b"ONE TWO");

        let (_, bytes) = sessions.step("w", Step::Undo).unwrap().unwrap();
        assert_eq!(bytes, b"one two");
        let (_, bytes) = sessions.step("w", Step::Undo).unwrap().unwrap();
        assert_eq!(bytes, b"one");
        // Back at the text on disk: not unsaved any more.
        assert!(!sessions.is_dirty("w"));
        assert!(sessions.step("w", Step::Undo).unwrap().is_none());

        let (_, bytes) = sessions.step("w", Step::Redo).unwrap().unwrap();
        assert_eq!(bytes, b"one two");
        assert!(sessions.is_dirty("w"));
        // A new edit clears what could be redone.
        sessions.edit("w", upper).unwrap();
        assert!(sessions.step("w", Step::Redo).unwrap().is_none());
    }

    #[test]
    fn raw_text_replacement_checks_the_version() {
        let path = temp_file("raw.md", b"one\r\ntwo\r\n");
        let sessions = Sessions::default();
        sessions.open("w", &path).unwrap();
        let (text, version) = sessions.text("w").unwrap();
        assert_eq!(text, "one\r\ntwo\r\n");
        let (v2, dirty) = sessions
            .replace_text("w", version, "one\r\n2\r\n".into())
            .unwrap();
        assert!(dirty && v2 > version);
        // Old version: refused, text unchanged.
        assert!(sessions.replace_text("w", version, "x".into()).is_err());
        assert_eq!(sessions.text("w").unwrap().0, "one\r\n2\r\n");
        // Undo works on raw replacements too.
        let (_, bytes) = sessions.step("w", Step::Undo).unwrap().unwrap();
        assert_eq!(bytes, b"one\r\ntwo\r\n");
    }

    #[test]
    fn block_source_counts_the_bom() {
        let path = temp_file("source.md", b"\xEF\xBB\xBF# Hi\n\nText.\n");
        let sessions = Sessions::default();
        sessions.open("w", &path).unwrap();
        // "Text." starts at byte 9 of the file (3 BOM + 6).
        assert_eq!(sessions.source("w", 9, 14).as_deref(), Some("Text."));
        assert_eq!(sessions.bom_len("w"), 3);
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

    #[test]
    fn saving_over_an_unseen_disk_change_is_refused() {
        let path = temp_file("conflict-save.md", b"mine");
        let sessions = Sessions::default();
        sessions.open("w", &path).unwrap();
        sessions.edit("w", upper).unwrap();
        // Changed on disk before the watcher reported it.
        std::fs::write(&path, b"theirs").unwrap();
        assert_eq!(sessions.save("w").unwrap(), SaveOutcome::Conflict);
        assert_eq!(std::fs::read(&path).unwrap(), b"theirs");
        assert!(sessions.is_dirty("w"));
        // The refused save remembered the disk version for comparing.
        assert_eq!(sessions.disk_text("w").unwrap(), "theirs");
    }

    #[test]
    fn keep_mine_allows_overwriting_that_version_only() {
        let path = temp_file("conflict-keep.md", b"mine");
        let sessions = Sessions::default();
        sessions.open("w", &path).unwrap();
        sessions.edit("w", upper).unwrap();
        std::fs::write(&path, b"theirs").unwrap();
        assert_eq!(sessions.disk_state("w"), DiskState::Changed);
        sessions.keep_local("w");
        // The version the user chose to overwrite no longer counts as a change.
        assert_eq!(sessions.disk_state("w"), DiskState::Unchanged);
        // A further change does, and blocks the save again.
        std::fs::write(&path, b"theirs, again").unwrap();
        assert_eq!(sessions.disk_state("w"), DiskState::Changed);
        assert_eq!(sessions.save("w").unwrap(), SaveOutcome::Conflict);
        sessions.keep_local("w");
        assert_eq!(sessions.save("w").unwrap(), SaveOutcome::Saved);
        assert_eq!(std::fs::read(&path).unwrap(), b"MINE");
        assert!(!sessions.is_dirty("w"));
    }

    #[test]
    fn loading_the_disk_version_is_undoable() {
        let path = temp_file("conflict-load.md", b"mine\r\n");
        let sessions = Sessions::default();
        sessions.open("w", &path).unwrap();
        sessions.edit("w", upper).unwrap();
        std::fs::write(&path, b"\xEF\xBB\xBFtheirs\r\n").unwrap();
        assert_eq!(sessions.disk_state("w"), DiskState::Changed);

        let (_, bytes) = sessions.load_disk_version("w").unwrap();
        // Exactly the disk version, BOM included, and it counts as saved.
        assert_eq!(bytes, b"\xEF\xBB\xBFtheirs\r\n");
        assert!(!sessions.is_dirty("w"));
        assert_eq!(sessions.disk_state("w"), DiskState::Unchanged);

        // Undo brings the user's text back, as unsaved changes.
        let (_, bytes) = sessions.step("w", Step::Undo).unwrap().unwrap();
        assert!(bytes.ends_with(b"MINE\r\n"));
        assert!(sessions.is_dirty("w"));
        // Nothing left to load.
        assert!(sessions.load_disk_version("w").is_err());
    }

    #[test]
    fn file_text_reads_the_file_as_it_is_now() {
        let path = temp_file("file-text.md", b"\xEF\xBB\xBFone\r\n");
        let sessions = Sessions::default();
        sessions.open("w", &path).unwrap();
        sessions.edit("w", upper).unwrap();
        // Not the edited text, and without the BOM.
        assert_eq!(sessions.file_text("w").unwrap(), "one\r\n");
        std::fs::write(&path, b"two\n").unwrap();
        assert_eq!(sessions.file_text("w").unwrap(), "two\n");
        std::fs::remove_file(&path).unwrap();
        assert!(sessions.file_text("w").is_err());
        assert!(sessions.file_text("other").is_err());
    }

    #[test]
    fn unsaved_text_is_kept_in_a_recovery_file_until_saved() {
        let (sessions, dir) = with_recovery("save");
        let path = temp_file("recover-save.md", b"one\r\n");
        sessions.open("w", &path).unwrap();
        sessions.write_recovery();
        assert!(recovery::list(&dir).is_empty(), "nothing unsaved yet");

        sessions.edit("w", upper).unwrap();
        sessions.write_recovery();
        let files = recovery::list(&dir);
        assert_eq!(files.len(), 1);
        let (header, text) = recovery::read(&dir, &files[0].id).unwrap();
        assert_eq!(text, "ONE\r\n");
        assert_eq!(header.path, path);
        assert_eq!(header.base_hash, scrald_core::hash::fnv1a(b"one\r\n"));
        // Still in use by the window, so not offered for restoring.
        assert!(sessions.orphaned_recoveries(None).is_empty());

        sessions.save("w").unwrap();
        assert!(recovery::list(&dir).is_empty());
    }

    #[test]
    fn undoing_back_to_the_saved_text_deletes_the_recovery_file() {
        let (sessions, dir) = with_recovery("undo");
        let path = temp_file("recover-undo.md", b"one");
        sessions.open("w", &path).unwrap();
        sessions.edit("w", upper).unwrap();
        sessions.write_recovery();
        assert_eq!(recovery::list(&dir).len(), 1);
        sessions.step("w", Step::Undo).unwrap();
        sessions.write_recovery();
        assert!(recovery::list(&dir).is_empty());
    }

    #[test]
    fn discarding_deletes_the_recovery_file() {
        let (sessions, dir) = with_recovery("discard");
        let path = temp_file("recover-discard.md", b"one");
        let other = temp_file("recover-other.md", b"two");
        sessions.open("w", &path).unwrap();
        sessions.edit("w", upper).unwrap();
        sessions.write_recovery();
        // Opening another document in the window ("Don't save").
        sessions.open("w", &other).unwrap();
        assert!(recovery::list(&dir).is_empty());

        sessions.edit("w", upper).unwrap();
        sessions.write_recovery();
        // Closing the window.
        sessions.forget("w");
        assert!(recovery::list(&dir).is_empty());
    }

    #[test]
    fn a_crash_leaves_a_file_that_can_be_restored_and_undone() {
        let (crashed, dir) = with_recovery("restore");
        let path = temp_file("recover-restore.md", b"one\n");
        crashed.open("w", &path).unwrap();
        crashed.edit("w", upper).unwrap();
        crashed.write_recovery();
        // Rust note: `std::mem::forget` drops nothing and runs no cleanup,
        // standing in for a process that stopped without closing its windows.
        std::mem::forget(crashed);

        let sessions = Sessions::default();
        sessions.set_recovery_dir(dir.clone());
        let orphans = sessions.orphaned_recoveries(Some(&path));
        assert_eq!(orphans.len(), 1);
        let id = orphans[0].id.clone();
        // Only offered for its own document.
        let other = temp_file("recover-unrelated.md", b"x");
        assert!(sessions.orphaned_recoveries(Some(&other)).is_empty());

        sessions.open("w", &path).unwrap();
        let (_, bytes) = sessions.restore_recovery("w", &id).unwrap();
        assert_eq!(bytes, b"ONE\n");
        assert!(sessions.is_dirty("w"));
        // The window took the file over: no longer an orphan, and kept.
        assert!(sessions.orphaned_recoveries(None).is_empty());
        assert_eq!(recovery::list(&dir).len(), 1);
        assert!(sessions.discard_recovery(&id).is_err());

        // Undo goes back to the file's text, and the file goes away.
        let (_, bytes) = sessions.step("w", Step::Undo).unwrap().unwrap();
        assert_eq!(bytes, b"one\n");
        sessions.write_recovery();
        assert!(recovery::list(&dir).is_empty());
    }

    #[test]
    fn an_orphaned_recovery_file_can_be_discarded() {
        let (crashed, dir) = with_recovery("discard-orphan");
        let path = temp_file("recover-discard-orphan.md", b"one");
        crashed.open("w", &path).unwrap();
        crashed.edit("w", upper).unwrap();
        crashed.write_recovery();
        std::mem::forget(crashed);

        let sessions = Sessions::default();
        sessions.set_recovery_dir(dir.clone());
        let id = sessions.orphaned_recoveries(None)[0].id.clone();
        // It isn't for another document.
        let other = temp_file("recover-discard-other.md", b"x");
        sessions.open("w", &other).unwrap();
        assert!(sessions.restore_recovery("w", &id).is_err());
        sessions.discard_recovery(&id).unwrap();
        assert!(recovery::list(&dir).is_empty());
    }
}
