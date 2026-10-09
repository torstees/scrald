//! Persistent app state in SQLite (DESIGN.md §12): what Scrald remembers
//! about each document, recent documents, and global settings. Only
//! metadata is stored, never document contents.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use rusqlite::{Connection, OptionalExtension, params};
use scrald_core::Flavor;
use scrald_core::theme::TextSizing;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Schema migrations, applied in order. `PRAGMA user_version` records how
/// many have run, so each runs exactly once per database.
const MIGRATIONS: &[&str] = &[
    // 1: documents and settings tables.
    "CREATE TABLE documents (
        path            TEXT PRIMARY KEY,   -- canonical absolute path
        identity_size   INTEGER,            -- file size, to reconnect moved files
        identity_hash   TEXT,               -- hash of the first KB, likewise
        theme           TEXT,
        flavor          TEXT,
        scroll_offset   INTEGER,
        scroll_fraction REAL,
        text_sizing     TEXT,
        zoom            REAL,
        window_x        INTEGER,
        window_y        INTEGER,
        window_width    INTEGER,
        window_height   INTEGER,
        window_maximized INTEGER,
        remote_images   INTEGER NOT NULL DEFAULT 0,
        last_opened     INTEGER NOT NULL,   -- Unix time, milliseconds
        pinned          INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX documents_last_opened ON documents (last_opened DESC);
    CREATE INDEX documents_identity ON documents (identity_size, identity_hash);
    CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL                 -- JSON
    );",
];

/// Setting key for the most recently used window size (DESIGN.md §11).
const LAST_WINDOW: &str = "window.last";

/// Setting key for the global default theme id (DESIGN.md §7.6).
pub const DEFAULT_THEME: &str = "theme.default";

/// Setting key for the global typography defaults (DESIGN.md §7.4).
const TYPOGRAPHY: &str = "typography";

/// Zoom is a multiplier on the base font size; outside this range the
/// reader stops being useful.
pub const MIN_ZOOM: f64 = 0.5;
pub const MAX_ZOOM: f64 = 3.0;

/// Global typography settings: what "Make this the default" stores, plus the
/// per-user "fill window" toggle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypographyDefaults {
    /// `None` means "use the theme's suggestion".
    pub text_sizing: Option<TextSizing>,
    /// `None` means 100%.
    pub zoom: Option<f64>,
    /// Let text run the full window width, ignoring the measure.
    #[serde(default)]
    pub fill_window: bool,
}

/// A reading position (DESIGN.md §4); mirrors `ScrollAnchor` in ui/src/reader/anchor.ts.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollAnchor {
    pub offset: usize,
    pub fraction: f64,
}

/// What Scrald remembers about a document. More fields arrive with themes,
/// zoom, and text sizing.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentMemory {
    /// Where the reader left off, if the document was opened before.
    pub anchor: Option<ScrollAnchor>,
    /// Whether the user allowed remote images for this document.
    pub remote_images: bool,
    /// Theme the user picked for this document in Scrald, if any.
    pub theme: Option<String>,
    /// Text sizing mode chosen for this document, if any.
    pub text_sizing: Option<TextSizing>,
    /// Zoom chosen for this document, if any.
    pub zoom: Option<f64>,
    /// Flavor chosen for this document, if any (detected flavors aren't stored).
    pub flavor: Option<Flavor>,
}

/// A window's position and size in physical pixels, plus whether it was
/// maximized. Position and size are the normal (un-maximized) bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

/// A row for the start screen's recent documents list (M7).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentDocument {
    pub path: String,
    pub last_opened: i64,
    pub pinned: bool,
}

/// The state database. All access goes through one connection behind a lock.
// Rust note: `rusqlite::Connection` can move between threads but not be used
// by two at once, so it lives inside a `Mutex`, like the asset registry.
pub struct StateStore {
    conn: Mutex<Connection>,
}

