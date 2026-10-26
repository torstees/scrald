//! Tauri commands exposed to the frontend.
//!
//! Payload types here are mirrored in TypeScript in `ui/src/lib/types.ts`;
//! keep the two in sync.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Context;
use scrald_core::search::{SearchIndex, SearchOptions, SearchResults};
use scrald_core::theme::contrast::{ContrastProblem, contrast_problems};
use scrald_core::theme::{Appearance, TextSizing, ThemeSource, ThemeSummary, resolve_theme};
use scrald_core::yaml_edit::Change;
use scrald_core::{DocumentModel, DocumentPreview, Flavor, LinkTarget, ParseOptions};
use serde::Serialize;
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;

use crate::protocol::AssetRegistry;
use crate::session::{DiskState, SaveOutcome, Sessions, Step};
use crate::state::{
    CoverImage, DEFAULT_THEME, DocumentMemory, FrontSettings, PropertiesPlacement, RecentDocument,
    ScrollAnchor, StateStore, TypographyDefaults,
};
use crate::themes::ThemeService;
use crate::vaults::VaultCache;
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
///
/// After the first model for a document, blocks the window already shows
/// are sent without their HTML (DESIGN.md §9.1): `htmlFrom[i]` names the
/// block in model `base` to copy block `i`'s HTML from.
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
    /// Unsaved changes to this document left behind when Scrald stopped
    /// before saving them (only when opening; the newest, if several).
    pub recovered: Option<RecoveredChanges>,
    /// This model's number.
    pub model: u64,
    /// The model the window must be showing to fill in left-out HTML;
    /// `None` when every block has its HTML.
    pub base: Option<u64>,
    /// For each block, the block in `base` whose HTML it has, or `None` if
    /// its HTML is included. Empty when `base` is `None`.
    pub html_from: Vec<Option<u32>>,
}

/// Unsaved changes found in a recovery file (DESIGN.md §9.3).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveredChanges {
    pub id: String,
    pub path: PathBuf,
    /// The title in the recovered text (front matter or file name).
    pub title: String,
    /// When the changes were last written, in milliseconds since the epoch.
    pub saved_at: u64,
    /// The file changed after these changes were made: restoring and saving
    /// would replace that newer version.
    pub file_changed: bool,
    /// The file can't be read any more, so the changes can't be restored
    /// into it (they can still be copied).
    pub file_missing: bool,
}

/// Describes a recovery file for the frontend. `None` if it can't be read.
fn describe_recovery(
    sessions: &Sessions,
    entry: &scrald_core::recovery::RecoveryEntry,
) -> Option<RecoveredChanges> {
    let (header, text) = sessions.recovery_text(&entry.id).ok()?;
    let disk = std::fs::read(&header.path).ok();
    Some(RecoveredChanges {
        id: entry.id.clone(),
        title: title_from_text(&header.path, &text),
        saved_at: header.saved_at,
        file_changed: disk
            .as_deref()
            .is_some_and(|bytes| scrald_core::hash::fnv1a(bytes) != header.base_hash),
        file_missing: disk.is_none(),
        path: header.path,
    })
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
    /// Whether the theme shows a document's cover, and its title when the
    /// text doesn't start with it (#189); the user's settings can override.
    pub front_cover: bool,
    pub front_title: bool,
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
// Most arguments are app state that Tauri injects; the frontend passes one.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn open_document(
    window: tauri::WebviewWindow,
    registry: tauri::State<'_, AssetRegistry>,
    watchers: tauri::State<'_, DocumentWatchers>,
    store: tauri::State<'_, StateStore>,
    tracker: tauri::State<'_, WindowTracker>,
    themes: tauri::State<'_, ThemeService>,
    sessions: tauri::State<'_, Sessions>,
    vaults: tauri::State<'_, VaultCache>,
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
    let doc = parse_in_background(
        &sessions,
        window.label(),
        path.clone(),
        bytes,
        &memory,
        &vaults,
    )
    .await?;
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

    let opened = finish_open(&window, &registry, &store, &themes, doc, memory, false)?;
    // For the start screen's recent list; never worth failing the open over.
    // The vault index is the one the parse just used, so this rarely scans.
    let preview = DocumentPreview::new(
        &path,
        opened.document.front_matter.as_ref(),
        vaults.for_document(&path),
    );
    if let Err(error) = store
        .set_document_preview(&path, &preview)
        .and_then(|()| store.set_document_display(&path, None, Some(&opened.theme.id)))
    {
        tracing::warn!(
            error = format!("{error:#}"),
            "could not store the document's title"
        );
    }
    let mut opened = send_model(&sessions, window.label(), opened, true);
    opened.recovered = sessions
        .orphaned_recoveries(Some(&path))
        .first()
        .and_then(|entry| describe_recovery(&sessions, entry));
    Ok(opened)
}

/// Parses document bytes on a background thread so the window stays
/// responsive, with the options remembered for the document. Rendering
/// results from the window's last parse are reused (`RenderCache`), so after
/// an edit only changed blocks are sanitized and highlighted again.
async fn parse_in_background(
    sessions: &Sessions,
    window: &str,
    path: PathBuf,
    bytes: Vec<u8>,
    memory: &DocumentMemory,
    vaults: &VaultCache,
) -> anyhow::Result<DocumentModel> {
    let allow_remote_images = memory.remote_images;
    let flavor = memory.flavor;
    let vaults = vaults.clone();
    let mut cache = sessions.take_render_cache(window);
    // Rust note: `move` makes the closure take ownership of `path`, `bytes`,
    // the cache, and the vault handle, so it can run on another thread after
    // this function's locals are gone. It hands the cache back with the
    // result, as a tuple.
    let (doc, cache) = tauri::async_runtime::spawn_blocking(move || {
        let options = ParseOptions {
            allow_remote_images,
            flavor,
            vault: vaults.for_document(&path),
        };
        let doc = scrald_core::parse_document_cached(path, &bytes, &options, &mut cache);
        (doc, cache)
    })
    .await
    .context("document parsing task failed")?;
    sessions.put_render_cache(window, cache);
    Ok(doc?)
}

