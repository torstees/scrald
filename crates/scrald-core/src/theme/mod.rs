//! Themes (DESIGN.md §7): loading theme folders, the bundled themes, and
//! choosing which theme a document uses.

pub mod color;
pub mod css;
pub mod schema;

use std::path::{Path, PathBuf};

use serde::Serialize;

pub use schema::{Appearance, TextSizing, ThemeFile};

/// Themes compiled into Scrald. Each is a TOML file only (no asset files),
/// so they can be embedded with `include_str!`.
const BUNDLED: &[(&str, &str)] = &[
    (
        "scrald-light",
        include_str!("../../themes/scrald-light/theme.toml"),
    ),
    (
        "scrald-dark",
        include_str!("../../themes/scrald-dark/theme.toml"),
    ),
    ("sepia", include_str!("../../themes/sepia/theme.toml")),
    (
        "technical",
        include_str!("../../themes/technical/theme.toml"),
    ),
];

/// The default theme for light and dark system appearance.
pub const DEFAULT_LIGHT: &str = "scrald-light";
pub const DEFAULT_DARK: &str = "scrald-dark";

/// A loaded, valid theme.
#[derive(Debug, Clone)]
pub struct Theme {
    /// The theme's folder name, used in front matter and settings.
    pub id: String,
    pub origin: ThemeOrigin,
    pub file: ThemeFile,
    /// Contents of the theme's extra stylesheet, if it has one.
    pub extra_css: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeOrigin {
    Bundled,
    /// A folder in the user theme directory.
    User {
        dir: PathBuf,
    },
}

impl Theme {
    /// The theme as CSS, with theme-folder URLs under `asset_base`.
    pub fn css(&self, asset_base: &str) -> String {
        css::theme_css(&self.file, asset_base, self.extra_css.as_deref())
    }

    pub fn summary(&self) -> ThemeSummary {
        let colors = self.file.resolved_colors();
        ThemeSummary {
            id: self.id.clone(),
            name: self.file.meta.name.clone(),
            author: self.file.meta.author.clone(),
            appearance: Some(self.file.meta.appearance),
            bundled: self.origin == ThemeOrigin::Bundled,
            background: colors.background.to_string(),
            foreground: colors.foreground.to_string(),
            accent: colors.accent.to_string(),
            error: None,
        }
    }
}

/// What the theme switcher shows for each theme, including broken user
/// themes (with `error` set) so problems are visible instead of silent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeSummary {
    pub id: String,
    pub name: String,
    pub author: Option<String>,
    pub appearance: Option<Appearance>,
    pub bundled: bool,
    /// Swatch colors for previews.
    pub background: String,
    pub foreground: String,
    pub accent: String,
    /// Why the theme couldn't be loaded, if it couldn't.
    pub error: Option<String>,
}

/// All available themes: bundled ones, plus the user's, which take
/// precedence when an id is the same.
#[derive(Debug, Clone, Default)]
pub struct ThemeLibrary {
    themes: Vec<Theme>,
    broken: Vec<ThemeSummary>,
}

impl ThemeLibrary {
    /// Loads the bundled themes and every theme folder in `user_dir`.
    pub fn load(user_dir: Option<&Path>) -> Self {
        let mut library = ThemeLibrary::default();
        for (id, text) in BUNDLED {
            // Bundled themes are tested to parse (see tests below), so a
            // failure here would be a build mistake; skip rather than panic.
            if let Ok(file) = ThemeFile::parse(text) {
                library.themes.push(Theme {
                    id: (*id).to_string(),
                    origin: ThemeOrigin::Bundled,
                    file,
                    extra_css: None,
                });
            }
        }
        if let Some(dir) = user_dir {
            library.load_user_dir(dir);
        }
        library
    }

