//! Google Fonts in the app (DESIGN.md §7.5): a theme's Google font families
//! are downloaded once into the cache folder, recorded in the state database
//! (`font_cache`), and served from there under the theme scheme, so Scrald
//! contacts Google only the first time a theme needs a family. Until a
//! download finishes, the theme's fallback fonts show; then every window
//! re-applies its theme.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use anyhow::Context;
use scrald_core::theme::ThemeFile;
use scrald_core::theme::google_fonts::{
    CachedFace, cache_folder, css_url, font_face_css, parse_css, variant,
};
use scrald_core::theme::schema::FontSource;
use tauri::http::{Response, StatusCode, header};
use tauri::{AppHandle, Emitter, Manager};

use crate::state::StateStore;
use crate::themes::THEMES_CHANGED;

/// The first path segment of font URLs under the theme scheme. Theme ids
/// come from folder names, and no theme can be served under this one.
pub const URL_PREFIX: &str = "_google-fonts";

/// Google sends woff2 files only to browsers it knows support them.
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                          (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";

/// Limits, so a misbehaving server can't fill the disk or memory.
const MAX_CSS_BYTES: u64 = 1024 * 1024;
const MAX_FONT_BYTES: u64 = 10 * 1024 * 1024;
const MAX_FILES: usize = 200;
const TIMEOUT: Duration = Duration::from_secs(30);

/// Downloads a URL's body, up to `limit` bytes. Taken as a parameter so
/// tests can stand in for the network.
// Rust note: `&dyn Fn(...)` is a reference to any function or closure with
// that signature, chosen at run time (like `std::function` in C++); `type`
// names it once so the signature isn't repeated.
pub type Fetch<'a> = &'a dyn Fn(&str, u64) -> anyhow::Result<Vec<u8>>;

/// The Google Fonts cache folder and which families are being downloaded.
pub struct FontCache {
    dir: Option<PathBuf>,
    // Families started this run (downloading, done, or failed): each is
    // tried once per run, so an offline machine doesn't retry on every
    // theme change.
    started: Mutex<HashSet<String>>,
}

impl FontCache {
    pub fn new(dir: Option<PathBuf>) -> Self {
        FontCache {
            dir,
            started: Mutex::new(HashSet::new()),
        }
    }

    /// `@font-face` rules for the theme's Google fonts that are downloaded,
    /// and a background download for each that isn't.
    pub fn css_for(&self, app: &AppHandle, theme: &ThemeFile) -> String {
        let Some(dir) = &self.dir else {
            return String::new();
        };
        let Some(store) = app.try_state::<StateStore>() else {
            return String::new();
        };
        let fonts = [
            &theme.fonts.body,
            &theme.fonts.heading,
            &theme.fonts.mono,
            &theme.fonts.ui,
        ];
        let mut css = String::new();
        let mut seen = HashSet::new();
        for spec in fonts.into_iter().flatten() {
            if spec.source != FontSource::Google {
                continue;
            }
            let variant = variant(spec);
            if !seen.insert((spec.family.clone(), variant.clone())) {
                continue;
            }
            let folder = cache_folder(&spec.family, &variant);
            let cached = store
                .cached_font(&spec.family, &variant)
                .ok()
                .flatten()
                .filter(|faces| all_present(&dir.join(&folder), faces));
            match cached {
                Some(faces) => {
                    let base = format!("{}{folder}/", crate::themes::asset_base(URL_PREFIX));
                    css.push_str(&font_face_css(&spec.family, &faces, &base));
                }
                None => self.start_download(app, dir, &spec.family, &variant),
            }
        }
        css
    }

    fn start_download(&self, app: &AppHandle, dir: &Path, family: &str, variant: &str) {
        let key = format!("{family}:{variant}");
        let first = self
            .started
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(key);
        if !first {
            return;
        }
        let (app, dir, family, variant) = (
            app.clone(),
            dir.to_path_buf(),
            family.to_string(),
            variant.to_string(),
        );
        tracing::info!(family, variant, "downloading Google font");
        std::thread::spawn(move || match download(&dir, &family, &variant, &fetch) {
            Ok(faces) => {
                let store = app.state::<StateStore>();
                if let Err(error) = store.save_font(&family, &variant, &faces) {
                    tracing::warn!(error = format!("{error:#}"), "could not record font");
                    return;
                }
                tracing::info!(family, files = faces.len(), "Google font downloaded");
                // Every window re-applies its theme, now with the font.
                if let Err(error) = app.emit(THEMES_CHANGED, ()) {
                    tracing::warn!(%error, "could not announce the new font");
                }
            }
            Err(error) => {
                tracing::warn!(
                    family,
                    error = format!("{error:#}"),
                    "Google font download failed"
                )
            }
        });
    }

    /// Serves a downloaded font file: `relative` is `<folder>/<file>`, the
    /// part of the URL after `_google-fonts/`.
    pub fn respond(&self, relative: &str) -> Response<Vec<u8>> {
        let file = self
            .dir
            .as_deref()
            .and_then(|dir| crate::themes::confined_file(dir, relative));
        match file.map(std::fs::read) {
            Some(Ok(bytes)) => Response::builder()
                .header(header::CONTENT_TYPE, "font/woff2")
                // The page's origin differs from this scheme's, and browsers
                // load fonts across origins only when the response allows it.
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(bytes)
                .unwrap_or_else(|_| status(StatusCode::INTERNAL_SERVER_ERROR)),
            _ => status(StatusCode::NOT_FOUND),
        }
    }
}

fn all_present(folder: &Path, faces: &[CachedFace]) -> bool {
    !faces.is_empty() && faces.iter().all(|f| folder.join(&f.file).is_file())
}

fn status(code: StatusCode) -> Response<Vec<u8>> {
    let mut response = Response::new(Vec::new());
    *response.status_mut() = code;
    response
}

/// Downloads `family` in `variant` (see `google_fonts::variant`) into its
/// folder in `dir` and returns its faces. If Google refuses the styles (a
/// listed weight the family doesn't have), the family's default style is
/// downloaded instead.
pub fn download(
    dir: &Path,
    family: &str,
    variant: &str,
    fetch: Fetch<'_>,
) -> anyhow::Result<Vec<CachedFace>> {
    let wanted = css_url(family, Some(variant))
        .with_context(|| format!("\"{family}\" is not a Google Fonts family name"))?;
    let css = match fetch(&wanted, MAX_CSS_BYTES) {
        Ok(css) => css,
        Err(error) => {
            tracing::info!(
                family,
                error = format!("{error:#}"),
                "trying the default style"
            );
            let plain = css_url(family, None).context("invalid family")?;
            fetch(&plain, MAX_CSS_BYTES)?
        }
    };
    let faces = parse_css(&String::from_utf8_lossy(&css));
    anyhow::ensure!(!faces.is_empty(), "Google Fonts sent no usable font files");
    let folder = dir.join(cache_folder(family, variant));
    std::fs::create_dir_all(&folder)
        .with_context(|| format!("could not create {}", folder.display()))?;
    // Several faces can share a file (one variable font for many weights).
    let mut urls: Vec<&str> = Vec::new();
    let mut cached = Vec::new();
    for face in &faces {
        let index = match urls.iter().position(|u| *u == face.url) {
            Some(i) => i,
            None => {
                anyhow::ensure!(urls.len() < MAX_FILES, "too many font files");
                let bytes = fetch(&face.url, MAX_FONT_BYTES)?;
                let file = folder.join(format!("{}.woff2", urls.len()));
                write_atomically(&file, &bytes)?;
                urls.push(&face.url);
                urls.len() - 1
            }
        };
        cached.push(CachedFace {
            style: face.style.clone(),
            weight: face.weight.clone(),
            unicode_range: face.unicode_range.clone(),
            file: format!("{index}.woff2"),
        });
    }
    Ok(cached)
}

/// Writes to a temporary name and renames, so a half-written file never has
/// the real name.
fn write_atomically(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let temp = path.with_extension("part");
    std::fs::write(&temp, bytes).with_context(|| format!("could not write {}", temp.display()))?;
    std::fs::rename(&temp, path).with_context(|| format!("could not write {}", path.display()))
}

/// The real network: an HTTPS GET with a browser's user agent, a timeout,
/// and a size limit.
fn fetch(url: &str, limit: u64) -> anyhow::Result<Vec<u8>> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build()
        .into();
    let response = agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .with_context(|| format!("could not fetch {url}"))?;
    let mut bytes = Vec::new();
    // Rust note: `take(n)` wraps a reader so it stops after n bytes; one more
    // than the limit tells "exactly at the limit" from "too big".
    response
        .into_body()
        .into_reader()
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("could not read {url}"))?;
    anyhow::ensure!(bytes.len() as u64 <= limit, "{url} is too large");
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scrald-fonts-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const CSS: &str = "@font-face { font-family: 'Literata'; font-style: normal; font-weight: 400; \
        src: url(https://fonts.gstatic.com/s/literata/a.woff2) format('woff2'); unicode-range: U+0000-00FF; }
        @font-face { font-family: 'Literata'; font-style: normal; font-weight: 700; \
        src: url(https://fonts.gstatic.com/s/literata/a.woff2) format('woff2'); unicode-range: U+0000-00FF; }
        @font-face { font-family: 'Literata'; font-style: italic; font-weight: 400; \
        src: url(https://fonts.gstatic.com/s/literata/b.woff2) format('woff2'); }";

    #[test]
    fn downloads_each_file_once() {
        let dir = temp_dir("once");
        let fetched = Mutex::new(Vec::new());
        let fake = |url: &str, _limit: u64| -> anyhow::Result<Vec<u8>> {
            fetched.lock().unwrap().push(url.to_string());
            if url.contains("googleapis") {
                Ok(CSS.as_bytes().to_vec())
            } else {
                Ok(format!("font from {url}").into_bytes())
            }
        };
        let faces = download(&dir, "Literata", "ital,wght@0,400;0,700;1,400", &fake).unwrap();
        assert_eq!(faces.len(), 3);
        // Two weights share one variable font file.
        assert_eq!(faces[0].file, "0.woff2");
        assert_eq!(faces[1].file, "0.woff2");
        assert_eq!(faces[2].file, "1.woff2");
        assert_eq!(fetched.lock().unwrap().len(), 3, "CSS plus two files");
        let folder = dir.join(cache_folder("Literata", "ital,wght@0,400;0,700;1,400"));
        assert!(all_present(&folder, &faces));
        let bytes = std::fs::read(folder.join("1.woff2")).unwrap();
        assert!(String::from_utf8(bytes).unwrap().ends_with("b.woff2"));
        assert!(!folder.join("1.part").exists());
    }

    #[test]
    fn falls_back_to_the_default_style() {
        let dir = temp_dir("fallback");
        let fake = |url: &str, _limit: u64| -> anyhow::Result<Vec<u8>> {
            if url.contains("wght@") {
                anyhow::bail!("http status: 400");
            }
            if url.contains("googleapis") {
                return Ok(CSS.as_bytes().to_vec());
            }
            Ok(b"font".to_vec())
        };
        assert_eq!(
            download(&dir, "Literata", "wght@650", &fake).unwrap().len(),
            3
        );
    }

    #[test]
    fn failures_are_reported() {
        let dir = temp_dir("fail");
        let offline = |_: &str, _: u64| -> anyhow::Result<Vec<u8>> { anyhow::bail!("offline") };
        assert!(download(&dir, "Literata", "wght@400", &offline).is_err());
        let empty = |_: &str, _: u64| -> anyhow::Result<Vec<u8>> { Ok(b"/* nothing */".to_vec()) };
        let error = download(&dir, "Literata", "wght@400", &empty).unwrap_err();
        assert!(error.to_string().contains("no usable font files"));
        let error = download(&dir, "Not/A&Font", "wght@400", &empty).unwrap_err();
        assert!(error.to_string().contains("not a Google Fonts family name"));
    }

    #[test]
    fn serves_only_files_in_the_cache() {
        let dir = temp_dir("serve");
        std::fs::create_dir_all(dir.join("literata-1")).unwrap();
        std::fs::write(dir.join("literata-1").join("0.woff2"), b"font").unwrap();
        std::fs::write(dir.join("..").join("scrald-fonts-outside.txt"), b"secret").unwrap();
        let cache = FontCache::new(Some(dir.clone()));
        assert_eq!(cache.respond("literata-1/0.woff2").body(), b"font");
        assert_eq!(
            cache.respond("../scrald-fonts-outside.txt").status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            cache.respond("literata-1/9.woff2").status(),
            StatusCode::NOT_FOUND
        );
        let _ = std::fs::remove_file(dir.join("..").join("scrald-fonts-outside.txt"));
    }
}