/// Leaves out the HTML the window already shows (unless `full`) and
/// records what it will show. Opening a document always sends it whole.
fn send_model(
    sessions: &Sessions,
    window: &str,
    mut opened: OpenedDocument,
    full: bool,
) -> OpenedDocument {
    // Before `send_blocks` leaves the window's HTML out of the blocks.
    sessions.set_search(window, SearchIndex::new(&opened.document));
    let sent = sessions.send_blocks(window, &mut opened.document.blocks, full);
    opened.model = sent.model;
    opened.base = sent.base;
    opened.html_from = sent.html_from;
    opened
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
        recovered: None,
        model: 0,
        base: None,
        html_from: Vec::new(),
    })
}

/// "Title — Scrald", with a leading "• " while there are unsaved changes.
fn set_window_title(
    window: &tauri::WebviewWindow,
    doc: &DocumentModel,
    dirty: bool,
) -> tauri::Result<()> {
    set_title_text(window, &document_title(doc), dirty)
}

/// The front matter's title, or one made from the file name.
fn document_title(doc: &DocumentModel) -> String {
    doc.front_matter
        .as_ref()
        .and_then(|fm| fm.title.clone())
        .unwrap_or_else(|| scrald_core::title_from_path(&doc.path))
}

fn set_title_text(window: &tauri::WebviewWindow, title: &str, dirty: bool) -> tauri::Result<()> {
    let marker = if dirty { "\u{2022} " } else { "" };
    window.set_title(&format!("{marker}{title} \u{2014} Scrald"))
}

/// The window title for document text, read from its front matter alone
/// (raw mode updates the title without parsing the whole document).
fn title_from_text(path: &std::path::Path, text: &str) -> String {
    scrald_core::frontmatter::split(text)
        .and_then(|split| {
            let yaml = &text[split.yaml.clone()];
            scrald_core::frontmatter::parse(yaml, scrald_core::SourceRange::default(), 0).title
        })
        .unwrap_or_else(|| scrald_core::title_from_path(path))
}

/// The whole document text (without BOM) and its version, for raw mode
/// (DESIGN.md §9.2).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawText {
    pub text: String,
    pub version: u64,
}

#[tauri::command]
pub fn document_text(
    window: tauri::WebviewWindow,
    sessions: tauri::State<'_, Sessions>,
) -> Result<RawText, CommandError> {
    let (text, version) = sessions
        .text(window.label())
        .context("no document is open in this window")?;
    Ok(RawText { text, version })
}

/// After a raw-mode text replacement: the text's new version, and whether
/// the document has unsaved changes.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSync {
    pub version: u64,
    pub dirty: bool,
}

/// Replaces the whole text from raw mode (in memory; saved with Ctrl+S).
/// Refused if `version` is out of date. Doesn't re-parse: the reader
/// re-parses when raw mode closes.
#[tauri::command]
pub fn set_document_text(
    window: tauri::WebviewWindow,
    sessions: tauri::State<'_, Sessions>,
    version: u64,
    text: String,
) -> Result<RawSync, CommandError> {
    let title = sessions
        .current(window.label())
        .map(|(path, _)| title_from_text(&path, &text));
    let (version, dirty) = sessions.replace_text(window.label(), version, text)?;
    if let Some(title) = title {
        set_title_text(&window, &title, dirty)?;
    }
    Ok(RawSync { version, dirty })
}

/// Matches of a search in the model numbered `model` (DESIGN.md §10).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchReply {
    pub model: u64,
    // Rust note: `flatten` writes the inner struct's fields at this level
    // in the JSON, as if they were declared here.
    #[serde(flatten)]
    pub results: SearchResults,
}

/// Searches the text of the document the window shows. `None` while
/// another search of this window is running, or with no document open:
/// the frontend runs one search at a time, so it then asks again.
#[tauri::command]
pub async fn search_document(
    window: tauri::WebviewWindow,
    sessions: tauri::State<'_, Sessions>,
    query: String,
    match_case: bool,
    whole_word: bool,
) -> Result<Option<SearchReply>, CommandError> {
    let Some((model, mut index)) = sessions.take_search(window.label()) else {
        return Ok(None);
    };
    let options = SearchOptions {
        match_case,
        whole_word,
    };
    // The first search reads every block's text out of its HTML (about
    // 70 ms for 500K words), so it runs off the main thread.
    let (results, index) = tauri::async_runtime::spawn_blocking(move || {
        let results = index.search(&query, options);
        (results, index)
    })
    .await
    .context("search task failed")?;
    sessions.put_search(window.label(), model, index);
    Ok(Some(SearchReply { model, results }))
}

/// Parses the window's document again from its text in memory (keeping
/// unsaved edits), after its flavor or remote-image setting changed. With
/// `full`, every block's HTML is sent, for a window that can't fill in
/// left-out HTML (it isn't showing the `base` model).
// Most arguments are app state that Tauri injects; the frontend passes one.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn reparse_document(
    window: tauri::WebviewWindow,
    registry: tauri::State<'_, AssetRegistry>,
    store: tauri::State<'_, StateStore>,
    themes: tauri::State<'_, ThemeService>,
    sessions: tauri::State<'_, Sessions>,
    vaults: tauri::State<'_, VaultCache>,
    full: bool,
) -> Result<OpenedDocument, CommandError> {
    let (path, bytes) = sessions
        .current(window.label())
        .context("no document is open in this window")?;
    let memory = store.memory(&path).unwrap_or_default();
    let doc = parse_in_background(&sessions, window.label(), path, bytes, &memory, &vaults).await?;
    let dirty = sessions.is_dirty(window.label());
    let opened = finish_open(&window, &registry, &store, &themes, doc, memory, dirty)?;
    Ok(send_model(&sessions, window.label(), opened, full))
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
    vaults: tauri::State<'_, VaultCache>,
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
    let memory = store.memory(&path).unwrap_or_default();
    let doc = parse_in_background(&sessions, window.label(), path, bytes, &memory, &vaults).await?;
    let dirty = sessions.is_dirty(window.label());
    let opened = finish_open(&window, &registry, &store, &themes, doc, memory, dirty)?;
    Ok(send_model(&sessions, window.label(), opened, false))
}

