//! Tauri commands exposed to the frontend.
//!
//! Payload types here are mirrored in TypeScript in `ui/src/lib/types.ts`;
//! keep the two in sync.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::Context;
use scrald_core::theme::{Appearance, TextSizing, ThemeSource, ThemeSummary, resolve_theme};
use scrald_core::yaml_edit::Change;
use scrald_core::{DocumentModel, Flavor, LinkTarget, ParseOptions};
use serde::Serialize;
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;

use crate::protocol::AssetRegistry;
use crate::session::{DiskState, Sessions};
use crate::state::{
    DEFAULT_THEME, DocumentMemory, RecentDocument, ScrollAnchor, StateStore, TypographyDefaults,
};
use crate::themes::ThemeService;
use crate::watcher::DocumentWatchers;
use crate::window::WindowTracker;

/// An error returned to the frontend, where it arrives as a rejected promise
/// with this message.
// Rust note: Tauri needs command errors to be `Serialize`. `anyhow::Error`
// isn't, so this wrapper carries the message text across the boundary.
#[derive(Debug, Serialize)]
pub struct CommandError(String);

// Rust note: implementing `From<E>` lets `?` convert errors automatically:
// any error type that `anyhow::Error` accepts becomes a `CommandError`.
impl<E: Into<anyhow::Error>> From<E> for CommandError {
    fn from(error: E) -> Self {
        let error: anyhow::Error = error.into();
        // `{:#}` includes the chain of `.context()` messages.
        CommandError(format!("{error:#}"))
    }
}

/// A parsed document plus the token its images are served under.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedDocument {
    /// Prefix for `scrald-asset` URLs: image `id` is at `<assetToken>-<id>`.
    pub asset_token: u64,
    pub document: DocumentModel,
    /// What Scrald remembers about this document from earlier sessions.
    pub memory: DocumentMemory,
    /// The theme this document uses, and where that choice came from.
    pub theme: ResolvedTheme,
    /// Whether the window holds edits that aren't saved yet.
    pub dirty: bool,
}

/// A document's theme after the resolution order in DESIGN.md §7.6.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedTheme {
    pub id: String,
    pub source: ThemeSource,
}

/// Everything the frontend needs to apply a theme.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeStyle {
    pub id: String,
    pub name: String,
    pub appearance: Appearance,
    /// Complete CSS: `--sk-*` variables, font faces, element styles, theme.css.
    pub css: String,
    /// Whether the theme numbers headings (the TOC shows numbers too).
    pub numbering: bool,
    /// Layout numbers the frontend needs for text sizing and zoom.
    pub layout: ThemeLayout,
}

/// The theme's `[layout]` values used by fit mode and zoom (DESIGN.md §7.4).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeLayout {
    pub measure: u32,
    pub font_size: f32,
    pub min_font_size: f32,
    pub max_font_size: f32,
    /// The theme's suggested text sizing mode.
    pub text_sizing: TextSizing,
}

/// Reads and parses a Markdown file, registers its images for serving, and
/// sets the window title from it.
///
/// Parsing runs on a background thread so the window stays responsive.
// Rust note: an `async fn` command runs on Tauri's async runtime instead of
// the main thread. `spawn_blocking` then moves the CPU-bound parse onto a
// thread pool, and `.await` waits for it without blocking anything.
#[tauri::command]
pub async fn open_document(
    window: tauri::WebviewWindow,
    registry: tauri::State<'_, AssetRegistry>,
    watchers: tauri::State<'_, DocumentWatchers>,
    store: tauri::State<'_, StateStore>,
    tracker: tauri::State<'_, WindowTracker>,
    themes: tauri::State<'_, ThemeService>,
    sessions: tauri::State<'_, Sessions>,
    path: PathBuf,
) -> Result<OpenedDocument, CommandError> {
    let started = Instant::now();
    // Remembering is a convenience: a database problem never blocks reading.
    let memory = store.record_open(&path).unwrap_or_else(|error| {
        tracing::warn!(
            error = format!("{error:#}"),
            "could not read document state"
        );
        DocumentMemory::default()
    });
    tracker.set_document(window.label(), &path);

    // The window's session keeps the text (and later, unsaved edits).
    let bytes = sessions.open(window.label(), &path)?;
    let doc = parse_in_background(path.clone(), bytes, &memory).await?;
    tracing::info!(
        path = %path.display(),
        blocks = doc.blocks.len(),
        words = doc.word_count,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "opened document"
    );

    // Live reload is a convenience: if watching fails (say, on a network
    // share), the document still opens.
    if let Err(error) = watchers.watch(window.app_handle(), window.label(), &path) {
        tracing::warn!(error = format!("{error:#}"), "live reload unavailable");
    }

    finish_open(&window, &registry, &store, &themes, doc, memory, false)
}

