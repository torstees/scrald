//! Crash-recovery files (DESIGN.md §9.3): copies of unsaved text, kept in the
//! app's data folder (never next to the document) until the changes are
//! saved or discarded.
//!
//! Each file is `<id>.recovery`: one line of JSON describing it, a line
//! break, then the document's text exactly as it was in memory (without a
//! BOM, with its own line endings). The header can be read without the
//! text, so listing recovery files stays cheap even for long documents.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const EXTENSION: &str = "recovery";
/// The header's `format`; files in any other format are ignored.
const FORMAT: u32 = 1;

/// What a recovery file is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryHeader {
    pub format: u32,
    /// The document the text belongs to.
    pub path: PathBuf,
    /// `hash::fnv1a` of the document's bytes as last read or saved, the
    /// version the unsaved changes were made to.
    pub base_hash: u64,
    /// When the text was written, in milliseconds since the Unix epoch.
    pub saved_at: u64,
}

impl RecoveryHeader {
    pub fn new(path: PathBuf, base_hash: u64, saved_at: u64) -> Self {
        RecoveryHeader {
            format: FORMAT,
            path,
            base_hash,
            saved_at,
        }
    }
}

/// A recovery file found in the folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryEntry {
    pub id: String,
    pub header: RecoveryHeader,
}

#[derive(Debug, thiserror::Error)]
pub enum RecoveryError {
    #[error("not a valid recovery id: {0:?}")]
    BadId(String),
    #[error("{} is not a Scrald recovery file", .0.display())]
    BadFile(PathBuf),
    // Rust note: `#[from]` lets `?` turn an `io::Error` into this variant
    // automatically, like an implicit converting constructor in C++.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Ids are lowercase letters, digits, and `-`: they become file names, and
/// some arrive from the frontend, so they must never contain a path.
pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn file_path(dir: &Path, id: &str) -> Result<PathBuf, RecoveryError> {
    if !is_valid_id(id) {
        return Err(RecoveryError::BadId(id.to_string()));
    }
    Ok(dir.join(format!("{id}.{EXTENSION}")))
}

/// Writes (or replaces) recovery file `id` in `dir`, atomically, creating
/// the folder if needed.
pub fn write(
    dir: &Path,
    id: &str,
    header: &RecoveryHeader,
    text: &str,
) -> Result<(), RecoveryError> {
    let path = file_path(dir, id)?;
    std::fs::create_dir_all(dir)?;
    // Compact JSON never contains a raw line break, so the header is
    // exactly the first line.
    let mut bytes = serde_json::to_vec(header).map_err(std::io::Error::from)?;
    bytes.push(b'\n');
    bytes.extend_from_slice(text.as_bytes());
    crate::save::write_atomic(&path, &bytes)?;
    Ok(())
}

/// Reads only the header of the file at `path`.
fn read_header(path: &Path) -> Result<RecoveryHeader, RecoveryError> {
    let mut line = String::new();
    BufReader::new(std::fs::File::open(path)?).read_line(&mut line)?;
    parse_header(path, &line)
}

fn parse_header(path: &Path, line: &str) -> Result<RecoveryHeader, RecoveryError> {
    match serde_json::from_str::<RecoveryHeader>(line.trim_end()) {
        Ok(header) if header.format == FORMAT => Ok(header),
        _ => Err(RecoveryError::BadFile(path.to_path_buf())),
    }
}

/// The header and text of recovery file `id`.
pub fn read(dir: &Path, id: &str) -> Result<(RecoveryHeader, String), RecoveryError> {
    let path = file_path(dir, id)?;
    let bytes = std::fs::read(&path)?;
    let bad = || RecoveryError::BadFile(path.clone());
    let newline = bytes.iter().position(|&b| b == b'\n').ok_or_else(bad)?;
    let header_line = std::str::from_utf8(&bytes[..newline]).map_err(|_| bad())?;
    let header = parse_header(&path, header_line)?;
    let text = String::from_utf8(bytes[newline + 1..].to_vec()).map_err(|_| bad())?;
    Ok((header, text))
}

/// Every readable recovery file in `dir`, newest first. Anything else in
/// the folder (temporary files, other formats) is skipped; a folder that
/// doesn't exist yet has no recovery files.
pub fn list(dir: &Path) -> Vec<RecoveryEntry> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    // Rust note: `flatten()` on an iterator of `Result`s keeps only the `Ok`
    // values, here skipping folder entries that couldn't be read.
    let mut found: Vec<RecoveryEntry> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()? != EXTENSION {
                return None;
            }
            let id = path.file_stem()?.to_str()?.to_string();
            if !is_valid_id(&id) {
                return None;
            }
            let header = read_header(&path).ok()?;
            Some(RecoveryEntry { id, header })
        })
        .collect();
    // Rust note: `Reverse` flips the ordering, so this sorts newest first.
    found.sort_by_key(|entry| std::cmp::Reverse(entry.header.saved_at));
    found
}

