//! Scrald core: all Markdown logic that can be tested without a window.
//!
//! This crate must stay free of Tauri and GUI dependencies, and must build
//! and pass its tests on Windows, macOS, and Linux (see DESIGN.md §11.1).

pub mod assets;
pub mod cache;
pub mod config;
pub mod delta;
pub mod document;
pub mod edit;
pub mod flavor;
pub mod frontmatter;
pub mod generate;
pub mod grid;
pub mod hash;
pub mod highlight;
pub mod links;
pub mod newdoc;
pub mod obsidian;
pub mod opening;
pub mod pandoc;
pub mod recovery;
pub mod render;
pub mod save;
pub mod search;
pub mod source;
pub mod theme;
pub mod tidy;
pub mod toc;
pub mod transclude;
pub mod yaml_edit;

use std::path::{Path, PathBuf};

// Rust note: `pub use` re-exports items so callers can write
// `scrald_core::DocumentModel` instead of `scrald_core::document::DocumentModel`.
pub use assets::ImageAsset;
pub use cache::RenderCache;
pub use document::{
    Block, BlockKind, DocumentModel, DocumentPreview, FeatureFlags, ImplicitFigures, ParseOptions,
    Section, load_document, parse_document, parse_document_cached, parse_document_with,
};
pub use flavor::{Flavor, FlavorSource};
pub use frontmatter::FrontMatter;
pub use links::{LinkTarget, resolve_link};
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

/// A path as the window title shows it: with the home folder written `~`
/// when `home` is given and the path is inside it (`~/notes/a.md`).
pub fn display_path(path: &Path, home: Option<&Path>) -> String {
    // Rust note: `strip_prefix` on a `Path` compares whole components, so
    // `/Users/ann` is not a prefix of `/Users/anna/a.md`.
    match home.and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Some(rest) => Path::new("~").join(rest).display().to_string(),
        None => path.display().to_string(),
    }
}

// Rust note: `#[cfg(test)]` compiles this module only for `cargo test`, so
// tests live next to the code they test without ending up in release builds.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_path_shortens_the_home_folder() {
        // Built with `join`, so the separators are right on every platform.
        let home = Path::new("/Users").join("ann");
        let inside = home.join("notes").join("a.md");
        let shown = Path::new("~").join("notes").join("a.md");
        assert_eq!(
            display_path(&inside, Some(&home)),
            shown.display().to_string()
        );
        assert_eq!(display_path(&home, Some(&home)), "~");
        // A folder that only starts with the same letters isn't inside it.
        let other = Path::new("/Users").join("anna").join("a.md");
        assert_eq!(
            display_path(&other, Some(&home)),
            other.display().to_string()
        );
        assert_eq!(display_path(&inside, None), inside.display().to_string());
    }

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
