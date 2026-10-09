//! Themes in the app (DESIGN.md §7): the theme library with hot reload of
//! the user theme folder, the `scrald-theme` URI scheme for theme assets,
//! and system font enumeration.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};
use scrald_core::theme::{Theme, ThemeLibrary, ThemeOrigin, ThemeSummary};
use tauri::http::{Request, Response, StatusCode, header};
use tauri::{AppHandle, Emitter};

pub const SCHEME: &str = "scrald-theme";

/// Event sent to every window when a user theme file changes.
pub const THEMES_CHANGED: &str = "themes-changed";

/// The URL prefix for files in theme `id`'s folder. Tauri serves custom
/// schemes as `http://<scheme>.localhost/` on Windows (and Android), and as
/// `<scheme>://localhost/` elsewhere (DESIGN.md §11.1).
pub fn asset_base(id: &str) -> String {
    if cfg!(any(windows, target_os = "android")) {
        format!("http://{SCHEME}.localhost/{id}/")
    } else {
        format!("{SCHEME}://localhost/{id}/")
    }
}

/// The loaded themes, reloaded whenever the user theme folder changes.
pub struct ThemeService {
    user_dir: Option<PathBuf>,
    library: Mutex<ThemeLibrary>,
    // Kept alive for as long as the service exists; dropping it stops watching.
    watcher: Mutex<Option<Debouncer<RecommendedWatcher>>>,
}

impl ThemeService {
    /// Loads bundled themes and those in `user_dir` (created if missing, so
    /// users can find where to put themes).
    pub fn new(user_dir: Option<PathBuf>) -> Self {
        if let Some(dir) = &user_dir
            && let Err(error) = std::fs::create_dir_all(dir)
        {
            tracing::warn!(dir = %dir.display(), %error, "could not create user theme folder");
        }
        let library = ThemeLibrary::load(user_dir.as_deref());
        ThemeService {
            user_dir,
            library: Mutex::new(library),
            watcher: Mutex::new(None),
        }
    }

    pub fn user_dir(&self) -> Option<&Path> {
        self.user_dir.as_deref()
    }

    fn library(&self) -> std::sync::MutexGuard<'_, ThemeLibrary> {
        self.library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn reload(&self) {
        *self.library() = ThemeLibrary::load(self.user_dir.as_deref());
    }

    pub fn summaries(&self) -> Vec<ThemeSummary> {
        self.library().summaries()
    }

    /// A copy of a theme (the library may reload at any moment).
    pub fn get(&self, id: &str) -> Option<Theme> {
        self.library().get(id).cloned()
    }

    /// Runs `f` with the current library, for theme resolution.
    // Rust note: taking a closure lets callers borrow the library briefly
    // without the lock guard escaping this method.
    pub fn with_library<T>(&self, f: impl FnOnce(&ThemeLibrary) -> T) -> T {
        f(&self.library())
    }

    pub fn duplicate(&self, id: &str) -> anyhow::Result<String> {
        let dir = self
            .user_dir
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("no user theme folder"))?;
        let new_id = self
            .library()
            .duplicate(id, dir)
            .map_err(anyhow::Error::msg)?;
        self.reload();
        Ok(new_id)
    }

    /// Watches the user theme folder and tells every window when it changes.
    pub fn watch(&self, app: &AppHandle) -> anyhow::Result<()> {
        let Some(dir) = self.user_dir.clone() else {
            return Ok(());
        };
        let app = app.clone();
        let mut debouncer = new_debouncer(
            Duration::from_millis(250),
            move |result: DebounceEventResult| {
                if let Err(error) = result {
                    tracing::warn!(%error, "theme folder watcher error");
                    return;
                }
                // Rust note: `state()` panics if the type was never managed;
                // `try_state` returns an Option instead.
                if let Some(service) = tauri::Manager::try_state::<ThemeService>(&app) {
                    service.reload();
                }
                tracing::debug!("user themes changed");
                if let Err(error) = app.emit(THEMES_CHANGED, ()) {
                    tracing::warn!(%error, "could not announce theme change");
                }
            },
        )?;
        debouncer.watcher().watch(&dir, RecursiveMode::Recursive)?;
        *self
            .watcher
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(debouncer);
        Ok(())
    }

    /// Serves a file from a user theme's folder: `/<theme-id>/<path>`.
    /// Bundled themes have no files. The resolved path must stay inside the
    /// theme folder, so `..` can't reach anything else.
    pub fn respond(&self, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
        let path = scrald_core::assets::percent_decode(request.uri().path());
        let Some((id, relative)) = path.trim_start_matches('/').split_once('/') else {
            return status(StatusCode::BAD_REQUEST);
        };
        let Some(Theme {
            origin: ThemeOrigin::User { dir },
            ..
        }) = self.get(id)
        else {
            return status(StatusCode::NOT_FOUND);
        };
        match confined_file(&dir, relative) {
            Some(file) => match std::fs::read(&file) {
                Ok(bytes) => Response::builder()
                    .header(header::CONTENT_TYPE, crate::protocol::mime_for(&file))
                    .body(bytes)
                    .unwrap_or_else(|_| status(StatusCode::INTERNAL_SERVER_ERROR)),
                Err(_) => status(StatusCode::NOT_FOUND),
            },
            None => {
                tracing::warn!(
                    theme = id,
                    path = relative,
                    "refused theme asset outside its folder"
                );
                status(StatusCode::FORBIDDEN)
            }
        }
    }
}

