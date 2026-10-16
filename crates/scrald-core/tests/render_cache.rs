//! A parse that reuses a `RenderCache` must give exactly the same document
//! as a fresh parse, before and after edits (DESIGN.md §9.1).

use std::path::{Path, PathBuf};

use scrald_core::generate::generate_document;
use scrald_core::{
    DocumentModel, ParseOptions, RenderCache, parse_document_cached, parse_document_with,
};

fn fixture_paths() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir.join("fixtures"))
        .expect("fixtures dir")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    paths.sort();
    paths.push(dir.join("vault").join("Saga.md"));
    paths.push(dir.join("pandoc-layout").join("layout.md"));
    paths
}

/// Every field, compared through its debug form (the model has no `PartialEq`).
fn same(a: &DocumentModel, b: &DocumentModel) -> bool {
    format!("{a:?}") == format!("{b:?}")
}

/// The text with a word added to every third block, so some blocks change
/// and the rest stay the same.
fn edited(text: &str, doc: &DocumentModel) -> String {
    let bom = usize::from(doc.has_bom) * 3;
    let mut out = text.to_string();
    // From the end, so earlier offsets stay valid.
    for block in doc.blocks.iter().rev().step_by(3) {
        let end = block.source.end - bom;
        let at = out[..end].trim_end().len();
        out.insert_str(at, " edited");
    }
    out
}

#[test]
fn cached_parses_match_fresh_ones() {
    for path in fixture_paths() {
        let name = path.display().to_string();
        let bytes = std::fs::read(&path).expect("read fixture");
        let text = String::from_utf8(bytes.clone()).expect("fixture is UTF-8");
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text).to_string();
        let mut cache = RenderCache::default();

        for allow_remote_images in [false, true, false] {
            let options = ParseOptions {
                allow_remote_images,
                ..ParseOptions::default()
            };
            let fresh = parse_document_with(path.clone(), &bytes, &options).expect("parse");
            let cached =
                parse_document_cached(path.clone(), &bytes, &options, &mut cache).expect("parse");
            assert!(same(&fresh, &cached), "{name}: cached parse differs");
            assert!(!cache.is_empty() || fresh.blocks.is_empty(), "{name}");

            let changed = edited(&text, &fresh);
            let fresh_edit =
                parse_document_with(path.clone(), changed.as_bytes(), &options).expect("parse");
            let cached_edit =
                parse_document_cached(path.clone(), changed.as_bytes(), &options, &mut cache)
                    .expect("parse");
            assert!(
                same(&fresh_edit, &cached_edit),
                "{name}: cached parse after an edit differs"
            );
        }
    }
}

#[test]
fn cached_parse_of_a_large_document_matches() {
    let text = generate_document(20_000, 7);
    let path = PathBuf::from("generated.md");
    let options = ParseOptions::default();
    let mut cache = RenderCache::default();
    parse_document_cached(path.clone(), text.as_bytes(), &options, &mut cache).expect("parse");
    let fresh = parse_document_with(path.clone(), text.as_bytes(), &options).expect("parse");
    let changed = edited(&text, &fresh);
    let fresh_edit =
        parse_document_with(path.clone(), changed.as_bytes(), &options).expect("parse");
    let cached_edit =
        parse_document_cached(path, changed.as_bytes(), &options, &mut cache).expect("parse");
    assert!(same(&fresh_edit, &cached_edit));
}