impl StateStore {
    /// Opens (or creates) the database at `path` and applies migrations.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("could not create {}", dir.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("could not open state database {}", path.display()))?;
        Self::with_connection(conn)
    }

    /// A throwaway database in memory: for tests, and as a fallback when the
    /// real one can't be opened (Scrald then just forgets on exit).
    pub fn in_memory() -> anyhow::Result<Self> {
        Self::with_connection(Connection::open_in_memory()?)
    }

    fn with_connection(mut conn: Connection) -> anyhow::Result<Self> {
        migrate(&mut conn)?;
        Ok(StateStore {
            conn: Mutex::new(conn),
        })
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Records that `path` was opened now and returns what's remembered
    /// about it. A file that was moved or renamed since it was last seen is
    /// reconnected to its old row by size and first-KB hash (DESIGN.md §12).
    pub fn record_open(&self, path: &Path) -> anyhow::Result<DocumentMemory> {
        let key = document_key(path);
        let identity = file_identity(path);
        let conn = self.conn();

        let known: bool = conn
            .query_row("SELECT 1 FROM documents WHERE path = ?1", [&key], |_| {
                Ok(true)
            })
            .optional()?
            .unwrap_or(false);
        if !known && let Some((size, hash)) = &identity {
            reconnect_moved(&conn, &key, *size, hash)?;
        }

        let (size, hash) = identity.unzip();
        conn.execute(
            "INSERT INTO documents (path, identity_size, identity_hash, last_opened)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (path) DO UPDATE SET
                identity_size = excluded.identity_size,
                identity_hash = excluded.identity_hash,
                last_opened = excluded.last_opened",
            params![key, size, hash, now_millis()],
        )?;

        let memory = conn.query_row(
            "SELECT scroll_offset, scroll_fraction, remote_images, theme, text_sizing, zoom, flavor
             FROM documents WHERE path = ?1",
            [&key],
            |row| {
                let offset: Option<i64> = row.get(0)?;
                let fraction: Option<f64> = row.get(1)?;
                // Rust note: `zip` pairs two Options: `Some` only if both are.
                let anchor = offset.zip(fraction).map(|(offset, fraction)| ScrollAnchor {
                    offset: usize::try_from(offset).unwrap_or(0),
                    fraction,
                });
                Ok(DocumentMemory {
                    anchor,
                    remote_images: row.get(2)?,
                    theme: row.get(3)?,
                    text_sizing: row
                        .get::<_, Option<String>>(4)?
                        .as_deref()
                        .and_then(parse_text_sizing),
                    zoom: row.get(5)?,
                    flavor: row
                        .get::<_, Option<String>>(6)?
                        .as_deref()
                        .and_then(Flavor::from_name),
                })
            },
        )?;
        Ok(memory)
    }

    pub fn save_reading_position(&self, path: &Path, anchor: ScrollAnchor) -> anyhow::Result<()> {
        self.conn().execute(
            "UPDATE documents SET scroll_offset = ?2, scroll_fraction = ?3 WHERE path = ?1",
            params![document_key(path), anchor.offset as i64, anchor.fraction],
        )?;
        Ok(())
    }

    pub fn set_remote_images(&self, path: &Path, allowed: bool) -> anyhow::Result<()> {
        self.conn().execute(
            "UPDATE documents SET remote_images = ?2 WHERE path = ?1",
            params![document_key(path), allowed],
        )?;
        Ok(())
    }

    /// Sets (or with `None`, clears) the theme chosen for a document.
    pub fn set_document_theme(&self, path: &Path, theme: Option<&str>) -> anyhow::Result<()> {
        self.conn().execute(
            "UPDATE documents SET theme = ?2 WHERE path = ?1",
            params![document_key(path), theme],
        )?;
        Ok(())
    }

    /// Sets (or with `None`, clears) a document's text sizing mode and zoom.
    pub fn set_document_typography(
        &self,
        path: &Path,
        text_sizing: Option<TextSizing>,
        zoom: Option<f64>,
    ) -> anyhow::Result<()> {
        self.conn().execute(
            "UPDATE documents SET text_sizing = ?2, zoom = ?3 WHERE path = ?1",
            params![
                document_key(path),
                text_sizing.map(text_sizing_name),
                zoom.map(clamp_zoom)
            ],
        )?;
        Ok(())
    }

    /// Sets (or with `None`, clears) the flavor chosen for a document. Only
    /// explicit choices are stored; detection runs fresh each time.
    pub fn set_document_flavor(&self, path: &Path, flavor: Option<Flavor>) -> anyhow::Result<()> {
        self.conn().execute(
            "UPDATE documents SET flavor = ?2 WHERE path = ?1",
            params![document_key(path), flavor.map(Flavor::name)],
        )?;
        Ok(())
    }

    pub fn typography_defaults(&self) -> anyhow::Result<TypographyDefaults> {
        Ok(self.get_setting(TYPOGRAPHY)?.unwrap_or_default())
    }

    pub fn set_typography_defaults(&self, defaults: TypographyDefaults) -> anyhow::Result<()> {
        let defaults = TypographyDefaults {
            zoom: defaults.zoom.map(clamp_zoom),
            ..defaults
        };
        self.set_setting(TYPOGRAPHY, &defaults)
    }

    /// Saves a window's geometry for the document it showed (if any) and as
    /// the most recently used window, which new documents start from.
    pub fn save_window(&self, path: Option<&Path>, geometry: WindowGeometry) -> anyhow::Result<()> {
        if let Some(path) = path {
            self.conn().execute(
                "UPDATE documents SET window_x = ?2, window_y = ?3, window_width = ?4,
                    window_height = ?5, window_maximized = ?6 WHERE path = ?1",
                params![
                    document_key(path),
                    geometry.x,
                    geometry.y,
                    geometry.width,
                    geometry.height,
                    geometry.maximized
                ],
            )?;
        }
        self.set_setting(LAST_WINDOW, &geometry)
    }

    /// Where a window for `path` should open: the document's own remembered
    /// geometry, else the last window used. `None` means use the defaults.
    pub fn window_for(&self, path: Option<&Path>) -> anyhow::Result<Option<WindowGeometry>> {
        if let Some(path) = path {
            let own = self
                .conn()
                .query_row(
                    "SELECT window_x, window_y, window_width, window_height, window_maximized
                     FROM documents WHERE path = ?1 AND window_width IS NOT NULL",
                    [document_key(path)],
                    |row| {
                        Ok(WindowGeometry {
                            x: row.get(0)?,
                            y: row.get(1)?,
                            width: row.get(2)?,
                            height: row.get(3)?,
                            maximized: row.get(4)?,
                        })
                    },
                )
                .optional()?;
            if own.is_some() {
                return Ok(own);
            }
        }
        self.get_setting(LAST_WINDOW)
    }

    /// Most recently opened documents first, pinned ones before the rest.
    pub fn recent_documents(&self, limit: usize) -> anyhow::Result<Vec<RecentDocument>> {
        let conn = self.conn();
        let mut statement = conn.prepare(
            "SELECT path, last_opened, pinned FROM documents
             ORDER BY pinned DESC, last_opened DESC LIMIT ?1",
        )?;
        let rows = statement.query_map([limit as i64], |row| {
            Ok(RecentDocument {
                path: row.get(0)?,
                last_opened: row.get(1)?,
                pinned: row.get(2)?,
            })
        })?;
        // Rust note: collecting an iterator of `Result`s into `Result<Vec<_>, _>`
        // stops at the first error, like a loop with an early `return Err`.
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Reads a JSON-encoded setting; `None` if unset or unreadable.
    // Rust note: `T: DeserializeOwned` means "any type serde can build from
    // data without borrowing it", so callers choose the type: `get_setting::<f64>`.
    pub fn get_setting<T: DeserializeOwned>(&self, key: &str) -> anyhow::Result<Option<T>> {
        let raw: Option<String> = self
            .conn()
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?;
        Ok(raw.and_then(|json| serde_json::from_str(&json).ok()))
    }

    pub fn set_setting<T: Serialize>(&self, key: &str, value: &T) -> anyhow::Result<()> {
        self.conn().execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![key, serde_json::to_string(value)?],
        )?;
        Ok(())
    }
}