/// The Markdown source of a block, for the block editor (DESIGN.md §9.1).
/// `start` and `end` are the block's `source` range.
#[tauri::command]
pub fn block_source(
    window: tauri::WebviewWindow,
    sessions: tauri::State<'_, Sessions>,
    start: usize,
    end: usize,
) -> Result<String, CommandError> {
    Ok(sessions
        .source(window.label(), start, end)
        .context("the block's range is outside the document")?)
}

/// Replaces a block's source (in memory; the file changes on save) and
/// returns the re-parsed document. `original` is the source the editor
/// started from: if the document changed meanwhile, nothing is changed.
// Most arguments are app state that Tauri injects; the frontend passes four.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn edit_block(
    window: tauri::WebviewWindow,
    registry: tauri::State<'_, AssetRegistry>,
    store: tauri::State<'_, StateStore>,
    themes: tauri::State<'_, ThemeService>,
    sessions: tauri::State<'_, Sessions>,
    vaults: tauri::State<'_, VaultCache>,
    start: usize,
    end: usize,
    original: String,
    text: String,
) -> Result<OpenedDocument, CommandError> {
    let bom_len = sessions.bom_len(window.label());
    let range = scrald_core::SourceRange::new(start, end);
    let (path, bytes) = sessions.edit(window.label(), |current| {
        scrald_core::edit::splice_block(current, range, bom_len, &original, &text)
    })?;
    let memory = store.memory(&path).unwrap_or_default();
    let doc = parse_in_background(&sessions, window.label(), path, bytes, &memory, &vaults).await?;
    let dirty = sessions.is_dirty(window.label());
    let opened = finish_open(&window, &registry, &store, &themes, doc, memory, dirty)?;
    Ok(send_model(&sessions, window.label(), opened, false))
}

/// Undoes the last edit (block, properties) in the window's document, or
/// with `redo` redoes the last undone one. `null` if there's nothing to do.
#[tauri::command]
pub async fn step_history(
    window: tauri::WebviewWindow,
    registry: tauri::State<'_, AssetRegistry>,
    store: tauri::State<'_, StateStore>,
    themes: tauri::State<'_, ThemeService>,
    sessions: tauri::State<'_, Sessions>,
    vaults: tauri::State<'_, VaultCache>,
    redo: bool,
) -> Result<Option<OpenedDocument>, CommandError> {
    let step = if redo { Step::Redo } else { Step::Undo };
    let Some((path, bytes)) = sessions.step(window.label(), step)? else {
        return Ok(None);
    };
    let memory = store.memory(&path).unwrap_or_default();
    let doc = parse_in_background(&sessions, window.label(), path, bytes, &memory, &vaults).await?;
    let dirty = sessions.is_dirty(window.label());
    let opened = finish_open(&window, &registry, &store, &themes, doc, memory, dirty)?;
    Ok(Some(send_model(&sessions, window.label(), opened, false)))
}

/// Saves the window's document to disk (atomically, keeping its BOM and
/// line endings) and clears the unsaved marker. Returns `conflict` without
/// writing if the file changed on disk and the user hasn't chosen what to
/// keep (DESIGN.md §9.3).
///
/// Async so that finding a cover image in an Obsidian vault (which may scan
/// it) doesn't run on the main thread.
#[tauri::command]
pub async fn save_document(
    window: tauri::WebviewWindow,
    sessions: tauri::State<'_, Sessions>,
    store: tauri::State<'_, StateStore>,
    vaults: tauri::State<'_, VaultCache>,
) -> Result<SaveOutcome, CommandError> {
    let outcome = sessions.save(window.label())?;
    // Rust note: a "let chain" mixes a plain condition with `let` patterns;
    // the body runs only if all of them hold.
    if outcome == SaveOutcome::Saved
        && let Some((path, _)) = sessions.current(window.label())
        && let Some((text, _)) = sessions.text(window.label())
    {
        // Only the marker changes; the title comes from the front matter.
        let preview = DocumentPreview::from_text(&path, &text, vaults.for_document(&path));
        set_title_text(&window, &preview.title, false)?;
        if let Err(error) = store.set_document_preview(&path, &preview) {
            tracing::warn!(
                error = format!("{error:#}"),
                "could not store the document's title"
            );
        }
    }
    Ok(outcome)
}

/// The text of the changed disk version found by `check_disk` or a refused
/// save, for comparing with the window's text.
#[tauri::command]
pub fn disk_text(
    window: tauri::WebviewWindow,
    sessions: tauri::State<'_, Sessions>,
) -> Result<String, CommandError> {
    Ok(sessions.disk_text(window.label())?)
}

/// The text of the file as it is on disk now, for comparing recovered
/// changes with it.
#[tauri::command]
pub fn file_text(
    window: tauri::WebviewWindow,
    sessions: tauri::State<'_, Sessions>,
) -> Result<String, CommandError> {
    Ok(sessions.file_text(window.label())?)
}

/// "Keep mine": the next save may replace the changed disk version.
#[tauri::command]
pub fn keep_local_version(window: tauri::WebviewWindow, sessions: tauri::State<'_, Sessions>) {
    sessions.keep_local(window.label());
}

/// "Load disk version": replaces the window's text with the changed disk
/// version (undoable) and returns the re-parsed document.
#[tauri::command]
pub async fn load_disk_version(
    window: tauri::WebviewWindow,
    registry: tauri::State<'_, AssetRegistry>,
    store: tauri::State<'_, StateStore>,
    themes: tauri::State<'_, ThemeService>,
    sessions: tauri::State<'_, Sessions>,
    vaults: tauri::State<'_, VaultCache>,
) -> Result<OpenedDocument, CommandError> {
    let (path, bytes) = sessions.load_disk_version(window.label())?;
    let memory = store.memory(&path).unwrap_or_default();
    let doc = parse_in_background(&sessions, window.label(), path, bytes, &memory, &vaults).await?;
    let dirty = sessions.is_dirty(window.label());
    let opened = finish_open(&window, &registry, &store, &themes, doc, memory, dirty)?;
    Ok(send_model(&sessions, window.label(), opened, false))
}

