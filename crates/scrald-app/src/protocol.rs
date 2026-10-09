//! The `scrald-asset` URI scheme (DESIGN.md §6.2): serves the image files a
//! document resolved, and nothing else.
//!
//! URLs carry an opaque `<token>-<id>` instead of a file path. The token
//! identifies one opened document; the id indexes its resolved images. A
//! document can't make the webview read arbitrary files, because only paths
//! that core resolved for an open document are ever registered.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::http::{Request, Response, StatusCode, header};

pub const SCHEME: &str = "scrald-asset";

/// Which files may be served, per opened document.
// Rust note: Tauri state is shared between threads, so mutable data inside it
// needs a lock. `Mutex<T>` is like a `std::mutex` that owns the data it
// guards: you can only reach the `T` through `lock()`.
#[derive(Default)]
pub struct AssetRegistry {
    inner: Mutex<RegistryInner>,
}

#[derive(Default)]
struct RegistryInner {
    next_token: u64,
    /// Window label -> (token, files) for the document currently shown there.
    by_window: HashMap<String, (u64, Vec<PathBuf>)>,
}

impl AssetRegistry {
    /// Registers the files of the document now shown in `window`, replacing
    /// whatever that window showed before. Returns the document's token.
    pub fn register(&self, window: &str, files: Vec<PathBuf>) -> u64 {
        // A poisoned lock means another thread panicked mid-update; the map is
        // still usable, so recover it rather than failing every request.
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.next_token += 1;
        let token = inner.next_token;
        inner.by_window.insert(window.to_string(), (token, files));
        token
    }

    /// The file for `token` and `id`, if that document is still open.
    pub fn lookup(&self, token: u64, id: usize) -> Option<PathBuf> {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner
            .by_window
            .values()
            .find(|(t, _)| *t == token)
            .and_then(|(_, files)| files.get(id).cloned())
    }

    /// Forgets the document shown in a window (when the window closes).
    pub fn remove_window(&self, window: &str) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.by_window.remove(window);
    }
}

/// Parses a request path like `/12-3` into (token 12, id 3).
pub fn parse_asset_path(path: &str) -> Option<(u64, usize)> {
    let (token, id) = path.trim_start_matches('/').split_once('-')?;
    Some((token.parse().ok()?, id.parse().ok()?))
}

/// MIME type for an image file, from its extension.
pub fn mime_for(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

/// Builds the response for one asset request.
pub fn respond(registry: &AssetRegistry, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let Some((token, id)) = parse_asset_path(request.uri().path()) else {
        return status(StatusCode::BAD_REQUEST);
    };
    let Some(file) = registry.lookup(token, id) else {
        tracing::warn!(token, id, "asset request for unknown document or image");
        return status(StatusCode::NOT_FOUND);
    };
    match std::fs::read(&file) {
        Ok(bytes) => Response::builder()
            .header(header::CONTENT_TYPE, mime_for(&file))
            // SVGs could contain script; this keeps them inert even if opened directly.
            .header(
                header::CONTENT_SECURITY_POLICY,
                "default-src 'none'; style-src 'unsafe-inline'",
            )
            .body(bytes)
            .unwrap_or_else(|_| status(StatusCode::INTERNAL_SERVER_ERROR)),
        Err(e) => {
            tracing::warn!(path = %file.display(), error = %e, "could not read asset");
            status(StatusCode::NOT_FOUND)
        }
    }
}

fn status(code: StatusCode) -> Response<Vec<u8>> {
    let mut response = Response::new(Vec::new());
    *response.status_mut() = code;
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_serves_only_the_current_document() {
        let registry = AssetRegistry::default();
        let first = registry.register("main", vec![PathBuf::from("a.png")]);
        assert_eq!(registry.lookup(first, 0), Some(PathBuf::from("a.png")));
        assert_eq!(registry.lookup(first, 1), None);

        // Opening another document in the same window revokes the first.
        let second = registry.register("main", vec![PathBuf::from("b.png")]);
        assert_ne!(first, second);
        assert_eq!(registry.lookup(first, 0), None);
        assert_eq!(registry.lookup(second, 0), Some(PathBuf::from("b.png")));

        registry.remove_window("main");
        assert_eq!(registry.lookup(second, 0), None);
    }

    #[test]
    fn parses_asset_paths() {
        assert_eq!(parse_asset_path("/12-3"), Some((12, 3)));
        assert_eq!(parse_asset_path("12-3"), Some((12, 3)));
        assert_eq!(parse_asset_path("/../etc/passwd"), None);
        assert_eq!(parse_asset_path("/12"), None);
        assert_eq!(parse_asset_path("/a-b"), None);
    }

    #[test]
    fn mime_types() {
        assert_eq!(mime_for(Path::new("x.PNG")), "image/png");
        assert_eq!(mime_for(Path::new("x.jpeg")), "image/jpeg");
        assert_eq!(mime_for(Path::new("x")), "application/octet-stream");
    }

    #[test]
    fn unknown_requests_are_rejected() {
        let registry = AssetRegistry::default();
        let request = Request::builder()
            .uri("http://scrald-asset.localhost/1-0")
            .body(Vec::new())
            .unwrap();
        assert_eq!(respond(&registry, &request).status(), StatusCode::NOT_FOUND);
        let bad = Request::builder()
            .uri("http://scrald-asset.localhost/nope")
            .body(Vec::new())
            .unwrap();
        assert_eq!(respond(&registry, &bad).status(), StatusCode::BAD_REQUEST);
    }
}
