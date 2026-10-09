//! Folder-level configuration: a `.scrald.toml` in a document's folder or any
//! folder above it (DESIGN.md §7.6). Lets a whole vault or book share a theme.

use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const FILE_NAME: &str = ".scrald.toml";

/// Settings from the nearest `.scrald.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FolderConfig {
    /// Theme id for documents under this folder.
    pub theme: Option<String>,
    /// Markdown flavor (`gfm`, `obsidian`, `pandoc`) for documents under it (M4).
    pub flavor: Option<String>,
}

/// A config file that was found, and what it said (or why it couldn't be read).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundConfig {
    pub path: PathBuf,
    pub config: Result<FolderConfig, String>,
}

/// The nearest `.scrald.toml` at or above `doc_path`'s folder. The nearest
/// file wins outright; settings aren't merged across several files.
pub fn find_folder_config(doc_path: &Path) -> Option<FoundConfig> {
    let start = doc_path.parent()?;
    // Rust note: `ancestors()` yields the path itself, then each parent in
    // turn up to the root, like repeatedly applying `os.path.dirname`.
    start.ancestors().find_map(|dir| {
        let path = dir.join(FILE_NAME);
        path.is_file().then(|| {
            let config = std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| {
                    toml::from_str::<FolderConfig>(&text).map_err(|e| e.message().to_string())
                });
            FoundConfig { path, config }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scrald-config-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn nearest_config_wins() {
        let root = temp_dir("nearest");
        let book = root.join("book");
        let chapters = book.join("chapters");
        std::fs::create_dir_all(&chapters).unwrap();
        std::fs::write(root.join(FILE_NAME), "theme = \"technical\"\n").unwrap();
        std::fs::write(
            book.join(FILE_NAME),
            "theme = \"sepia\"\nflavor = \"pandoc\"\n",
        )
        .unwrap();

        let found = find_folder_config(&chapters.join("one.md")).unwrap();
        assert_eq!(found.path, book.join(FILE_NAME));
        assert_eq!(
            found.config.unwrap(),
            FolderConfig {
                theme: Some("sepia".into()),
                flavor: Some("pandoc".into())
            }
        );
        let found = find_folder_config(&root.join("notes.md")).unwrap();
        assert_eq!(found.config.unwrap().theme.as_deref(), Some("technical"));
    }

    #[test]
    fn invalid_config_reports_why() {
        let dir = temp_dir("invalid");
        std::fs::write(dir.join(FILE_NAME), "theme = 3\n").unwrap();
        let found = find_folder_config(&dir.join("a.md")).unwrap();
        assert!(found.config.is_err());
    }

    #[test]
    fn no_config_anywhere() {
        // Nothing above the system temp folder has a .scrald.toml.
        let dir = temp_dir("none");
        assert_eq!(find_folder_config(&dir.join("a.md")), None);
    }
}
