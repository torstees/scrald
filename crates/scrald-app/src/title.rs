//! Window titles with the document's path. The system title bar shows them
//! on Windows and Linux; on macOS the page draws its own title in the title
//! bar, so it can shorten the path from the start and keep the file name
//! (`ui/src/chrome/TitleBar.svelte`). Each window's current title is kept
//! here, and the page is sent every change.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{Emitter, EventTarget, Manager};

/// Sent to a window's page whenever its title changes.
pub const WINDOW_TITLE: &str = "window-title";

/// What a window's title says.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowTitle {
    /// The document's title, or "Scrald" for the start screen.
    pub title: String,
    /// Whether the document has unsaved changes.
    pub dirty: bool,
    /// The document's path as shown, with the home folder as `~` on macOS
    /// and Linux; `None` for the start screen.
    pub path: Option<String>,
    /// The full path, for the tooltip and for copying.
    pub full_path: Option<String>,
}

impl WindowTitle {
    /// The start screen's title.
    pub fn start() -> Self {
        WindowTitle {
            title: "Scrald".to_string(),
            ..WindowTitle::default()
        }
    }

    /// A document's title. `home` is the home folder, written `~` in the path.
    pub fn document(title: &str, path: &Path, dirty: bool, home: Option<&Path>) -> Self {
        WindowTitle {
            title: title.to_string(),
            dirty,
            path: Some(scrald_core::display_path(path, home)),
            full_path: Some(path.display().to_string()),
        }
    }

    /// The text for the system title bar: "• Title — ~/notes/a.md". The
    /// title comes first, so it's what stays when the system cuts the end.
    pub fn text(&self) -> String {
        let marker = if self.dirty { "\u{2022} " } else { "" };
        match &self.path {
            Some(path) => format!("{marker}{} \u{2014} {path}", self.title),
            None => format!("{marker}{}", self.title),
        }
    }
}

/// Each window's current title, by window label.
#[derive(Default)]
pub struct WindowTitles {
    inner: Mutex<HashMap<String, WindowTitle>>,
}

impl WindowTitles {
    fn set(&self, label: &str, title: WindowTitle) {
        let mut titles = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        titles.insert(label.to_string(), title);
    }

    pub fn get(&self, label: &str) -> WindowTitle {
        let titles = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        titles
            .get(label)
            .cloned()
            .unwrap_or_else(WindowTitle::start)
    }

    pub fn forget(&self, label: &str) {
        let mut titles = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        titles.remove(label);
    }
}

/// The home folder to write as `~`: on macOS and Linux, where that's
/// usual. Windows paths are shown in full.
pub fn home_for_display<R: tauri::Runtime>(
    manager: &impl Manager<R>,
) -> Option<std::path::PathBuf> {
    if cfg!(windows) {
        None
    } else {
        manager.path().home_dir().ok()
    }
}

/// Sets a window's title: the system's, the one kept here, and the page's.
// Rust note: `impl Manager<R>` accepts any Tauri type that can reach the app
// (a `Window`, a `WebviewWindow`, ...), like a template parameter in C++.
pub fn set_title<R: tauri::Runtime>(
    window: &tauri::Window<R>,
    title: WindowTitle,
) -> tauri::Result<()> {
    window.set_title(&title.text())?;
    if let Some(titles) = window.try_state::<WindowTitles>() {
        titles.set(window.label(), title.clone());
    }
    let target = EventTarget::webview_window(window.label());
    if let Err(error) = window.app_handle().emit_to(target, WINDOW_TITLE, title) {
        tracing::warn!(%error, "could not send the window its title");
    }
    Ok(())
}

/// The window's current title, for its page to draw (macOS).
#[tauri::command]
pub fn window_title(window: tauri::Window, titles: tauri::State<'_, WindowTitles>) -> WindowTitle {
    titles.get(window.label())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_text_puts_the_title_before_the_path() {
        let home = Path::new("/Users").join("ann");
        let path = home.join("notes").join("a.md");
        let title = WindowTitle::document("Groceries", &path, true, Some(&home));
        let shown = Path::new("~").join("notes").join("a.md");
        assert_eq!(
            title.text(),
            format!("\u{2022} Groceries \u{2014} {}", shown.display())
        );
        assert_eq!(title.full_path, Some(path.display().to_string()));
        assert_eq!(WindowTitle::start().text(), "Scrald");
    }
}
