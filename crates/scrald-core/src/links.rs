//! Classifying links clicked in a document (DESIGN.md §10): external URLs
//! open in the browser, Markdown files open in Scrald.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::assets::percent_decode;

/// File extensions treated as Markdown documents.
const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown", "mdown", "mkd", "mkdn"];

/// URL schemes that may be handed to the system's default handler.
const EXTERNAL_SCHEMES: &[&str] = &["http", "https", "mailto"];

/// Where a link points. Serialized as `{ "kind": "document", ... }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LinkTarget {
    /// A web or mail link, opened in the default browser or mail client.
    External { url: String },
    /// Another Markdown document, opened in Scrald. `fragment` is the part
    /// after `#`, a heading slug to scroll to.
    Document {
        path: PathBuf,
        fragment: Option<String>,
    },
    /// An anchor in the current document (`#slug`).
    Fragment { id: String },
    /// A local file that isn't Markdown. Scrald doesn't open these: a link
    /// to an executable must never run it.
    LocalFile { path: PathBuf },
    /// Anything else (`javascript:`, `ftp:`, unknown schemes).
    Unsupported { href: String },
}

/// Classifies `href` from a document at `doc_path`. Relative paths resolve
/// against the document's directory.
pub fn resolve_link(doc_path: &Path, href: &str) -> LinkTarget {
    let href = href.trim();
    if let Some(id) = href.strip_prefix('#') {
        return LinkTarget::Fragment {
            id: percent_decode(id),
        };
    }

    if let Some(scheme) = scheme_of(href) {
        let lower = scheme.to_ascii_lowercase();
        if EXTERNAL_SCHEMES.contains(&lower.as_str()) {
            return LinkTarget::External {
                url: href.to_string(),
            };
        }
        if lower == "file" {
            let rest = &href[scheme.len() + 1..];
            let rest = rest.strip_prefix("//").unwrap_or(rest);
            return local_target(&file_path_from_url(rest), fragment_of(href));
        }
        return LinkTarget::Unsupported {
            href: href.to_string(),
        };
    }

    let (path_part, fragment) = split_fragment(href);
    let path_part = path_part.split('?').next().unwrap_or_default();
    let relative = PathBuf::from(percent_decode(path_part));
    let path = if relative.is_absolute() {
        relative
    } else {
        doc_path.parent().unwrap_or(Path::new("")).join(relative)
    };
    local_target(&path, fragment)
}

fn local_target(path: &Path, fragment: Option<String>) -> LinkTarget {
    if is_markdown(path) {
        LinkTarget::Document {
            path: path.to_path_buf(),
            fragment,
        }
    } else {
        LinkTarget::LocalFile {
            path: path.to_path_buf(),
        }
    }
}

/// Whether a path has a Markdown extension (case-insensitive).
pub fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| MARKDOWN_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// The scheme of a URL (`https` in `https://...`). A single letter before
/// the colon is a Windows drive (`C:`), not a scheme.
fn scheme_of(href: &str) -> Option<&str> {
    let colon = href.find(':')?;
    let scheme = &href[..colon];
    let valid = scheme.len() > 1
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    valid.then_some(scheme)
}

/// Splits `path#fragment`; an empty fragment counts as none.
fn split_fragment(href: &str) -> (&str, Option<String>) {
    match href.split_once('#') {
        Some((path, frag)) if !frag.is_empty() => (path, Some(percent_decode(frag))),
        Some((path, _)) => (path, None),
        None => (href, None),
    }
}

fn fragment_of(href: &str) -> Option<String> {
    split_fragment(href).1
}

/// Path from a `file://` URL body (after `//`), dropping any fragment and
/// the leading `/` before a Windows drive letter.
fn file_path_from_url(rest: &str) -> PathBuf {
    let rest = rest.split(['#', '?']).next().unwrap_or_default();
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    let decoded = percent_decode(rest);
    let b = decoded.as_bytes();
    if b.len() >= 3 && b[0] == b'/' && b[1].is_ascii_alphabetic() && b[2] == b':' {
        PathBuf::from(&decoded[1..])
    } else {
        PathBuf::from(decoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> PathBuf {
        PathBuf::from("notes").join("index.md")
    }

    #[test]
    fn external_links() {
        assert_eq!(
            resolve_link(&doc(), "https://example.com/a?b#c"),
            LinkTarget::External {
                url: "https://example.com/a?b#c".into()
            }
        );
        assert!(matches!(
            resolve_link(&doc(), "HTTP://e.com"),
            LinkTarget::External { .. }
        ));
        assert!(matches!(
            resolve_link(&doc(), "mailto:a@b.c"),
            LinkTarget::External { .. }
        ));
    }

    #[test]
    fn markdown_documents_relative_to_the_current_one() {
        assert_eq!(
            resolve_link(&doc(), "chapter%202.md#the-fjord"),
            LinkTarget::Document {
                path: PathBuf::from("notes").join("chapter 2.md"),
                fragment: Some("the-fjord".into())
            }
        );
        assert_eq!(
            resolve_link(&doc(), "../README.MARKDOWN"),
            LinkTarget::Document {
                path: PathBuf::from("notes").join("../README.MARKDOWN"),
                fragment: None
            }
        );
    }

    #[test]
    fn fragments_in_this_document() {
        assert_eq!(
            resolve_link(&doc(), "#sk%C3%A5l"),
            LinkTarget::Fragment { id: "skål".into() }
        );
    }

    #[test]
    fn other_local_files_are_not_documents() {
        assert_eq!(
            resolve_link(&doc(), "setup.exe"),
            LinkTarget::LocalFile {
                path: PathBuf::from("notes").join("setup.exe")
            }
        );
        assert!(matches!(
            resolve_link(&doc(), "folder/"),
            LinkTarget::LocalFile { .. }
        ));
    }

    #[test]
    fn file_urls() {
        assert_eq!(
            resolve_link(&doc(), "file:///C:/docs/a%20b.md#x"),
            LinkTarget::Document {
                path: PathBuf::from("C:/docs/a b.md"),
                fragment: Some("x".into())
            }
        );
    }

    #[test]
    fn dangerous_or_unknown_schemes_are_unsupported() {
        for href in [
            "javascript:alert(1)",
            "ftp://x/y",
            "vbscript:x",
            "data:text/html,hi",
        ] {
            assert!(
                matches!(resolve_link(&doc(), href), LinkTarget::Unsupported { .. }),
                "{href}"
            );
        }
    }

    #[test]
    fn drive_letters_are_paths() {
        assert!(matches!(
            resolve_link(&doc(), "C:/x/y.md"),
            LinkTarget::Document { .. }
        ));
    }
}
