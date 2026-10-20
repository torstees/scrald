//! New documents (DESIGN.md §11): checking the file name, the built-in and
//! sample templates, and the text a new document starts with.

use crate::flavor::Flavor;
use crate::yaml_edit::{self, Change, EditError};

/// File extensions a new document's name may already end with; any other
/// name gets `.md` added. The same list Scrald opens (dialogs, drag and drop).
const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown", "mdown", "mkd", "mkdn", "txt"];

/// Names Windows reserves for devices, with or without an extension.
const RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Why a file name can't be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NameError {
    #[error("the file needs a name")]
    Empty,
    #[error("a file name can't contain {0}")]
    Character(char),
    #[error("\"{0}\" is a name Windows reserves for a device")]
    Reserved(String),
    #[error("a file name can't end with a space or a dot")]
    TrailingDotOrSpace,
}

/// The file name to create for what the user typed: trimmed, with `.md`
/// added unless it already ends with a Markdown extension. Refuses names no
/// platform could store, and those Windows refuses (`<>:"/\|?*`, control
/// characters, reserved device names, a trailing dot or space), so a
/// document made on one system can be copied to another.
pub fn file_name(typed: &str) -> Result<String, NameError> {
    let name = typed.trim();
    if name.is_empty() || name == "." || name == ".." {
        return Err(NameError::Empty);
    }
    if let Some(c) = name.chars().find(|c| {
        matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control()
    }) {
        return Err(NameError::Character(c));
    }
    if name.ends_with('.') {
        return Err(NameError::TrailingDotOrSpace);
    }
    let has_extension = name
        .rsplit_once('.')
        .is_some_and(|(_, ext)| MARKDOWN_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()));
    let name = if has_extension {
        name.to_string()
    } else {
        format!("{name}.md")
    };
    // `CON.md` is as reserved as `CON`: Windows looks at the part before the
    // first dot.
    let stem = name.split('.').next().unwrap_or_default().trim_end();
    if RESERVED_NAMES.iter().any(|r| r.eq_ignore_ascii_case(stem)) {
        return Err(NameError::Reserved(stem.to_string()));
    }
    Ok(name)
}

/// The templates that come with Scrald. They add front matter only, never
/// placeholder text that would have to be deleted (DESIGN.md §11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinTemplate {
    /// No front matter besides the title.
    Plain,
    /// `tags` and `created`, as Obsidian notes usually have.
    Obsidian,
    /// `author` and `date`, which Pandoc reads.
    Pandoc,
}

impl BuiltinTemplate {
    pub const ALL: [BuiltinTemplate; 3] = [
        BuiltinTemplate::Plain,
        BuiltinTemplate::Obsidian,
        BuiltinTemplate::Pandoc,
    ];

    /// The id the app stores and the dialog sends: `builtin:obsidian`.
    pub fn id(self) -> &'static str {
        match self {
            BuiltinTemplate::Plain => "builtin:plain",
            BuiltinTemplate::Obsidian => "builtin:obsidian",
            BuiltinTemplate::Pandoc => "builtin:pandoc",
        }
    }

    pub fn from_id(id: &str) -> Option<BuiltinTemplate> {
        Self::ALL.into_iter().find(|t| t.id() == id)
    }

    /// The name in the dialog's menu.
    pub fn name(self) -> &'static str {
        match self {
            BuiltinTemplate::Plain => "Plain (GFM)",
            BuiltinTemplate::Obsidian => "Obsidian note",
            BuiltinTemplate::Pandoc => "Pandoc document",
        }
    }

    /// The flavor a document made from it is set to, as the user's choice
    /// for that document (stored by the app, not written in the file).
    pub fn flavor(self) -> Flavor {
        match self {
            BuiltinTemplate::Plain => Flavor::Gfm,
            BuiltinTemplate::Obsidian => Flavor::Obsidian,
            BuiltinTemplate::Pandoc => Flavor::Pandoc,
        }
    }

    /// The template's text. An empty `title:` comes first, so the title
    /// lands at the top of the front matter.
    pub fn text(self) -> &'static str {
        match self {
            BuiltinTemplate::Plain => "",
            BuiltinTemplate::Obsidian => "---\ntitle:\ntags: []\ncreated: {{date}}\n---\n",
            BuiltinTemplate::Pandoc => "---\ntitle:\nauthor:\ndate: {{date}}\n---\n",
        }
    }
}

/// File name of the sample template written to the templates folder.
pub const SAMPLE_TEMPLATE_NAME: &str = "Sample.md";

/// A user template to learn from: every feature, explained in YAML comments.
pub const SAMPLE_TEMPLATE: &str = "\
---
# A sample template from Scrald. Every .md file in this folder is offered
# as a template in New document (Ctrl+N), named after its file. Edit this
# one or copy it; Scrald won't write it again.
#
# `title` gets the title typed in the New document dialog, written the same
# way the properties panel writes it.
#
# Below the front matter, the word title in double braces becomes the title,
# and the word date in double braces becomes today's date (as 2026-10-19).
# The date works in the front matter too.
title:
tags: []
# To choose the flavor of documents made from this template, uncomment:
# scrald-flavor: obsidian
---

# {{title}}