/// Every set of unsaved changes left behind when Scrald stopped before they
/// were saved or discarded, newest first (offered when Scrald starts without
/// a document).
#[tauri::command]
pub fn recovered_changes(sessions: tauri::State<'_, Sessions>) -> Vec<RecoveredChanges> {
    sessions
        .orphaned_recoveries(None)
        .iter()
        .filter_map(|entry| describe_recovery(&sessions, entry))
        .collect()
}

/// Restores recovered changes into this window's document, which must be
/// the one they belong to. They become unsaved changes; Ctrl+Z goes back to
/// the file's text.
#[tauri::command]
pub async fn restore_recovered(
    window: tauri::WebviewWindow,
    registry: tauri::State<'_, AssetRegistry>,
    store: tauri::State<'_, StateStore>,
    themes: tauri::State<'_, ThemeService>,
    sessions: tauri::State<'_, Sessions>,
    vaults: tauri::State<'_, VaultCache>,
    id: String,
) -> Result<OpenedDocument, CommandError> {
    let (path, bytes) = sessions.restore_recovery(window.label(), &id)?;
    tracing::info!(path = %path.display(), "restored recovered changes");
    let memory = store.memory(&path).unwrap_or_default();
    let doc = parse_in_background(&sessions, window.label(), path, bytes, &memory, &vaults).await?;
    let dirty = sessions.is_dirty(window.label());
    let opened = finish_open(&window, &registry, &store, &themes, doc, memory, dirty)?;
    Ok(send_model(&sessions, window.label(), opened, false))
}

/// Deletes recovered changes the user chose not to restore.
#[tauri::command]
pub fn discard_recovered(
    sessions: tauri::State<'_, Sessions>,
    id: String,
) -> Result<(), CommandError> {
    sessions.discard_recovery(&id)?;
    tracing::info!("discarded recovered changes");
    Ok(())
}

/// The text of recovered changes, for copying when their file is gone.
#[tauri::command]
pub fn recovered_text(
    sessions: tauri::State<'_, Sessions>,
    id: String,
) -> Result<String, CommandError> {
    Ok(sessions.recovery_text(&id)?.1)
}

/// How the file on disk compares with what this window last read or saved,
/// so a change notification caused by Scrald's own save can be ignored. A
/// changed version is remembered for `disk_text` and `load_disk_version`.
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
    window: tauri::WebviewWindow,
    document: PathBuf,
) -> Result<Option<String>, CommandError> {
    use tauri_plugin_dialog::DialogExt;
    let doc_dir = document.parent().map(PathBuf::from).unwrap_or_default();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        // Owned by the window: modal to it, and focus returns to it after.
        window
            .dialog()
            .file()
            .set_parent(&window)
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
    app: tauri::AppHandle,
    themes: tauri::State<'_, ThemeService>,
    fonts: tauri::State<'_, crate::fonts::FontCache>,
    id: String,
) -> Result<ThemeStyle, CommandError> {
    let theme = themes
        .get(&id)
        .ok_or_else(|| anyhow::anyhow!("no theme named {id}"))?;
    Ok(style_for(&app, &fonts, theme))
}

/// What the theme editor shows for a theme (DESIGN.md §7.2).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeDetails {
    pub id: String,
    /// Only themes in the user folder can be edited.
    pub editable: bool,
    /// The theme's folder, for user themes.
    pub folder: Option<String>,
    pub values: scrald_core::theme::edit::EditorValues,
    /// Color pairs below WCAG AA, worst first (DESIGN.md §7.2).
    pub contrast: Vec<ContrastProblem>,
}

#[tauri::command]
pub fn theme_details(
    themes: tauri::State<'_, ThemeService>,
    id: String,
) -> Result<ThemeDetails, CommandError> {
    let theme = themes
        .get(&id)
        .ok_or_else(|| anyhow::anyhow!("no theme named {id}"))?;
    let folder = match &theme.origin {
        scrald_core::theme::ThemeOrigin::User { dir } => Some(dir.display().to_string()),
        scrald_core::theme::ThemeOrigin::Bundled => None,
    };
    Ok(ThemeDetails {
        editable: folder.is_some(),
        values: scrald_core::theme::edit::EditorValues::from_theme(&theme.file),
        contrast: contrast_problems(&theme.file),
        id: theme.id,
        folder,
    })
}

/// Theme `id`'s color pairs below WCAG AA, worst first, for the switcher's
/// "Check contrast" button and after an import (DESIGN.md §7.2).
#[tauri::command]
pub fn theme_contrast(
    themes: tauri::State<'_, ThemeService>,
    id: String,
) -> Result<Vec<ContrastProblem>, CommandError> {
    let theme = themes
        .get(&id)
        .ok_or_else(|| anyhow::anyhow!("no theme named {id}"))?;
    Ok(contrast_problems(&theme.file))
}

/// A theme as it would be with the editor's changes: its styles, and the
/// editor's values for it (derived colors follow the edited ones).
#[derive(Debug, Clone, Serialize)]
pub struct ThemeEditPreview {
    pub style: ThemeStyle,
    pub values: scrald_core::theme::edit::EditorValues,
    /// The edited theme's color pairs below WCAG AA, worst first.
    pub contrast: Vec<ContrastProblem>,
}

/// Theme `id` as it would look with the editor's `changes`, without saving.
/// Fails with the problems the edited theme would have.
#[tauri::command]
pub fn preview_theme_edit(
    app: tauri::AppHandle,
    themes: tauri::State<'_, ThemeService>,
    fonts: tauri::State<'_, crate::fonts::FontCache>,
    id: String,
    changes: Vec<scrald_core::theme::edit::ThemeChange>,
) -> Result<ThemeEditPreview, CommandError> {
    let (theme, _) = themes.edited(&id, &changes)?;
    let values = scrald_core::theme::edit::EditorValues::from_theme(&theme.file);
    let contrast = contrast_problems(&theme.file);
    Ok(ThemeEditPreview {
        style: style_for(&app, &fonts, theme),
        values,
        contrast,
    })
}

