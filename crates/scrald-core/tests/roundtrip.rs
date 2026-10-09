//! Round-trip tests (DESIGN.md §14): block ranges must tile the original
//! file exactly, so that edits splice into the right bytes and a no-op edit
//! reproduces the file byte for byte.

use std::path::{Path, PathBuf};

use scrald_core::generate::generate_document;
use scrald_core::{BlockKind, DocumentModel, parse_document};

fn fixture_paths() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("fixtures dir")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    paths.sort();
    paths
}

/// True if `gap` (text between blocks) holds nothing a reader would see:
/// blank lines, or link reference definitions, which comrak keeps out of the
/// AST.
fn is_invisible_gap(gap: &str) -> bool {
    gap.lines().all(|line| {
        let t = line.trim();
        t.is_empty() || (t.starts_with('[') && t.contains("]:") && !t.starts_with("[^"))
    })
}

/// Checks the tiling invariants and returns the reconstructed bytes.
fn check_tiling(name: &str, bytes: &[u8], doc: &DocumentModel) -> Vec<u8> {
    let body_start = doc
        .front_matter
        .as_ref()
        .map_or(usize::from(doc.has_bom) * 3, |fm| fm.range.end);
    let mut rebuilt = bytes[..body_start].to_vec();
    let mut pos = body_start;
    for block in &doc.blocks {
        let r = block.source;
        assert!(
            r.start >= pos,
            "{name}: block {} starts at {} before previous end {pos}",
            block.id,
            r.start
        );
        assert!(
            r.end <= bytes.len(),
            "{name}: block {} ends past the file",
            block.id
        );
        assert!(!r.is_empty(), "{name}: block {} is empty", block.id);
        let gap = std::str::from_utf8(&bytes[pos..r.start]).expect("gap is on char boundaries");
        assert!(
            is_invisible_gap(gap),
            "{name}: visible text between blocks: {gap:?}"
        );
        std::str::from_utf8(&bytes[r.as_range()]).expect("block is on char boundaries");
        rebuilt.extend_from_slice(&bytes[pos..r.end]);
        pos = r.end;
    }
    let tail = std::str::from_utf8(&bytes[pos..]).expect("tail is on char boundaries");
    assert!(
        is_invisible_gap(tail),
        "{name}: visible text after last block: {tail:?}"
    );
    rebuilt.extend_from_slice(&bytes[pos..]);
    rebuilt
}

/// Re-parsing a block's own source must give back one block of the same kind.
fn check_blocks_reparse(name: &str, bytes: &[u8], doc: &DocumentModel) {
    for block in &doc.blocks {
        // A footnote definition alone has no reference, so comrak drops it.
        if block.kind == BlockKind::FootnoteDefinition {
            continue;
        }
        let slice = &bytes[block.source.as_range()];
        let again = parse_document(PathBuf::from("block.md"), slice).expect("block parses");
        let kinds: Vec<&BlockKind> = again.blocks.iter().map(|b| &b.kind).collect();
        assert_eq!(
            kinds,
            [&block.kind],
            "{name}: block {} {:?}",
            block.id,
            String::from_utf8_lossy(slice)
        );
    }
}

#[test]
fn fixtures_tile_and_round_trip() {
    for path in fixture_paths() {
        let name = path.display().to_string();
        let bytes = std::fs::read(&path).expect("fixture readable");
        let doc = parse_document(path.clone(), &bytes).expect("fixture parses");
        let rebuilt = check_tiling(&name, &bytes, &doc);
        assert_eq!(rebuilt, bytes, "{name}: reconstruction differs");
        check_blocks_reparse(&name, &bytes, &doc);
    }
}

#[test]
fn generated_document_tiles() {
    let text = generate_document(30_000, 3);
    let doc = parse_document(PathBuf::from("gen.md"), text.as_bytes()).expect("parses");
    let rebuilt = check_tiling("generated", text.as_bytes(), &doc);
    assert_eq!(rebuilt, text.as_bytes());
    check_blocks_reparse("generated", text.as_bytes(), &doc);
}

#[test]
fn generated_document_tiles_with_crlf_and_bom() {
    let text = generate_document(10_000, 5).replace('\n', "\r\n");
    let mut bytes = b"\xEF\xBB\xBF".to_vec();
    bytes.extend_from_slice(text.as_bytes());
    let doc = parse_document(PathBuf::from("gen.md"), &bytes).expect("parses");
    let rebuilt = check_tiling("generated-crlf", &bytes, &doc);
    assert_eq!(rebuilt, bytes);
    check_blocks_reparse("generated-crlf", &bytes, &doc);
}