Created {{date}}.

## Notes

## Next steps
";

/// The text a new document starts with: `template` with `title` set in its
/// front matter by the same minimal edit the properties panel makes
/// (DESIGN.md §8.3), then `{{title}}` replaced in the body and `{{date}}`
/// (`2026-10-19`) everywhere. Without a title, no front matter is added and
/// `{{title}}` becomes empty. Line endings follow the template.
pub fn starting_text(template: &str, title: Option<&str>, date: &str) -> Result<String, EditError> {
    let title = title.map(str::trim).filter(|t| !t.is_empty());
    let text = match title {
        Some(title) => yaml_edit::edit_document(template, "title", &Change::Text(title.into()))?,
        None => template.to_string(),
    };
    // `{{title}}` only below the front matter: inside it, raw text could
    // break the YAML, and `title` is already set there properly.
    let body_start = crate::frontmatter::split(&text).map_or(0, |split| split.block.end);
    let (front, body) = text.split_at(body_start);
    Ok(format!(
        "{}{}",
        front.replace("{{date}}", date),
        body.replace("{{title}}", title.unwrap_or_default())
            .replace("{{date}}", date)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_get_md_unless_they_have_a_markdown_extension() {
        assert_eq!(file_name("  Notes ").unwrap(), "Notes.md");
        assert_eq!(file_name("README.markdown").unwrap(), "README.markdown");
        assert_eq!(file_name("todo.TXT").unwrap(), "todo.TXT");
        assert_eq!(file_name("v1.2 plan").unwrap(), "v1.2 plan.md");
    }

    #[test]
    fn names_windows_refuses_are_refused() {
        assert_eq!(file_name(""), Err(NameError::Empty));
        assert_eq!(file_name(".."), Err(NameError::Empty));
        assert_eq!(file_name("a/b"), Err(NameError::Character('/')));
        assert_eq!(file_name("what?"), Err(NameError::Character('?')));
        assert_eq!(file_name("tab\there"), Err(NameError::Character('\t')));
        assert_eq!(file_name("con"), Err(NameError::Reserved("con".into())));
        assert_eq!(
            file_name("LPT1.md"),
            Err(NameError::Reserved("LPT1".into()))
        );
        assert_eq!(file_name("notes."), Err(NameError::TrailingDotOrSpace));
        assert!(file_name("console").is_ok());
    }

    #[test]
    fn plain_without_a_title_is_empty() {
        assert_eq!(
            starting_text(BuiltinTemplate::Plain.text(), None, "2026-10-19").unwrap(),
            ""
        );
    }

    #[test]
    fn plain_with_a_title_is_just_front_matter() {
        assert_eq!(
            starting_text(
                BuiltinTemplate::Plain.text(),
                Some("The Long Winter"),
                "2026-10-19"
            )
            .unwrap(),
            "---\ntitle: The Long Winter\n---\n"
        );
    }

    #[test]
    fn the_title_is_quoted_like_a_property_edit() {
        let text = starting_text(
            BuiltinTemplate::Obsidian.text(),
            Some("Notes: day 1"),
            "2026-10-19",
        )
        .unwrap();
        assert_eq!(
            text,
            "---\ntitle: \"Notes: day 1\"\ntags: []\ncreated: 2026-10-19\n---\n"
        );
        let pandoc =
            starting_text(BuiltinTemplate::Pandoc.text(), Some("Yes"), "2026-10-19").unwrap();
        assert_eq!(
            pandoc,
            "---\ntitle: \"Yes\"\nauthor:\ndate: 2026-10-19\n---\n"
        );
    }

    #[test]
    fn placeholders_fill_the_body_but_not_the_front_matter() {
        let template = "---\nsubtitle: {{title}}\nday: {{date}}\n---\n# {{title}}\n\n{{date}}\n";
        let text = starting_text(template, Some("A: B"), "2026-10-19").unwrap();
        assert_eq!(
            text,
            "---\nsubtitle: {{title}}\nday: 2026-10-19\ntitle: \"A: B\"\n---\n# A: B\n\n2026-10-19\n"
        );
    }

    #[test]
    fn the_sample_template_works() {
        let text = starting_text(SAMPLE_TEMPLATE, Some("Trip"), "2026-10-19").unwrap();
        assert!(text.contains("\ntitle: Trip\ntags: []\n"), "{text}");
        assert!(text.contains("\n# Trip\n"), "{text}");
        assert!(text.contains("\nCreated 2026-10-19.\n"), "{text}");
        assert!(!text.contains("{{"), "{text}");
        // The front matter still reads as YAML, comments and all.
        let doc = crate::document::parse_document("new.md".into(), text.as_bytes()).unwrap();
        let fm = doc.front_matter.unwrap();
        assert_eq!(fm.title.as_deref(), Some("Trip"));
        assert_eq!(fm.tags, Vec::<String>::new());
        assert_eq!(fm.error, None);
    }

    #[test]
    fn crlf_templates_stay_crlf() {
        let text =
            starting_text("---\r\ntitle:\r\n---\r\n# {{title}}\r\n", Some("X"), "d").unwrap();
        assert_eq!(text, "---\r\ntitle: X\r\n---\r\n# X\r\n");
    }
}
