//! Writing documents back to disk (DESIGN.md §9.3): the text is encoded with
//! the BOM it was read with, and written atomically, so a crash or a full disk
//! can never leave a half-written document behind.

use std::io::Write;
use std::path::{Path, PathBuf};

/// The bytes to write for a document's text: a UTF-8 BOM first if the file
/// had one. Line endings are whatever the text holds; edits keep them.
pub fn encode(text: &str, has_bom: bool) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() + 3);
    if has_bom {
        bytes.extend_from_slice(b"\xEF\xBB\xBF");
    }
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

/// Writes `bytes` to `path` atomically: into a temporary file in the same
/// folder (so the rename can't cross file systems), flushed to disk, then
/// renamed over the original. The original's permissions are kept. If any
/// step fails, the original is untouched and the temporary file removed.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temp = temp_path(path);
    let result = write_then_rename(path, &temp, bytes);
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

fn write_then_rename(path: &Path, temp: &Path, bytes: &[u8]) -> std::io::Result<()> {
    // Rust note: `create_new` fails if the file exists, so we never write
    // into someone else's file that happens to have the temporary name.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    if let Ok(metadata) = std::fs::metadata(path) {
        std::fs::set_permissions(temp, metadata.permissions())?;
    }
    // On Windows, `rename` replaces an existing file (MoveFileEx with
    // MOVEFILE_REPLACE_EXISTING); on Unix it's an atomic replace.
    std::fs::rename(temp, path)
}

/// `notes/.chapter.md.scrald-save-1234-0`: hidden, next to the document,
/// unique per process and attempt.
fn temp_path(path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    // Rust note: a `static` atomic counter is shared by all threads without a
    // lock; `fetch_add` returns the old value and increments in one step.
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document".to_string());
    path.with_file_name(format!(".{name}.scrald-save-{}-{n}", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scrald-save-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn encode_keeps_the_bom() {
        assert_eq!(encode("a\r\n", true), b"\xEF\xBB\xBFa\r\n");
        assert_eq!(encode("a\n", false), b"a\n");
    }

    #[test]
    fn writes_replace_the_file_and_leave_no_temp_files() {
        let dir = temp_dir("replace");
        let path = dir.join("doc.md");
        std::fs::write(&path, "old").unwrap();
        write_atomic(&path, b"new contents").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new contents");
        write_atomic(&path, b"again").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"again");
        let names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["doc.md"]);
    }

    #[test]
    fn a_failed_write_leaves_the_original() {
        let dir = temp_dir("fail");
        // The "document" is a folder, so the final rename must fail.
        let path = dir.join("folder.md");
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("inside.txt"), "x").unwrap();
        assert!(write_atomic(&path, b"data").is_err());
        assert!(path.is_dir());
        let leftovers = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(leftovers, 1, "temporary file was not cleaned up");
    }
}