/// Parses document bytes on a background thread so the window stays
/// responsive, with the options remembered for the document.
async fn parse_in_background(
    path: PathBuf,
    bytes: Vec<u8>,
    memory: &DocumentMemory,
) -> anyhow::Result<DocumentModel> {
    let options = ParseOptions {
        allow_remote_images: memory.remote_images,
        flavor: memory.flavor,
    };
    // Rust note: `move` makes the closure take ownership of `path`, `bytes`,
    // and `options`, so it can run on another thread after this function's
    // locals are gone. The double `?` unwraps two layers: the thread's
    // result, then the parse's.
    let doc = tauri::async_runtime::spawn_blocking(move || {
        scrald_core::parse_document_with(path, &bytes, &options)
    })
    .await
    .context("document parsing task failed")??;
    Ok(doc)
}

/// The steps shared by opening, re-parsing, and editing: window title,
/// theme, and the files the asset protocol may serve.
fn finish_open(
    window: &tauri::WebviewWindow,
    registry: &AssetRegistry,
    store: &StateStore,
    themes: &ThemeService,
    doc: DocumentModel,
    memory: DocumentMemory,
    dirty: bool,
) -> Result<OpenedDocument, CommandError> {
    set_window_title(window, &doc, dirty)?;
    let theme = resolve_document_theme(window, store, themes, &doc, &memory);
    let files = doc.images.iter().map(|image| image.path.clone()).collect();
    let asset_token = registry.register(window.label(), files);
    Ok(OpenedDocument {
        asset_token,
        document: doc,
        memory,
        theme,
        dirty,
    })
}

/// "Title — Scrald", with a leading "• " while there are unsaved changes.
fn set_window_title(
    window: &tauri::WebviewWindow,
    doc: &DocumentModel,
    dirty: bool,
) -> tauri::Result<()> {
    let title = doc
        .front_matter
        .as_ref()
        .and_then(|fm| fm.title.clone())
        .unwrap_or_else(|| scrald_core::title_from_path(&doc.path));
    let marker = if dirty { "\u{2022} " } else { "" };
    window.set_title(&format!("{marker}{title} \u{2014} Scrald"))
}

/// Parses the window's document again from its text in memory (keeping
/// unsaved edits), after its flavor or remote-image setting changed.
#[tauri::command]
pub async fn reparse_document(
    window: tauri::WebviewWindow,
    registry: tauri::State<'_, AssetRegistry>,
    store: tauri::State<'_, StateStore>,
    themes: tauri::State<'_, ThemeService>,
    sessions: tauri::State<'_, Sessions>,
) -> Result<OpenedDocument, CommandError> {
    let (path, bytes) = sessions
        .current(window.label())
        .context("no document is open in this window")?;
    let memory = store.record_open(&path).unwrap_or_default();
    let doc = parse_in_background(path, bytes, &memory).await?;
    let dirty = sessions.is_dirty(window.label());
    finish_open(&window, &registry, &store, &themes, doc, memory, dirty)
}