/// `dir/relative`, if it exists and (after resolving links and `..`) is
/// inside `dir`.
fn confined_file(dir: &Path, relative: &str) -> Option<PathBuf> {
    let root = std::fs::canonicalize(dir).ok()?;
    let file = std::fs::canonicalize(dir.join(relative)).ok()?;
    (file.starts_with(&root) && file.is_file()).then_some(file)
}

fn status(code: StatusCode) -> Response<Vec<u8>> {
    let mut response = Response::new(Vec::new());
    *response.status_mut() = code;
    response
}

/// Installed font families, sorted, without duplicates. Scanning the system
/// takes a moment, so it's done once and cached.
pub fn system_font_families() -> Vec<String> {
    static FAMILIES: OnceLock<Vec<String>> = OnceLock::new();
    // Rust note: `OnceLock` runs the closure the first time only, even if
    // several threads ask at once, like a thread-safe lazy static in C++.
    FAMILIES
        .get_or_init(|| {
            let mut db = fontdb::Database::new();
            db.load_system_fonts();
            let mut families: Vec<String> = db
                .faces()
                .flat_map(|face| face.families.iter().map(|(name, _)| name.clone()))
                .collect();
            families.sort_by_key(|name| name.to_lowercase());
            families.dedup();
            families
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("scrald-app-themes-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn request(path: &str) -> Request<Vec<u8>> {
        Request::builder()
            .uri(format!("http://scrald-theme.localhost{path}"))
            .body(Vec::new())
            .unwrap()
    }

    #[test]
    fn serves_files_inside_a_user_theme_only() {
        let dir = temp_dir("serve");
        let theme = dir.join("mine");
        std::fs::create_dir_all(theme.join("assets")).unwrap();
        std::fs::write(
            theme.join("theme.toml"),
            "[meta]\nname = \"Mine\"\nschema = 1\nappearance = \"light\"\n",
        )
        .unwrap();
        std::fs::write(theme.join("assets").join("rule one.svg"), "<svg/>").unwrap();
        std::fs::write(dir.join("outside.txt"), "secret").unwrap();
        let service = ThemeService::new(Some(dir.clone()));

        let ok = service.respond(&request("/mine/assets/rule%20one.svg"));
        assert_eq!(ok.status(), StatusCode::OK);
        assert_eq!(ok.body(), b"<svg/>");
        assert_eq!(
            service.respond(&request("/mine/../outside.txt")).status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            service.respond(&request("/mine/nope.png")).status(),
            StatusCode::FORBIDDEN
        );
        // Bundled themes have no folder to serve from.
        assert_eq!(
            service.respond(&request("/sepia/x.png")).status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            service.respond(&request("/nothing")).status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn duplicate_and_reload() {
        let dir = temp_dir("dup");
        let service = ThemeService::new(Some(dir.clone()));
        let id = service.duplicate("technical").unwrap();
        assert!(service.get(&id).is_some());
        assert!(dir.join(&id).join("theme.toml").is_file());
    }

    #[test]
    fn asset_base_has_trailing_slash() {
        assert!(asset_base("sepia").ends_with("/sepia/"));
    }
}