/// Saves the editor's `changes` into theme `id`'s `theme.toml`.
#[tauri::command]
pub fn save_theme_edit(
    themes: tauri::State<'_, ThemeService>,
    id: String,
    changes: Vec<scrald_core::theme::edit::ThemeChange>,
) -> Result<(), CommandError> {
    Ok(themes.save_edit(&id, &changes)?)
}

fn style_for(
    app: &tauri::AppHandle,
    fonts: &crate::fonts::FontCache,
    theme: scrald_core::theme::Theme,
) -> ThemeStyle {
    // Google Fonts the theme uses: rules for downloaded ones, and downloads
    // started for the others (DESIGN.md §7.5).
    let mut css = fonts.css_for(app, &theme.file);
    css.push_str(&theme.css(&crate::themes::asset_base(&theme.id)));
    ThemeStyle {
        css,
        name: theme.file.meta.name.clone(),
        appearance: theme.file.meta.appearance,
        numbering: theme.file.elements.headings.numbering,
        front_cover: theme.file.elements.front.cover,
        front_title: theme.file.elements.front.title,
        layout: ThemeLayout {
            measure: theme.file.layout.measure,
            font_size: theme.file.layout.font_size,
            min_font_size: theme.file.layout.min_font_size,
            max_font_size: theme.file.layout.max_font_size,
            text_sizing: theme.file.layout.text_sizing,
        },
        id: theme.id,
    }
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

/// Where the properties panel goes: "top", "left", or "right".
#[tauri::command]
pub fn properties_placement(
    store: tauri::State<'_, StateStore>,
) -> Result<PropertiesPlacement, CommandError> {
    Ok(store.properties_placement()?)
}

/// Moves the properties panel, in every window.
#[tauri::command]
pub fn set_properties_placement(
    app: tauri::AppHandle,
    store: tauri::State<'_, StateStore>,
    placement: PropertiesPlacement,
) -> Result<(), CommandError> {
    store.set_properties_placement(placement)?;
    settings_changed(&app, "properties.placement")
}

/// Event sent to every window when autosave is turned on or off, with the
/// new value.
pub const AUTOSAVE_CHANGED: &str = "autosave-changed";

/// Whether documents save themselves after a pause in editing (off by
/// default, DESIGN.md §9.3).
#[tauri::command]
pub fn autosave(store: tauri::State<'_, StateStore>) -> Result<bool, CommandError> {
    Ok(store.autosave()?)
}

/// Turns autosave on or off, for every window.
#[tauri::command]
pub fn set_autosave(
    app: tauri::AppHandle,
    store: tauri::State<'_, StateStore>,
    enabled: bool,
) -> Result<(), CommandError> {
    store.set_autosave(enabled)?;
    tracing::info!(enabled, "autosave");
    // Rust note: `Emitter` is a trait; its `emit` method is only callable
    // once the trait is in scope, hence the `use` inside the function.
    use tauri::Emitter;
    app.emit(AUTOSAVE_CHANGED, enabled)?;
    Ok(())
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
    app: tauri::AppHandle,
    store: tauri::State<'_, StateStore>,
    defaults: TypographyDefaults,
) -> Result<(), CommandError> {
    store.set_typography_defaults(defaults)?;
    settings_changed(&app, "typography")
}

/// Event sent to every window when a global setting changes, with the
/// setting's name: "theme.default", "typography", "contents.depth",
/// "properties.placement", or "reader.front".
/// (Autosave has its own event; `properties.open` isn't announced, since
/// it's only how the panel starts and open windows keep theirs.)
pub const SETTINGS_CHANGED: &str = "settings-changed";

fn settings_changed(app: &tauri::AppHandle, name: &str) -> Result<(), CommandError> {
    use tauri::Emitter;
    app.emit(SETTINGS_CHANGED, name)?;
    Ok(())
}

/// The deepest heading level the contents sidebar lists (1–6).
#[tauri::command]
pub fn contents_depth(store: tauri::State<'_, StateStore>) -> Result<u8, CommandError> {
    Ok(store.contents_depth()?)
}

#[tauri::command]
pub fn set_contents_depth(
    app: tauri::AppHandle,
    store: tauri::State<'_, StateStore>,
    depth: u8,
) -> Result<(), CommandError> {
    store.set_contents_depth(depth)?;
    settings_changed(&app, "contents.depth")
}

/// Whether documents show their cover and a missing title: the theme's
/// choice unless the user overrides it (#189).
#[tauri::command]
pub fn front_settings(store: tauri::State<'_, StateStore>) -> Result<FrontSettings, CommandError> {
    Ok(store.front_settings()?)
}

#[tauri::command]
pub fn set_front_settings(
    app: tauri::AppHandle,
    store: tauri::State<'_, StateStore>,
    settings: FrontSettings,
) -> Result<(), CommandError> {
    store.set_front_settings(settings)?;
    settings_changed(&app, "reader.front")
}

/// The global default theme's id; `null` when the built-in light or dark
/// theme follows the system.
#[tauri::command]
pub fn default_theme(store: tauri::State<'_, StateStore>) -> Result<Option<String>, CommandError> {
    Ok(store.default_theme()?)
}

/// Sets the theme for one document, or with `null` returns it to its
/// default (front matter, folder, or global).
#[tauri::command]
pub fn set_document_theme(
    store: tauri::State<'_, StateStore>,
    path: PathBuf,
    id: Option<String>,
) -> Result<(), CommandError> {
    store.set_document_theme(&path, id.as_deref())?;
    // The start screen's swatch follows a theme chosen here. (Returning to
    // the default is picked up at the next open.)
    if let Some(id) = &id {
        store.set_document_display(&path, None, Some(id))?;
    }
    Ok(())
}

/// Sets the global default theme, or with `null` goes back to following the
/// system's light or dark mode.
#[tauri::command]
pub fn set_default_theme(
    app: tauri::AppHandle,
    store: tauri::State<'_, StateStore>,
    id: Option<String>,
) -> Result<(), CommandError> {
    store.set_default_theme(id.as_deref())?;
    settings_changed(&app, "theme.default")
}

/// Copies a theme into the user theme folder to customize; returns the new id.
#[tauri::command]
pub fn duplicate_theme(
    themes: tauri::State<'_, ThemeService>,
    id: String,
) -> Result<String, CommandError> {
    Ok(themes.duplicate(&id)?)
}

/// A terminal theme file the user picked, and the schemes in it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalThemeFile {
    pub path: PathBuf,
    pub names: Vec<String>,
}