/// Changes one front matter key in the window's document (in memory; the
/// file changes on save) and returns the re-parsed document. Only that key's
/// text changes (DESIGN.md §8.3). Setting `scrald-theme` or `scrald-flavor`
/// also clears the document's stored theme or flavor choice, so the value
/// written to the document is the one in effect.
// Most arguments are app state that Tauri injects; the frontend passes two.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn edit_front_matter(
    window: tauri::WebviewWindow,
    registry: tauri::State<'_, AssetRegistry>,
    store: tauri::State<'_, StateStore>,
    themes: tauri::State<'_, ThemeService>,
    sessions: tauri::State<'_, Sessions>,
    key: String,
    change: Change,
) -> Result<OpenedDocument, CommandError> {
    let (path, bytes) = sessions.edit(window.label(), |text| {
        scrald_core::yaml_edit::edit_document(text, &key, &change)
    })?;
    match key.as_str() {
        "scrald-theme" => store.set_document_theme(&path, None)?,
        "scrald-flavor" => store.set_document_flavor(&path, None)?,
        _ => {}
    }
    let memory = store.record_open(&path).unwrap_or_default();
    let doc = parse_in_background(path, bytes, &memory).await?;
    let dirty = sessions.is_dirty(window.label());
    finish_open(&window, &registry, &store, &themes, doc, memory, dirty)
}

/// Saves the window's document to disk (atomically, keeping its BOM and
/// line endings) and clears the unsaved marker.
#[tauri::command]
pub fn save_document(
    window: tauri::WebviewWindow,
    sessions: tauri::State<'_, Sessions>,
) -> Result<(), CommandError> {
    sessions.save(window.label())?;
    if let Some((path, bytes)) = sessions.current(window.label()) {
        // Only the title changes; a parse failure here can't lose anything.
        if let Ok(doc) = scrald_core::parse_document(path, &bytes) {
            set_window_title(&window, &doc, false)?;
        }
    }
    Ok(())
}

/// How the file on disk compares with what this window last read or saved,
/// so a change notification caused by Scrald's own save can be ignored.
#[tauri::command]
pub fn check_disk(window: tauri::WebviewWindow, sessions: tauri::State<'_, Sessions>) -> DiskState {
    sessions.disk_state(window.label())
}

/// Asks for a folder for the `assets` property and returns it relative to
/// the document (`../images`), or absolute if it's on another drive. `None`
/// if the user cancelled. Runs in Rust so the frontend needs no file dialog
/// permission.
#[tauri::command]
pub async fn pick_assets_folder(
    app: tauri::AppHandle,
    document: PathBuf,
) -> Result<Option<String>, CommandError> {
    use tauri_plugin_dialog::DialogExt;
    let doc_dir = document.parent().map(PathBuf::from).unwrap_or_default();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_directory(&doc_dir)
            .set_title("Folder for this document's images")
            .blocking_pick_folder()
    })
    .await
    .context("folder picker failed")?;
    let Some(folder) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };
    let doc_dir = document.parent().map(PathBuf::from).unwrap_or_default();
    Ok(Some(
        scrald_core::assets::relative_path(&doc_dir, &folder)
            .unwrap_or_else(|| folder.display().to_string()),
    ))
}

/// Applies DESIGN.md §7.6: the per-document choice, then front matter, then
/// the nearest `.scrald.toml`, then the global default, then the built-in
/// default for the system's light or dark mode.
fn resolve_document_theme(
    window: &tauri::WebviewWindow,
    store: &StateStore,
    themes: &ThemeService,
    doc: &DocumentModel,
    memory: &DocumentMemory,
) -> ResolvedTheme {
    let front_matter = doc.front_matter.as_ref().and_then(|fm| fm.theme.clone());
    let folder = scrald_core::config::find_folder_config(&doc.path)
        .and_then(|found| match found.config {
            Ok(config) => config.theme,
            Err(error) => {
                tracing::warn!(path = %found.path.display(), %error, "ignoring invalid folder config");
                None
            }
        });
    let default = store.get_setting::<String>(DEFAULT_THEME).ok().flatten();
    let system_dark = matches!(window.theme(), Ok(tauri::Theme::Dark));
    let (id, source) = themes.with_library(|library| {
        resolve_theme(
            library,
            memory.theme.as_deref(),
            front_matter.as_deref(),
            folder.as_deref(),
            default.as_deref(),
            system_dark,
        )
    });
    ResolvedTheme { id, source }
}

/// Every theme, for the switcher: bundled, user, and broken ones (with errors).
#[tauri::command]
pub fn list_themes(themes: tauri::State<'_, ThemeService>) -> Vec<ThemeSummary> {
    themes.summaries()
}

