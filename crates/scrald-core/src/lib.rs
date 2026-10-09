//! Scrald core: all Markdown logic that can be tested without a window.
//!
//! This crate must stay free of Tauri and GUI dependencies, and must build
//! and pass its tests on Windows, macOS, and Linux (see DESIGN.md §11.1).

pub mod document;
pub mod frontmatter;
pub mod generate;
pub mod render;
pub mod source;
pub mod toc;

use std::path::{Path, PathBuf};

// Rust note: `pub use` re-exports items so callers can write
// `scrald_core::DocumentModel` instead of `scrald_core::document::DocumentModel`.
pub use document::{
    Block, BlockKind, DocumentModel, FeatureFlags, Section, load_document, parse_document,
};
pub use frontmatter::FrontMatter;
pub use source::{LineEnding, SourceRange};
pub use toc::TocEntry;

/// Errors from reading a document.
// Rust note: `thiserror`'s derive writes the `Display` and `Error` impls; the
// `#[error("...")]` text is the message, with `{field}` filled in.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DocumentError {
    #[error("could not read {path}: {message}")]
    Io { path: PathBuf, message: String },
    #[error("file is not valid UTF-8 (first bad byte at offset {offset})")]
    NotUtf8 { offset: usize },
}

/// Title to show for a document before its front matter has been read:
/// the file name without its extension, e.g. `notes/The Long Winter.md`
/// becomes `The Long Winter`. Falls back to `"Untitled"` for paths with no
/// usable file name.
// Rust note: `&Path` is a borrowed path, like `const std::filesystem::path&`
// in C++. The caller keeps ownership; we return a new owned `String`.
pub fn title_from_path(path: &Path) -> String {
    // Rust note: `file_stem()` returns `Option<&OsStr>` (a value that may be
    // absent, like Python's `None`). `and_then` chains another Option-returning
    // step; `to_str()` fails only for names that aren't valid UTF-8.
    match path.file_stem().and_then(|stem| stem.to_str()) {
        Some(stem) if !stem.is_empty() => stem.to_string(),
        _ => "Untitled".to_string(),
    }
}

// Rust note: `#[cfg(test)]` compiles this module only for `cargo test`, so
// tests live next to the code they test without ending up in release builds.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_directory_and_extension() {
        // Build paths with `join` so the test is portable across platforms.
        let path = PathBuf::from("notes").join("The Long Winter.md");
        assert_eq!(title_from_path(&path), "The Long Winter");
    }

    #[test]
    fn keeps_multi_byte_characters() {
        assert_eq!(title_from_path(Path::new("skål.md")), "skål");
    }

    #[test]
    fn only_last_extension_is_removed() {
        assert_eq!(
            title_from_path(Path::new("chapter.draft.md")),
            "chapter.draft"
        );
    }

    #[test]
    fn falls_back_to_untitled() {
        assert_eq!(title_from_path(Path::new("")), "Untitled");
        assert_eq!(title_from_path(Path::new("..")), "Untitled");
    }
}