/// Terminal theme files are a few KB; anything far bigger isn't one.
const TERMINAL_THEME_MAX_BYTES: u64 = 4 * 1024 * 1024;

fn read_terminal_theme(path: &Path) -> anyhow::Result<String> {
    let size = std::fs::metadata(path)
        .with_context(|| format!("could not read {}", path.display()))?
        .len();
    anyhow::ensure!(
        size <= TERMINAL_THEME_MAX_BYTES,
        "{} is too large to be a terminal theme",
        path.display()
    );
    std::fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))
}

/// Asks for a terminal color scheme file (DESIGN.md §7.3) and lists the
/// schemes in it; `None` if the user cancelled. Fails if the file holds no
/// schemes Scrald can read.
#[tauri::command]
pub async fn pick_terminal_theme(
    window: tauri::WebviewWindow,
) -> Result<Option<TerminalThemeFile>, CommandError> {
    use tauri_plugin_dialog::DialogExt;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Import a terminal theme")
            .add_filter(
                "Terminal themes",
                &["json", "itermcolors", "toml", "conf", "yaml", "yml"],
            )
            // Ghostty's theme files have no extension.
            .add_filter("All files", &["*"])
            .blocking_pick_file()
    })
    .await
    .context("file picker failed")?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };
    let text = read_terminal_theme(&path)?;
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    let (schemes, _) = scrald_core::theme::import::read_schemes(&text, &file_name)
        .map_err(|e| anyhow::anyhow!("{file_name}: {e}"))?;
    let names = schemes.into_iter().map(|s| s.name).collect();
    Ok(Some(TerminalThemeFile { path, names }))
}

/// Creates a theme for each scheme named in `names` from the terminal theme
/// file at `path` (chosen with `pick_terminal_theme`). Returns the new ids.
#[tauri::command]
pub fn import_terminal_theme(
    themes: tauri::State<'_, ThemeService>,
    path: PathBuf,
    names: Vec<String>,
) -> Result<Vec<String>, CommandError> {
    let text = read_terminal_theme(&path)?;
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    Ok(themes.import_terminal(&text, &file_name, &names)?)
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

/// Recently opened documents for the start screen: pinned first, then
/// newest first, each marked if its file is gone.
///
/// Cover images that still exist are registered for this window, so the
/// page can load them by token (DESIGN.md §6.2); only covers read from the
/// front matter of documents the user opened are ever served.
#[tauri::command]
pub async fn recent_documents(
    window: tauri::WebviewWindow,
    store: tauri::State<'_, StateStore>,
    registry: tauri::State<'_, AssetRegistry>,
    limit: usize,
) -> Result<Vec<RecentDocument>, CommandError> {
    let mut recent = store.recent_documents(limit)?;
    // Checking a file on a disconnected network drive can take seconds, so
    // it happens on a worker thread.
    let (mut recent, covers) = tauri::async_runtime::spawn_blocking(move || {
        let mut covers: Vec<(usize, PathBuf)> = Vec::new();
        for (row, document) in recent.iter_mut().enumerate() {
            document.exists = Path::new(&document.path).is_file();
            if document.exists
                && let Some(cover) = document.cover_path.as_deref().map(PathBuf::from)
                && cover.is_file()
            {
                covers.push((row, cover));
            }
        }
        (recent, covers)
    })
    .await
    .context("checking recent documents failed")?;
    let files = covers.iter().map(|(_, file)| file.clone()).collect();
    let token = registry.register_covers(window.label(), files);
    for (id, (row, _)) in covers.iter().enumerate() {
        recent[*row].cover = Some(CoverImage { token, id });
    }
    Ok(recent)
}

/// Pins a document to the top of the start screen, or unpins it.
#[tauri::command]
pub fn set_document_pinned(
    store: tauri::State<'_, StateStore>,
    path: PathBuf,
    pinned: bool,
) -> Result<(), CommandError> {
    Ok(store.set_pinned(&path, pinned)?)
}

/// "Remove from list": forgets everything remembered about a document.
#[tauri::command]
pub fn forget_document(
    store: tauri::State<'_, StateStore>,
    path: PathBuf,
) -> Result<(), CommandError> {
    Ok(store.forget_document(&path)?)
}

/// Asks for a Markdown file to open; `None` if the user cancelled. Starts
/// in `near`'s folder when given (the open document). Runs in Rust so the
/// frontend needs no file dialog permission.
#[tauri::command]
pub async fn pick_document(
    window: tauri::WebviewWindow,
    near: Option<PathBuf>,
) -> Result<Option<PathBuf>, CommandError> {
    use tauri_plugin_dialog::DialogExt;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Open")
            .add_filter("Markdown", MARKDOWN_EXTENSIONS)
            .add_filter("All files", &["*"]);
        // Rust note: builder methods take `self` by value and return it, so
        // an optional step reassigns the variable (hence `let mut`).
        if let Some(folder) = near.as_deref().and_then(Path::parent) {
            dialog = dialog.set_directory(folder);
        }
        dialog.blocking_pick_file()
    })
    .await
    .context("file picker failed")?;
    Ok(picked.and_then(|p| p.into_path().ok()))
}

/// File extensions Scrald opens as Markdown (dialogs, drag and drop).
pub const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown", "mdown", "mkd", "mkdn", "txt"];

/// A folder the New document dialog offers (DESIGN.md §11).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderChoice {
    pub path: String,
    /// In the user's list of remembered folders.
    pub remembered: bool,
    /// The template last used there (remembered folders only).
    pub template: Option<String>,
    /// Whether the folder is still there; missing ones are only listed in
    /// the settings, to be removed.
    pub exists: bool,
}