/// The CSS and details for one theme.
#[tauri::command]
pub fn theme_style(
    themes: tauri::State<'_, ThemeService>,
    id: String,
) -> Result<ThemeStyle, CommandError> {
    let theme = themes
        .get(&id)
        .ok_or_else(|| anyhow::anyhow!("no theme named {id}"))?;
    Ok(ThemeStyle {
        css: theme.css(&crate::themes::asset_base(&theme.id)),
        name: theme.file.meta.name.clone(),
        appearance: theme.file.meta.appearance,
        numbering: theme.file.elements.headings.numbering,
        layout: ThemeLayout {
            measure: theme.file.layout.measure,
            font_size: theme.file.layout.font_size,
            min_font_size: theme.file.layout.min_font_size,
            max_font_size: theme.file.layout.max_font_size,
            text_sizing: theme.file.layout.text_sizing,
        },
        id: theme.id,
    })
}

/// Sets a document's text sizing mode and zoom; `null` clears either, so the
/// global default (or the theme's suggestion) applies again.
#[tauri::command]
pub fn set_document_typography(
    store: tauri::State<'_, StateStore>,
    path: PathBuf,
    text_sizing: Option<TextSizing>,
    zoom: Option<f64>,
) -> Result<(), CommandError> {
    Ok(store.set_document_typography(&path, text_sizing, zoom)?)
}

/// Sets the Markdown flavor for one document, or with `null` goes back to
/// front matter, folder config, or detection. The frontend reopens the
/// document afterwards to apply it.
#[tauri::command]
pub fn set_document_flavor(
    store: tauri::State<'_, StateStore>,
    path: PathBuf,
    flavor: Option<Flavor>,
) -> Result<(), CommandError> {
    Ok(store.set_document_flavor(&path, flavor)?)
}

/// Whether the properties panel starts open (a global preference).
#[tauri::command]
pub fn properties_open(store: tauri::State<'_, StateStore>) -> Result<bool, CommandError> {
    Ok(store.properties_open()?)
}

#[tauri::command]
pub fn set_properties_open(
    store: tauri::State<'_, StateStore>,
    open: bool,
) -> Result<(), CommandError> {
    Ok(store.set_properties_open(open)?)
}

/// Global typography settings ("Make this the default" and "fill window").
#[tauri::command]
pub fn typography_defaults(
    store: tauri::State<'_, StateStore>,
) -> Result<TypographyDefaults, CommandError> {
    Ok(store.typography_defaults()?)
}

#[tauri::command]
pub fn set_typography_defaults(
    store: tauri::State<'_, StateStore>,
    defaults: TypographyDefaults,
) -> Result<(), CommandError> {
    Ok(store.set_typography_defaults(defaults)?)
}

/// Sets the theme for one document, or with `null` returns it to its
/// default (front matter, folder, or global).
#[tauri::command]
pub fn set_document_theme(
    store: tauri::State<'_, StateStore>,
    path: PathBuf,
    id: Option<String>,
) -> Result<(), CommandError> {
    Ok(store.set_document_theme(&path, id.as_deref())?)
}

/// Sets the global default theme.
#[tauri::command]
pub fn set_default_theme(
    store: tauri::State<'_, StateStore>,
    id: String,
) -> Result<(), CommandError> {
    Ok(store.set_setting(DEFAULT_THEME, &id)?)
}

/// Copies a theme into the user theme folder to customize; returns the new id.
#[tauri::command]
pub fn duplicate_theme(
    themes: tauri::State<'_, ThemeService>,
    id: String,
) -> Result<String, CommandError> {
    Ok(themes.duplicate(&id)?)
}

/// Where user themes live, to show in the switcher.
#[tauri::command]
pub fn user_theme_folder(themes: tauri::State<'_, ThemeService>) -> Option<String> {
    themes.user_dir().map(|dir| dir.display().to_string())
}

/// Installed font family names, sorted (for font pickers, M8).
#[tauri::command]
pub async fn system_fonts() -> Result<Vec<String>, CommandError> {
    let families = tauri::async_runtime::spawn_blocking(crate::themes::system_font_families)
        .await
        .context("font scan failed")?;
    Ok(families)
}

/// Remembers where the reader is in a document.
#[tauri::command]
pub fn save_reading_position(
    store: tauri::State<'_, StateStore>,
    path: PathBuf,
    anchor: ScrollAnchor,
) -> Result<(), CommandError> {
    Ok(store.save_reading_position(&path, anchor)?)
}