/// Deletes recovery file `id`. A file that's already gone is fine.
pub fn remove(dir: &Path, id: &str) -> Result<(), RecoveryError> {
    match std::fs::remove_file(file_path(dir, id)?) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("scrald-recovery-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn header(saved_at: u64) -> RecoveryHeader {
        RecoveryHeader::new(PathBuf::from("notes").join("skål.md"), 42, saved_at)
    }

    #[test]
    fn text_round_trips_exactly() {
        let dir = temp_dir("round-trip");
        // CRLF, multi-byte text, a leading line break, and no final one.
        let text = "\r\n---\r\ntitle: Skål\r\n---\r\n\r\nNorðr \u{1F6F6}";
        write(&dir, "a-1", &header(5), text).unwrap();
        let (read_header, read_text) = read(&dir, "a-1").unwrap();
        assert_eq!(read_header, header(5));
        assert_eq!(read_text, text);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn writing_again_replaces_the_text() {
        let dir = temp_dir("replace");
        write(&dir, "a", &header(1), "old").unwrap();
        write(&dir, "a", &header(2), "new").unwrap();
        assert_eq!(read(&dir, "a").unwrap(), (header(2), "new".to_string()));
        assert_eq!(list(&dir).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_is_newest_first_and_skips_other_files() {
        let dir = temp_dir("list");
        assert!(list(&dir).is_empty(), "a missing folder has no files");
        write(&dir, "older", &header(1), "a").unwrap();
        write(&dir, "newer", &header(2), "b").unwrap();
        std::fs::write(dir.join("notes.txt"), "x").unwrap();
        std::fs::write(dir.join("junk.recovery"), "not json\ntext").unwrap();
        std::fs::write(dir.join("Upper.recovery"), "{}\n").unwrap();
        let future = r#"{"format":2,"path":"a.md","baseHash":1,"savedAt":3}"#;
        std::fs::write(dir.join("future.recovery"), format!("{future}\ntext")).unwrap();
        let ids: Vec<String> = list(&dir).into_iter().map(|e| e.id).collect();
        assert_eq!(ids, ["newer", "older"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_deletes_and_tolerates_missing_files() {
        let dir = temp_dir("remove");
        write(&dir, "a", &header(1), "text").unwrap();
        remove(&dir, "a").unwrap();
        assert!(list(&dir).is_empty());
        remove(&dir, "a").unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ids_cannot_name_other_files() {
        let dir = temp_dir("ids");
        for id in ["", "../a", "a/b", r"a\b", "A", "a.b", &"a".repeat(65)] {
            assert!(!is_valid_id(id), "{id:?}");
            assert!(matches!(read(&dir, id), Err(RecoveryError::BadId(_))));
            assert!(matches!(remove(&dir, id), Err(RecoveryError::BadId(_))));
        }
        assert!(is_valid_id("19a2b3c4d5e-1f2c-3"));
    }
}
