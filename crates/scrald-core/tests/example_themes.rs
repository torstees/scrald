//! The themes shipped in `assets/themes/` for users to copy into their own
//! themes folder. Like the bundled themes, they must load without errors,
//! meet the contrast check, and every file they name must be there.

use scrald_core::theme::contrast::contrast_problems;
use scrald_core::theme::load_theme_dir;
use std::path::PathBuf;

fn example_theme(id: &str) -> PathBuf {
    // Rust note: `env!` reads a variable at compile time; Cargo sets
    // CARGO_MANIFEST_DIR to this crate's folder, so the path works from any
    // working directory.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
        .join("themes")
        .join(id)
}

#[test]
fn skald_theme_loads_and_meets_contrast() {
    let dir = example_theme("skald");
    let theme = load_theme_dir(&dir).unwrap_or_else(|errors| panic!("skald theme: {errors:?}"));
    assert_eq!(theme.id, "skald");
    assert_eq!(theme.file.meta.name, "Skald");
    assert!(theme.extra_css.is_some(), "theme.css is loaded");

    let problems: Vec<String> = contrast_problems(&theme.file)
        .iter()
        .map(|p| p.to_string())
        .collect();
    assert!(problems.is_empty(), "contrast problems: {problems:#?}");
}

#[test]
fn skald_theme_files_exist() {
    let dir = example_theme("skald");
    let theme = load_theme_dir(&dir).unwrap_or_else(|errors| panic!("skald theme: {errors:?}"));

    let mut named: Vec<String> = Vec::new();
    // Rust note: iterating over `Option`s and calling `flatten()` skips the
    // `None`s, like a comprehension with `if x is not None` in Python.
    for font in [&theme.file.fonts.body, &theme.file.fonts.heading]
        .into_iter()
        .flatten()
    {
        named.extend(font.files.iter().map(|f| f.path.clone()));
    }
    named.extend(theme.file.elements.hr.image.clone());
    // The stylesheet's own font, loaded with `url()`.
    named.push("assets/fonts/UnifrakturMaguntia-Regular.woff2".to_string());

    assert!(
        named.len() >= 5,
        "fonts, divider, and the title font: {named:?}"
    );
    for path in named {
        assert!(dir.join(&path).is_file(), "missing {path}");
    }
}