/// Remembers whether remote images may load for a document. The frontend
/// reopens the document afterwards to apply it.
#[tauri::command]
pub fn set_remote_images(
    store: tauri::State<'_, StateStore>,
    path: PathBuf,
    allowed: bool,
) -> Result<(), CommandError> {
    Ok(store.set_remote_images(&path, allowed)?)
}

/// Recently opened documents, newest first (pinned first). For the start screen.
#[tauri::command]
pub fn recent_documents(
    store: tauri::State<'_, StateStore>,
    limit: usize,
) -> Result<Vec<RecentDocument>, CommandError> {
    Ok(store.recent_documents(limit)?)
}

/// Classifies a link clicked in the document at `document`.
#[tauri::command]
pub fn resolve_link(document: PathBuf, href: String) -> LinkTarget {
    scrald_core::resolve_link(&document, &href)
}

/// Opens a web or mail link in the system's default handler. Anything that
/// isn't `http`, `https`, or `mailto` is refused, so a document can't use
/// this to launch programs.
#[tauri::command]
pub fn open_external(app: tauri::AppHandle, url: String) -> Result<(), CommandError> {
    match scrald_core::resolve_link(std::path::Path::new(""), &url) {
        LinkTarget::External { url } => {
            app.opener().open_url(url, None::<&str>)?;
            Ok(())
        }
        _ => Err(anyhow::anyhow!("not a web or mail link: {url}").into()),
    }
}

/// Logs a timing measured in the frontend, so performance numbers end up in
/// the same log as the backend's (DESIGN.md §4 budget checks).
#[tauri::command]
pub fn report_timing(name: String, ms: f64) {
    tracing::info!(name, ms = (ms * 10.0).round() / 10.0, "frontend timing");
}

/// What the app was launched with. Built once at startup and stored as Tauri
/// managed state.
// Rust note: `#[derive(...)]` asks the compiler to generate trait impls:
// `Clone` gives `.clone()` (an explicit deep copy, like a C++ copy ctor), and
// serde's `Serialize` generates the JSON conversion Tauri needs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchInfo {
    /// Absolute path from the command line, if one was given.
    pub path: Option<PathBuf>,
    /// Display title for the window and page.
    pub title: String,
    /// Whether `path` existed when the app started.
    pub exists: bool,
}

impl LaunchInfo {
    /// Builds launch info from the first command-line argument, if any.
    pub fn from_arg(arg: Option<PathBuf>) -> Self {
        // Rust note: `map` transforms the value inside an Option only if it's
        // present, much like Python's `f(x) if x is not None else None`.
        let path = arg.map(|p| std::path::absolute(&p).unwrap_or(p));
        let title = match &path {
            Some(p) => scrald_core::title_from_path(p),
            None => "Scrald".to_string(),
        };
        let exists = path.as_ref().is_some_and(|p| p.is_file());
        LaunchInfo {
            path,
            title,
            exists,
        }
    }
}

/// Returns the path and title the app was launched with.
// Rust note: `State<'_, T>` borrows Tauri's managed state for the duration of
// this call; `'_` tells the compiler "infer the lifetime", so you rarely need
// to name it. We return a clone because the frontend needs an owned value.
#[tauri::command]
pub fn launch_info(state: tauri::State<'_, LaunchInfo>) -> LaunchInfo {
    state.inner().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_argument_means_no_document() {
        let info = LaunchInfo::from_arg(None);
        assert_eq!(info.path, None);
        assert_eq!(info.title, "Scrald");
        assert!(!info.exists);
    }

    #[test]
    fn relative_argument_becomes_absolute() {
        let info = LaunchInfo::from_arg(Some(PathBuf::from("does-not-exist.md")));
        let path = info.path.expect("path should be set");
        assert!(path.is_absolute());
        assert_eq!(info.title, "does-not-exist");
        assert!(!info.exists);
    }

    #[test]
    fn existing_file_is_detected() {
        // This crate's own manifest is a file that always exists.
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let info = LaunchInfo::from_arg(Some(manifest));
        assert!(info.exists);
        assert_eq!(info.title, "Cargo");
    }
}