fn text_sizing_name(mode: TextSizing) -> &'static str {
    match mode {
        TextSizing::Fixed => "fixed",
        TextSizing::Fit => "fit",
    }
}

fn parse_text_sizing(name: &str) -> Option<TextSizing> {
    match name {
        "fixed" => Some(TextSizing::Fixed),
        "fit" => Some(TextSizing::Fit),
        _ => None,
    }
}

/// Keeps zoom in range, and treats a non-number as 100%.
pub fn clamp_zoom(zoom: f64) -> f64 {
    if zoom.is_finite() {
        zoom.clamp(MIN_ZOOM, MAX_ZOOM)
    } else {
        1.0
    }
}

fn migrate(conn: &mut Connection) -> anyhow::Result<()> {
    let applied: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let applied = usize::try_from(applied).unwrap_or(0);
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(applied) {
        // Each migration and its version bump commit together, or not at all.
        let tx = conn.transaction()?;
        tx.execute_batch(sql)
            .with_context(|| format!("state database migration {} failed", index + 1))?;
        tx.pragma_update(None, "user_version", (index + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

/// If a remembered document with the same identity no longer exists at its
/// old path, moves its row to `key` so its memory follows the file.
fn reconnect_moved(conn: &Connection, key: &str, size: i64, hash: &str) -> anyhow::Result<()> {
    let mut statement = conn.prepare(
        "SELECT path FROM documents WHERE identity_size = ?1 AND identity_hash = ?2
         ORDER BY last_opened DESC",
    )?;
    let candidates = statement
        .query_map(params![size, hash], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(old) = candidates.iter().find(|p| !Path::new(p).exists()) {
        tracing::info!(from = %old, to = %key, "reconnecting moved document");
        conn.execute(
            "UPDATE documents SET path = ?2 WHERE path = ?1",
            params![old, key],
        )?;
    }
    Ok(())
}

/// The database key for a document: its canonical absolute path as text.
/// Falls back to the path as given if it can't be canonicalized (say, the
/// file was deleted).
pub fn document_key(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    strip_verbatim_prefix(&canonical).display().to_string()
}

/// Windows `canonicalize` returns `\\?\C:\...`; the plain `C:\...` form is
/// what users and other APIs expect. UNC paths (`\\?\UNC\...`) are left alone.
fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => path.to_path_buf(),
    }
}

/// Size and a hash of the first KB: a cheap fingerprint to recognize a file
/// after it moves. `None` if the file can't be read.
fn file_identity(path: &Path) -> Option<(i64, String)> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).ok()?;
    let size = i64::try_from(file.metadata().ok()?.len()).ok()?;
    let mut head = Vec::with_capacity(1024);
    file.by_ref().take(1024).read_to_end(&mut head).ok()?;
    Some((size, format!("{:016x}", fnv1a(&head))))
}

/// FNV-1a, a tiny hash that, unlike `DefaultHasher`, gives the same value in
/// every Rust version, which matters for values stored on disk.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh temp directory for one test.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scrald-state-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn doc(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn first_open_has_no_memory() {
        let dir = temp_dir("first");
        let store = StateStore::in_memory().unwrap();
        let memory = store.record_open(&doc(&dir, "a.md", "# A")).unwrap();
        assert_eq!(memory, DocumentMemory::default());
    }

    #[test]
    fn remembers_position_and_remote_images() {
        let dir = temp_dir("remember");
        let path = doc(&dir, "a.md", "# A");
        let store = StateStore::in_memory().unwrap();
        store.record_open(&path).unwrap();
        let anchor = ScrollAnchor {
            offset: 1234,
            fraction: 0.25,
        };
        store.save_reading_position(&path, anchor).unwrap();
        store.set_remote_images(&path, true).unwrap();

        let memory = store.record_open(&path).unwrap();
        assert_eq!(memory.anchor, Some(anchor));
        assert!(memory.remote_images);
    }

    #[test]
    fn remembers_and_clears_document_theme() {
        let dir = temp_dir("theme");
        let path = doc(&dir, "a.md", "# A");
        let store = StateStore::in_memory().unwrap();
        store.record_open(&path).unwrap();
        store.set_document_theme(&path, Some("sepia")).unwrap();
        assert_eq!(
            store.record_open(&path).unwrap().theme.as_deref(),
            Some("sepia")
        );
        store.set_document_theme(&path, None).unwrap();
        assert_eq!(store.record_open(&path).unwrap().theme, None);
    }

    #[test]
    fn remembers_document_typography_clamped() {
        let dir = temp_dir("typography");
        let path = doc(&dir, "a.md", "# A");
        let store = StateStore::in_memory().unwrap();
        store.record_open(&path).unwrap();
        store
            .set_document_typography(&path, Some(TextSizing::Fit), Some(9.0))
            .unwrap();
        let memory = store.record_open(&path).unwrap();
        assert_eq!(memory.text_sizing, Some(TextSizing::Fit));
        assert_eq!(memory.zoom, Some(MAX_ZOOM));
        store.set_document_typography(&path, None, None).unwrap();
        let memory = store.record_open(&path).unwrap();
        assert_eq!((memory.text_sizing, memory.zoom), (None, None));
    }

    #[test]
    fn remembers_and_clears_document_flavor() {
        let dir = temp_dir("flavor");
        let path = doc(&dir, "a.md", "# A");
        let store = StateStore::in_memory().unwrap();
        store.record_open(&path).unwrap();
        store
            .set_document_flavor(&path, Some(Flavor::Pandoc))
            .unwrap();
        assert_eq!(
            store.record_open(&path).unwrap().flavor,
            Some(Flavor::Pandoc)
        );
        store.set_document_flavor(&path, None).unwrap();
        assert_eq!(store.record_open(&path).unwrap().flavor, None);
    }

    #[test]
    fn typography_defaults_round_trip() {
        let store = StateStore::in_memory().unwrap();
        assert_eq!(
            store.typography_defaults().unwrap(),
            TypographyDefaults::default()
        );
        let defaults = TypographyDefaults {
            text_sizing: Some(TextSizing::Fit),
            zoom: Some(0.1),
            fill_window: true,
        };
        store.set_typography_defaults(defaults).unwrap();
        let stored = store.typography_defaults().unwrap();
        assert_eq!(stored.zoom, Some(MIN_ZOOM));
        assert!(stored.fill_window);
        assert_eq!(stored.text_sizing, Some(TextSizing::Fit));
    }

    #[test]
    fn same_file_through_different_paths_is_one_document() {
        let dir = temp_dir("canonical");
        let path = doc(&dir, "a.md", "# A");
        let store = StateStore::in_memory().unwrap();
        store.record_open(&path).unwrap();
        store
            .save_reading_position(
                &path,
                ScrollAnchor {
                    offset: 7,
                    fraction: 0.0,
                },
            )
            .unwrap();
        let roundabout = dir.join(".").join("a.md");
        assert_eq!(
            store
                .record_open(&roundabout)
                .unwrap()
                .anchor
                .map(|a| a.offset),
            Some(7)
        );
    }

    #[test]
    fn moved_file_keeps_its_memory() {
        let dir = temp_dir("moved");
        let old = doc(&dir, "draft.md", "# The Long Winter\n\nChapter one.");
        let store = StateStore::in_memory().unwrap();
        store.record_open(&old).unwrap();
        store
            .save_reading_position(
                &old,
                ScrollAnchor {
                    offset: 42,
                    fraction: 0.5,
                },
            )
            .unwrap();

        let new = dir.join("final.md");
        std::fs::rename(&old, &new).unwrap();
        let memory = store.record_open(&new).unwrap();
        assert_eq!(memory.anchor.map(|a| a.offset), Some(42));
        assert_eq!(store.recent_documents(10).unwrap().len(), 1);
    }

    #[test]
    fn copies_do_not_steal_memory() {
        let dir = temp_dir("copy");
        let original = doc(&dir, "a.md", "same text");
        let store = StateStore::in_memory().unwrap();
        store.record_open(&original).unwrap();
        store
            .save_reading_position(
                &original,
                ScrollAnchor {
                    offset: 9,
                    fraction: 0.0,
                },
            )
            .unwrap();
        // The original still exists, so the copy is a different document.
        let copy = doc(&dir, "b.md", "same text");
        assert_eq!(store.record_open(&copy).unwrap().anchor, None);
        assert_eq!(store.recent_documents(10).unwrap().len(), 2);
    }

    #[test]
    fn window_geometry_per_document_then_last_used() {
        let dir = temp_dir("window");
        let a = doc(&dir, "a.md", "a");
        let b = doc(&dir, "b.md", "b");
        let store = StateStore::in_memory().unwrap();
        assert_eq!(store.window_for(Some(&a)).unwrap(), None);

        store.record_open(&a).unwrap();
        let geometry = WindowGeometry {
            x: 10,
            y: 20,
            width: 900,
            height: 700,
            maximized: false,
        };
        store.save_window(Some(&a), geometry).unwrap();
        assert_eq!(store.window_for(Some(&a)).unwrap(), Some(geometry));
        // A document never opened before starts from the last window.
        assert_eq!(store.window_for(Some(&b)).unwrap(), Some(geometry));
        assert_eq!(store.window_for(None).unwrap(), Some(geometry));
    }

    #[test]
    fn recent_documents_newest_first() {
        let dir = temp_dir("recent");
        let store = StateStore::in_memory().unwrap();
        for name in ["one.md", "two.md", "three.md"] {
            store.record_open(&doc(&dir, name, name)).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let names: Vec<String> = store
            .recent_documents(2)
            .unwrap()
            .into_iter()
            .map(|r| {
                Path::new(&r.path)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(names, ["three.md", "two.md"]);
    }

    #[test]
    fn settings_round_trip_as_json() {
        let store = StateStore::in_memory().unwrap();
        assert_eq!(store.get_setting::<f64>("zoom").unwrap(), None);
        store.set_setting("zoom", &1.25).unwrap();
        assert_eq!(store.get_setting::<f64>("zoom").unwrap(), Some(1.25));
    }

    #[test]
    fn database_on_disk_survives_reopen_and_migrates_once() {
        let dir = temp_dir("disk");
        let db = dir.join("state").join("scrald.db");
        let path = doc(&dir, "a.md", "a");
        {
            let store = StateStore::open(&db).unwrap();
            store.record_open(&path).unwrap();
            store.set_remote_images(&path, true).unwrap();
        }
        let store = StateStore::open(&db).unwrap();
        assert!(store.record_open(&path).unwrap().remote_images);
    }

    #[test]
    fn verbatim_prefix_is_stripped_for_drive_paths_only() {
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"\\?\C:\docs\a.md")),
            PathBuf::from(r"C:\docs\a.md")
        );
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"\\?\UNC\server\share\a.md")),
            PathBuf::from(r"\\?\UNC\server\share\a.md")
        );
    }

    #[test]
    fn fnv1a_is_stable() {
        // Known FNV-1a 64-bit values.
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    }
}