/// What the New document dialog and the settings offer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewDocumentOptions {
    /// Remembered folders first (missing ones included), then the folders
    /// of recent documents, then the Documents folder if nothing else is.
    pub folders: Vec<FolderChoice>,
    /// The folder of the most recently opened document, else the first
    /// remembered folder, else the Documents folder.
    pub default_folder: Option<String>,
    pub templates: Vec<crate::newdoc::TemplateChoice>,
    /// Whether "Remember this folder" starts checked.
    pub remember_by_default: bool,
    pub templates_folder: Option<String>,
}

/// How many recent documents' folders the dialog suggests.
const RECENT_FOLDERS: usize = 5;

/// The folders, templates, and defaults for a new document.
#[tauri::command]
pub async fn new_document_options(
    app: tauri::AppHandle,
    store: tauri::State<'_, StateStore>,
    templates: tauri::State<'_, crate::newdoc::Templates>,
) -> Result<NewDocumentOptions, CommandError> {
    let remembered = store.new_document_folders()?;
    let recent = store.last_opened_paths(50)?;
    let documents = app.path().document_dir().ok();
    // Checking folders on a disconnected network drive can take seconds, so
    // it happens on a worker thread (like the start screen's list).
    let (folders, default_folder) = tauri::async_runtime::spawn_blocking(move || {
        let mut folders: Vec<FolderChoice> = remembered
            .into_iter()
            .map(|f| FolderChoice {
                exists: Path::new(&f.path).is_dir(),
                path: f.path,
                remembered: true,
                template: f.template,
            })
            .collect();
        let mut recent_folders: Vec<String> = Vec::new();
        for document in &recent {
            let Some(folder) = Path::new(document).parent() else {
                continue;
            };
            let folder = folder.display().to_string();
            if !recent_folders.contains(&folder) && Path::new(&folder).is_dir() {
                recent_folders.push(folder);
            }
            if recent_folders.len() == RECENT_FOLDERS {
                break;
            }
        }
        let default_folder = recent_folders
            .first()
            .cloned()
            .or_else(|| folders.iter().find(|f| f.exists).map(|f| f.path.clone()))
            .or_else(|| documents.as_ref().map(|d| d.display().to_string()));
        // Rust note: an `Option` is also an iterator of zero or one items, so
        // `chain(option)` appends the default folder only when there is one.
        for folder in recent_folders.into_iter().chain(default_folder.clone()) {
            if !folders.iter().any(|f| f.path == folder) {
                folders.push(FolderChoice {
                    path: folder,
                    remembered: false,
                    template: None,
                    exists: true,
                });
            }
        }
        (folders, default_folder)
    })
    .await
    .context("checking folders failed")?;
    Ok(NewDocumentOptions {
        folders,
        default_folder,
        templates: templates.list(),
        remember_by_default: store.remember_new_folders()?,
        templates_folder: templates.dir().map(|d| d.display().to_string()),
    })
}

/// Shows the system folder dialog for a new document's folder (or one to
/// remember, from the settings), starting in `near` when given.
#[tauri::command]
pub async fn pick_folder(
    window: tauri::WebviewWindow,
    near: Option<PathBuf>,
) -> Result<Option<PathBuf>, CommandError> {
    use tauri_plugin_dialog::DialogExt;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Choose a folder");
        if let Some(folder) = near.filter(|f| f.is_dir()) {
            dialog = dialog.set_directory(folder);
        }
        dialog.blocking_pick_folder()
    })
    .await
    .context("folder picker failed")?;
    Ok(picked.and_then(|p| p.into_path().ok()))
}

/// What the New document dialog asks for.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewDocumentRequest {
    pub folder: PathBuf,
    /// As typed; `.md` is added unless it has a Markdown extension.
    pub file_name: String,
    /// Empty or missing for none: then the template is ignored (Plain).
    pub title: Option<String>,
    pub template: String,
    /// Today's date where the user is, `2026-10-19` (the frontend knows the
    /// time zone).
    pub date: String,
    /// Add the folder to the remembered ones.
    pub remember: bool,
}

/// Creates a new document and returns its path; the frontend then opens it.
/// A built-in template also sets the document's flavor, as the user's
/// choice for it (DESIGN.md §11).
#[tauri::command]
pub fn create_document(
    store: tauri::State<'_, StateStore>,
    templates: tauri::State<'_, crate::newdoc::Templates>,
    request: NewDocumentRequest,
) -> Result<PathBuf, CommandError> {
    let name = scrald_core::newdoc::file_name(&request.file_name)?;
    let title = request
        .title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty());
    // A template needs a title; without one the document is plain.
    let template_id = match title {
        Some(_) => request.template.as_str(),
        None => scrald_core::newdoc::BuiltinTemplate::Plain.id(),
    };
    let (template, flavor) = templates.text(template_id)?;
    let text = scrald_core::newdoc::starting_text(&template, title, &request.date)?;
    let path = crate::newdoc::create_file(&request.folder, &name, &text)?;
    tracing::info!(path = %path.display(), template = template_id, "created document");

    if let Some(flavor) = flavor {
        // The flavor is stored with the document's row, which opening makes.
        store.record_open(&path)?;
        store.set_document_flavor(&path, Some(flavor))?;
    }
    if request.remember {
        store.remember_new_document_folder(&request.folder, title.map(|_| template_id))?;
    } else if title.is_some() {
        store.set_new_document_folder_template(&request.folder, template_id)?;
    }
    Ok(path)
}

/// Adds a folder to the remembered ones (from the settings).
#[tauri::command]
pub fn remember_new_document_folder(
    store: tauri::State<'_, StateStore>,
    path: PathBuf,
) -> Result<(), CommandError> {
    Ok(store.remember_new_document_folder(&path, None)?)
}

#[tauri::command]
pub fn forget_new_document_folder(
    store: tauri::State<'_, StateStore>,
    path: PathBuf,
) -> Result<(), CommandError> {
    Ok(store.forget_new_document_folder(&path)?)
}

#[tauri::command]
pub fn remember_new_folders(store: tauri::State<'_, StateStore>) -> Result<bool, CommandError> {
    Ok(store.remember_new_folders()?)
}

