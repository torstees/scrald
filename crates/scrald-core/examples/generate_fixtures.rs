//! Writes the large generated fixtures to `target/fixtures/` for manual
//! performance testing in the app (DESIGN.md §4):
//!
//!     cargo run -p scrald-core --example generate_fixtures
//!
//! The files are generated, not committed; rerun this after a fresh clone.

// A command-line tool reports progress on stdout; the lint is for library code.
#![allow(clippy::print_stdout)]

use std::path::PathBuf;

use scrald_core::generate::generate_document;

fn main() -> std::io::Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("fixtures");
    std::fs::create_dir_all(&dir)?;
    for (label, words) in [("100k", 100_000), ("250k", 250_000), ("500k", 500_000)] {
        let path = dir.join(format!("large-{label}.md"));
        let text = generate_document(words, 42);
        std::fs::write(&path, &text)?;
        println!("wrote {} ({} KB)", path.display(), text.len() / 1024);
    }
    Ok(())
}