    fn load_user_dir(&mut self, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut folders: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.join("theme.toml").is_file())
            .collect();
        folders.sort();
        for folder in folders {
            let id = folder_id(&folder);
            match load_theme_dir(&folder) {
                Ok(theme) => {
                    // A user theme replaces a bundled theme with the same id.
                    self.themes.retain(|t| t.id != theme.id);
                    self.themes.push(theme);
                }
                Err(problems) => self.broken.push(ThemeSummary {
                    id: id.clone(),
                    name: id,
                    author: None,
                    appearance: None,
                    bundled: false,
                    background: String::new(),
                    foreground: String::new(),
                    accent: String::new(),
                    error: Some(problems.join("\n")),
                }),
            }
        }
    }

    pub fn get(&self, id: &str) -> Option<&Theme> {
        self.themes.iter().find(|t| t.id == id)
    }

    pub fn contains(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    /// Summaries of every theme, valid ones first, sorted by name.
    pub fn summaries(&self) -> Vec<ThemeSummary> {
        let mut list: Vec<ThemeSummary> = self.themes.iter().map(Theme::summary).collect();
        list.sort_by_key(|s| s.name.to_lowercase());
        list.extend(self.broken.iter().cloned());
        list
    }

    /// Copies a theme into `user_dir` as a new theme to customize, and
    /// returns the new theme's id. The copy is named "<name> (copy)".
    pub fn duplicate(&self, id: &str, user_dir: &Path) -> Result<String, String> {
        let theme = self.get(id).ok_or_else(|| format!("no theme named {id}"))?;
        let mut new_id = format!("{id}-copy");
        let mut n = 2;
        while user_dir.join(&new_id).exists() || self.contains(&new_id) {
            new_id = format!("{id}-copy-{n}");
            n += 1;
        }
        let target = user_dir.join(&new_id);
        let io = |e: std::io::Error| format!("could not create {}: {e}", target.display());
        match &theme.origin {
            ThemeOrigin::User { dir } => copy_dir(dir, &target).map_err(io)?,
            ThemeOrigin::Bundled => {
                std::fs::create_dir_all(&target).map_err(io)?;
                let text = BUNDLED
                    .iter()
                    .find(|(b, _)| *b == id)
                    .map(|(_, t)| *t)
                    .unwrap_or_default();
                std::fs::write(target.join("theme.toml"), text).map_err(io)?;
            }
        }
        let toml_path = target.join("theme.toml");
        let text = std::fs::read_to_string(&toml_path).map_err(io)?;
        let renamed = rename_in_toml(&text, &format!("{} (copy)", theme.file.meta.name));
        std::fs::write(&toml_path, renamed).map_err(io)?;
        Ok(new_id)
    }
}

/// Loads one theme folder: `theme.toml`, plus the stylesheet named by
/// `[css] file`, if any.
pub fn load_theme_dir(dir: &Path) -> Result<Theme, Vec<String>> {
    let toml_path = dir.join("theme.toml");
    let text = std::fs::read_to_string(&toml_path)
        .map_err(|e| vec![format!("could not read {}: {e}", toml_path.display())])?;
    let file = ThemeFile::parse(&text)?;
    let extra_css = match &file.css.file {
        Some(name) => Some(
            std::fs::read_to_string(dir.join(name))
                .map_err(|e| vec![format!("css.file: could not read {name}: {e}")])?,
        ),
        None => None,
    };
    Ok(Theme {
        id: folder_id(dir),
        origin: ThemeOrigin::User {
            dir: dir.to_path_buf(),
        },
        file,
        extra_css,
    })
}

