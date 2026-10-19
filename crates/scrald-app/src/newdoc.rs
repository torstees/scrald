//! New documents (DESIGN.md §11): the templates folder with its sample, the
//! templates offered in the dialog, and creating the file. Names and
//! starting text come from `scrald_core::newdoc`.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use scrald_core::Flavor;
use scrald_core::newdoc::{BuiltinTemplate, SAMPLE_TEMPLATE, SAMPLE_TEMPLATE_NAME};
use serde::Serialize;

use crate::state::StateStore;

/// Prefix of a user template's id; the rest is its file name.
const USER_PREFIX: &str = "user:";

/// A template in the dialog's menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TemplateChoice {
    /// `builtin:obsidian`, or `user:<file name>`.
    pub id: String,
    pub name: String,
}

/// The user's templates folder (`templates/` in the app config folder).
pub struct Templates {
    dir: Option<PathBuf>,
}

impl Templates {
    pub fn new(dir: Option<PathBuf>) -> Self {
        Templates { dir }
    }

    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    /// Creates the folder, and writes the sample template the first time
    /// only: an edited sample is never replaced, a deleted one stays gone.
    pub fn ensure_sample(&self, store: &StateStore) -> anyhow::Result<()> {
        let Some(dir) = &self.dir else { return Ok(()) };
        std::fs::create_dir_all(dir)
            .with_context(|| format!("could not create {}", dir.display()))?;
        if store.sample_template_written()? {
            return Ok(());
        }
        let path = dir.join(SAMPLE_TEMPLATE_NAME);
        if !path.exists() {
            std::fs::write(&path, SAMPLE_TEMPLATE)
                .with_context(|| format!("could not write {}", path.display()))?;
        }
        store.set_sample_template_written()
    }

    /// The built-in templates, then the user's (`*.md` in the folder) by name.
    pub fn list(&self) -> Vec<TemplateChoice> {
        let mut choices: Vec<TemplateChoice> = BuiltinTemplate::ALL
            .iter()
            .map(|t| TemplateChoice {
                id: t.id().to_string(),
                name: t.name().to_string(),
            })
            .collect();
        let mut user: Vec<TemplateChoice> = self
            .user_files()
            .into_iter()
            .map(|file| TemplateChoice {
                name: file.strip_suffix(".md").unwrap_or(&file).to_string(),
                id: format!("{USER_PREFIX}{file}"),
            })
            .collect();
        user.sort_by_key(|c| c.name.to_lowercase());
        choices.extend(user);
        choices
    }

    /// File names of the `.md` files in the templates folder.
    fn user_files(&self) -> Vec<String> {
        let Some(entries) = self
            .dir
            .as_ref()
            .and_then(|dir| std::fs::read_dir(dir).ok())
        else {
            return Vec::new();
        };
        entries
            .flatten()
            .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| name.to_ascii_lowercase().ends_with(".md"))
            .collect()
    }

    /// The text of template `id`, and the flavor a built-in one sets.
    /// A user template must be one of the files `list` offers, so an id
    /// can't reach outside the folder.
    pub fn text(&self, id: &str) -> anyhow::Result<(String, Option<Flavor>)> {
        if let Some(builtin) = BuiltinTemplate::from_id(id) {
            return Ok((builtin.text().to_string(), Some(builtin.flavor())));
        }
        let file = id
            .strip_prefix(USER_PREFIX)
            .filter(|file| self.user_files().iter().any(|f| f == file))
            .ok_or_else(|| anyhow::anyhow!("there's no template {id}"))?;
        let dir = self.dir.as_ref().context("there's no templates folder")?;
        let text = std::fs::read_to_string(dir.join(file))
            .with_context(|| format!("could not read the template {file}"))?;
        Ok((
            text.strip_prefix('\u{feff}').unwrap_or(&text).to_string(),
            None,
        ))
    }
}

/// Creates `name` in `folder` with `text`. Never replaces a file: one that
/// already exists (even one created a moment ago) is an error.
pub fn create_file(folder: &Path, name: &str, text: &str) -> anyhow::Result<PathBuf> {
    if !folder.is_absolute() || !folder.is_dir() {
        anyhow::bail!("{} isn't a folder", folder.display());
    }
    let path = folder.join(name);
    // Rust note: `create_new(true)` asks the OS to fail if the file exists,
    // in the same step that creates it, so there's no gap for a race
    // between checking and creating (like `O_EXCL` in C).
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            anyhow::bail!("there's already a file named {name} in that folder")
        }
        Err(e) => {
            return Err(e).with_context(|| format!("could not create {}", path.display()));
        }
    };
    file.write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
        .with_context(|| format!("could not write {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scrald-newdoc-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_sample_is_written_once() {
        let dir = temp_dir("sample");
        let store = StateStore::in_memory().unwrap();
        let templates = Templates::new(Some(dir.join("templates")));
        templates.ensure_sample(&store).unwrap();
        let sample = dir.join("templates").join(SAMPLE_TEMPLATE_NAME);
        assert_eq!(std::fs::read_to_string(&sample).unwrap(), SAMPLE_TEMPLATE);
        // Deleted by the user: it doesn't come back.
        std::fs::remove_file(&sample).unwrap();
        templates.ensure_sample(&store).unwrap();
        assert!(!sample.exists());
    }

    #[test]
    fn templates_list_builtins_then_user_files_by_name() {
        let dir = temp_dir("list");
        std::fs::write(dir.join("zettel.md"), "z").unwrap();
        std::fs::write(dir.join("Chapter.md"), "\u{feff}# {{title}}\n").unwrap();
        std::fs::write(dir.join("notes.txt"), "not a template").unwrap();
        let templates = Templates::new(Some(dir.clone()));
        let names: Vec<String> = templates.list().into_iter().map(|c| c.name).collect();
        assert_eq!(
            names,
            [
                "Plain (GFM)",
                "Obsidian note",
                "Pandoc document",
                "Chapter",
                "zettel"
            ]
        );
        let (text, flavor) = templates.text("user:Chapter.md").unwrap();
        assert_eq!((text.as_str(), flavor), ("# {{title}}\n", None));
        assert_eq!(
            templates.text("builtin:pandoc").unwrap().1,
            Some(Flavor::Pandoc)
        );
        // Only listed files: no way out of the folder.
        assert!(templates.text("user:../secret.md").is_err());
        assert!(templates.text("user:notes.txt").is_err());
    }

    #[test]
    fn creating_never_replaces_a_file() {
        let dir = temp_dir("create");
        let path = create_file(&dir, "New.md", "hello\n").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello\n");
        let again = create_file(&dir, "New.md", "other").unwrap_err();
        assert!(
            again.to_string().contains("already a file named New.md"),
            "{again}"
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello\n");
        assert!(create_file(&dir.join("missing"), "x.md", "").is_err());
        assert!(create_file(Path::new("relative"), "x.md", "").is_err());
    }
}
