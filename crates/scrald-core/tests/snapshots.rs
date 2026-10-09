//! Snapshot tests: every fixture in tests/fixtures/*.md is parsed and
//! rendered, and the result is compared with a reviewed snapshot in
//! tests/snapshots/. Review changes with `cargo insta review`.

use std::fmt::Write;
use std::path::PathBuf;

use scrald_core::{DocumentModel, parse_document};

/// A readable text form of a document model for snapshots.
fn describe(doc: &DocumentModel) -> String {
    let mut out = String::new();
    // Rust note: `writeln!` formats into any `fmt::Write`, like `fprintf`.
    // Writing to a String can't fail, so the Result is safe to ignore.
    let _ = writeln!(
        out,
        "bom: {}  line_ending: {:?}  words: {}  features: {:?}",
        doc.has_bom, doc.line_ending, doc.word_count, doc.features
    );
    let _ = writeln!(out, "flavor: {:?} ({:?})", doc.flavor, doc.flavor_source);
    if let Some(fm) = &doc.front_matter {
        let json = serde_json::to_string_pretty(fm).unwrap_or_default();
        let _ = writeln!(out, "front_matter: {json}");
    }
    for entry in &doc.toc {
        let _ = writeln!(
            out,
            "toc: {}{} #{} (block {})",
            "  ".repeat(usize::from(entry.level.saturating_sub(1))),
            entry.text,
            entry.slug,
            entry.block_id
        );
    }
    for s in &doc.sections {
        let _ = writeln!(
            out,
            "section {}: heading={:?} level={} blocks={}..{}",
            s.id,
            s.heading_block,
            s.level,
            s.first_block,
            s.first_block + s.block_count
        );
    }
    for note in &doc.inline_footnotes {
        let _ = writeln!(out, "inline footnote {}: {}", note.name, note.html.trim());
    }
    for b in &doc.blocks {
        let _ = writeln!(
            out,
            "\n--- block {} {:?} @{}..{} section={}",
            b.id, b.kind, b.source.start, b.source.end, b.section
        );
        out.push_str(b.html.trim_end());
        out.push('\n');
    }
    out
}

#[test]
fn fixtures() {
    // Rust note: `insta::glob!` runs the closure once per matching file and
    // names each snapshot after the file, e.g. `snapshots__fixtures@tables.md.snap`.
    insta::glob!("fixtures/*.md", |path| {
        let bytes = std::fs::read(path).expect("fixture should be readable");
        let doc =
            parse_document(PathBuf::from("fixture.md"), &bytes).expect("fixture should parse");
        insta::assert_snapshot!(describe(&doc));
    });
}