fn folder_id(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Replaces the first `name = "..."` line (the one in `[meta]`) without
/// re-serializing the file, so comments and layout survive.
fn rename_in_toml(text: &str, new_name: &str) -> String {
    let escaped = new_name.replace('\\', "\\\\").replace('"', "\\\"");
    let mut done = false;
    let lines: Vec<String> = text
        .lines()
        .map(|line| {
            if !done && line.trim_start().starts_with("name") && line.contains('=') {
                done = true;
                let indent = &line[..line.len() - line.trim_start().len()];
                format!("{indent}name = \"{escaped}\"")
            } else {
                line.to_string()
            }
        })
        .collect();
    let mut out = lines.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// Where a document's theme choice came from (DESIGN.md §7.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemeSource {
    /// The user picked it for this document in Scrald.
    Document,
    /// `scrald-theme` in the document's front matter.
    FrontMatter,
    /// `theme` in a `.scrald.toml` in the document's folder or above.
    Folder,
    /// The global default (or the built-in default for the system appearance).
    Default,
}

/// The theme a document uses, in resolution order: the per-document
/// override, front matter, folder config, global default, then the
/// built-in default for the system appearance. Unknown ids are skipped, so
/// a typo in front matter falls through instead of breaking the document.
pub fn resolve_theme(
    library: &ThemeLibrary,
    document: Option<&str>,
    front_matter: Option<&str>,
    folder: Option<&str>,
    default: Option<&str>,
    system_dark: bool,
) -> (String, ThemeSource) {
    let candidates = [
        (document, ThemeSource::Document),
        (front_matter, ThemeSource::FrontMatter),
        (folder, ThemeSource::Folder),
        (default, ThemeSource::Default),
    ];
    for (id, source) in candidates {
        if let Some(id) = id.map(str::trim).filter(|id| library.contains(id)) {
            return (id.to_string(), source);
        }
    }
    let builtin = if system_dark {
        DEFAULT_DARK
    } else {
        DEFAULT_LIGHT
    };
    (builtin.to_string(), ThemeSource::Default)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scrald-themes-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_theme(dir: &Path, id: &str, toml: &str) {
        std::fs::create_dir_all(dir.join(id)).unwrap();
        std::fs::write(dir.join(id).join("theme.toml"), toml).unwrap();
    }

    const USER_THEME: &str = "# my theme\n[meta]\nname = \"Mine\"\nschema = 1\nappearance = \"light\"\n[css]\nfile = \"theme.css\"\n";

    #[test]
    fn bundled_themes_all_parse_and_have_distinct_names() {
        for (id, text) in BUNDLED {
            if let Err(problems) = ThemeFile::parse(text) {
                panic!("bundled theme {id} is invalid: {problems:?}");
            }
        }
        let library = ThemeLibrary::load(None);
        assert_eq!(library.summaries().len(), BUNDLED.len());
        assert!(library.contains(DEFAULT_LIGHT) && library.contains(DEFAULT_DARK));
        let mut names: Vec<String> = library.summaries().into_iter().map(|s| s.name).collect();
        names.dedup();
        assert_eq!(names.len(), BUNDLED.len());
    }

    #[test]
    fn user_themes_load_with_css_and_override_bundled() {
        let dir = temp_dir("user");
        write_theme(&dir, "mine", USER_THEME);
        std::fs::write(dir.join("mine").join("theme.css"), "p { color: red }").unwrap();
        write_theme(
            &dir,
            "sepia",
            &USER_THEME
                .replace("Mine", "My Sepia")
                .replace("[css]\nfile = \"theme.css\"\n", ""),
        );
        let library = ThemeLibrary::load(Some(&dir));
        let mine = library.get("mine").unwrap();
        assert_eq!(mine.extra_css.as_deref(), Some("p { color: red }"));
        assert_eq!(library.get("sepia").unwrap().file.meta.name, "My Sepia");
        assert_eq!(library.summaries().len(), BUNDLED.len() + 1);
    }

    #[test]
    fn broken_user_themes_are_listed_with_errors() {
        let dir = temp_dir("broken");
        write_theme(&dir, "oops", "[meta]\nname = \"Oops\"\n");
        let library = ThemeLibrary::load(Some(&dir));
        let broken = library
            .summaries()
            .into_iter()
            .find(|s| s.id == "oops")
            .unwrap();
        assert!(broken.error.unwrap().contains("schema"));
        assert!(!library.contains("oops"));
    }

    #[test]
    fn duplicate_bundled_theme() {
        let dir = temp_dir("dup");
        let library = ThemeLibrary::load(Some(&dir));
        let id = library.duplicate("sepia", &dir).unwrap();
        assert_eq!(id, "sepia-copy");
        let copy = load_theme_dir(&dir.join("sepia-copy")).unwrap();
        assert_eq!(copy.file.meta.name, "Sepia (copy)");
        // Comments in the original survive the rename.
        let text = std::fs::read_to_string(dir.join("sepia-copy").join("theme.toml")).unwrap();
        assert!(text.contains('#'));

        let library = ThemeLibrary::load(Some(&dir));
        assert_eq!(library.duplicate("sepia", &dir).unwrap(), "sepia-copy-2");
    }

    #[test]
    fn duplicate_user_theme_copies_its_files() {
        let dir = temp_dir("dupuser");
        write_theme(&dir, "mine", USER_THEME);
        std::fs::write(dir.join("mine").join("theme.css"), "p {}").unwrap();
        let library = ThemeLibrary::load(Some(&dir));
        let id = library.duplicate("mine", &dir).unwrap();
        assert!(dir.join(&id).join("theme.css").is_file());
        assert!(
            std::fs::read_to_string(dir.join(&id).join("theme.toml"))
                .unwrap()
                .starts_with("# my theme")
        );
    }

    #[test]
    fn resolution_order() {
        let library = ThemeLibrary::load(None);
        let r = |d, fm, f, def, dark| resolve_theme(&library, d, fm, f, def, dark);
        assert_eq!(
            r(Some("sepia"), Some("technical"), None, None, false),
            ("sepia".into(), ThemeSource::Document)
        );
        assert_eq!(
            r(None, Some("technical"), Some("sepia"), None, false),
            ("technical".into(), ThemeSource::FrontMatter)
        );
        assert_eq!(
            r(None, Some("no-such"), Some("sepia"), None, false),
            ("sepia".into(), ThemeSource::Folder)
        );
        assert_eq!(
            r(None, None, None, Some("technical"), false),
            ("technical".into(), ThemeSource::Default)
        );
        assert_eq!(
            r(None, None, None, None, true),
            (DEFAULT_DARK.into(), ThemeSource::Default)
        );
        assert_eq!(
            r(None, None, None, None, false),
            (DEFAULT_LIGHT.into(), ThemeSource::Default)
        );
    }

    #[test]
    fn rename_keeps_everything_else() {
        let text = "# c\n[meta]\nname = \"Old\"\nschema = 1\n[fonts]\nbody = { family = \"X\" }\n";
        assert_eq!(
            rename_in_toml(text, "New \"1\""),
            text.replace("\"Old\"", "\"New \\\"1\\\"\"")
        );
    }
}