/// Sets whether "Remember this folder" starts checked.
#[tauri::command]
pub fn set_remember_new_folders(
    store: tauri::State<'_, StateStore>,
    remember: bool,
) -> Result<(), CommandError> {
    Ok(store.set_remember_new_folders(remember)?)
}

/// Opens the templates folder in the system file manager. Only that
/// folder: the page can't ask for any other path.
#[tauri::command]
pub fn open_templates_folder(
    app: tauri::AppHandle,
    templates: tauri::State<'_, crate::newdoc::Templates>,
) -> Result<(), CommandError> {
    let dir = templates
        .dir()
        .context("there's no templates folder")?
        .to_path_buf();
    std::fs::create_dir_all(&dir).with_context(|| format!("could not create {}", dir.display()))?;
    app.opener()
        .open_path(dir.display().to_string(), None::<&str>)?;
    Ok(())
}

/// The theme for a window without a document (the start screen): the
/// global default, else the built-in one for the system's light or dark mode.
#[tauri::command]
pub fn start_theme(
    window: tauri::WebviewWindow,
    store: tauri::State<'_, StateStore>,
    themes: tauri::State<'_, ThemeService>,
) -> ResolvedTheme {
    let default = store.get_setting::<String>(DEFAULT_THEME).ok().flatten();
    let system_dark = matches!(window.theme(), Ok(tauri::Theme::Dark));
    let (id, source) = themes.with_library(|library| {
        resolve_theme(library, None, None, None, default.as_deref(), system_dark)
    });
    ResolvedTheme { id, source }
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

/// Opens the system's print dialog for the window's page (DESIGN.md §10).
/// The page gets itself ready first, and puts itself back when the browser
/// reports `afterprint`. It goes through Tauri rather than `window.print()`
/// because macOS's webview only prints from the native side.
#[tauri::command]
pub fn print_window(webview: tauri::Webview) -> Result<(), CommandError> {
    webview.print()?;
    Ok(())
}

/// Logs a timing measured in the frontend, so performance numbers end up in
/// the same log as the backend's (DESIGN.md §4 budget checks).
#[tauri::command]
pub fn report_timing(name: String, ms: f64) {
    tracing::info!(name, ms = (ms * 10.0).round() / 10.0, "frontend timing");
}

/// Logs the web features the reader uses that this window's webview lacks
/// (DESIGN.md §11.1), so checking WKWebView or WebKitGTK only needs the log.
#[tauri::command]
pub fn report_webview(user_agent: String, missing: Vec<String>) {
    if missing.is_empty() {
        tracing::info!(user_agent, "webview has every feature the reader uses");
    } else {
        tracing::warn!(
            user_agent,
            missing = missing.join(", "),
            "webview lacks features; using fallbacks"
        );
    }
}

/// What a window was opened with: the file from the command line (of this
/// process, or of a later launch handed over to it), or nothing.
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
        let cwd = std::env::current_dir().unwrap_or_default();
        Self::from_arg_in(arg, &cwd)
    }

    /// Like `from_arg`, with a relative path taken from `cwd`: the folder a
    /// later launch was started in, which needn't be this process's.
    pub fn from_arg_in(arg: Option<PathBuf>, cwd: &Path) -> Self {
        // Rust note: `map` transforms the value inside an Option only if it's
        // present, much like Python's `f(x) if x is not None else None`.
        let path = arg.map(|p| {
            let joined = cwd.join(&p);
            std::path::absolute(&joined).unwrap_or(joined)
        });
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

    /// The window title: the document's, or "Scrald" for the start screen
    /// (also shown when the file doesn't exist).
    pub fn window_title(&self) -> String {
        if self.exists {
            format!("{} \u{2014} Scrald", self.title)
        } else {
            "Scrald".to_string()
        }
    }
}

/// Returns the path and title this window was opened with.
// Rust note: `State<'_, T>` borrows Tauri's managed state for the duration of
// this call; `'_` tells the compiler "infer the lifetime", so you rarely need
// to name it.
#[tauri::command]
pub fn launch_info(window: tauri::Window, launches: tauri::State<'_, Launches>) -> LaunchInfo {
    launches.get(window.label())
}

/// What each window was opened with, by window label.
#[derive(Default)]
pub struct Launches {
    inner: std::sync::Mutex<std::collections::HashMap<String, LaunchInfo>>,
}

impl Launches {
    pub fn set(&self, window: &str, launch: LaunchInfo) {
        self.lock().insert(window.to_string(), launch);
    }

    /// The window's launch info; nothing (the start screen) if it has none.
    pub fn get(&self, window: &str) -> LaunchInfo {
        self.lock()
            .get(window)
            .cloned()
            .unwrap_or_else(|| LaunchInfo::from_arg(None))
    }

    pub fn forget(&self, window: &str) {
        self.lock().remove(window);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, std::collections::HashMap<String, LaunchInfo>> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
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
        assert_eq!(info.window_title(), "Cargo \u{2014} Scrald");
    }

    #[test]
    fn relative_argument_is_taken_from_the_given_folder() {
        // A later launch's relative path belongs to the folder it ran in.
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let info = LaunchInfo::from_arg_in(Some(PathBuf::from("Cargo.toml")), &crate_dir);
        assert_eq!(info.path, Some(crate_dir.join("Cargo.toml")));
        assert!(info.exists);
        // An absolute path ignores the folder.
        let manifest = crate_dir.join("Cargo.toml");
        let elsewhere = LaunchInfo::from_arg_in(Some(manifest.clone()), Path::new("/elsewhere"));
        assert_eq!(elsewhere.path, Some(manifest));
    }

    #[test]
    fn missing_file_titles_the_window_scrald() {
        let info = LaunchInfo::from_arg(Some(PathBuf::from("does-not-exist.md")));
        assert_eq!(info.window_title(), "Scrald");
    }

    #[test]
    fn windows_without_launch_info_show_the_start_screen() {
        let launches = Launches::default();
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        launches.set("main", LaunchInfo::from_arg(Some(manifest.clone())));
        assert_eq!(launches.get("main").path, Some(manifest));
        assert_eq!(launches.get("window-1").path, None);
        launches.forget("main");
        assert_eq!(launches.get("main").path, None);
    }
}
