//! Rendering results kept from one parse of a document to the next, so that
//! after an edit only the changed blocks pay for the slow steps (DESIGN.md
//! §9.1). Sanitizing and syntax highlighting take about 70% of a parse, and
//! both depend only on their input text, so a result can be reused whenever
//! the input is exactly the same.

use std::collections::HashMap;

/// Pass the same cache to every parse of a document (see
/// `parse_document_cached`). It keeps only what the last parse used, so it
/// stays about the size of the document's HTML.
#[derive(Debug, Default)]
pub struct RenderCache {
    /// Whether the sanitized HTML was made with remote images allowed; the
    /// sanitizer keeps or drops remote `src` depending on it.
    allow_remote_images: bool,
    sanitized: Memo<String>,
    highlighted: Memo<Option<String>>,
}

impl RenderCache {
    /// Gets ready for a parse with these settings: results made with other
    /// settings are dropped.
    pub(crate) fn start(&mut self, allow_remote_images: bool) {
        if self.allow_remote_images != allow_remote_images {
            *self = RenderCache {
                allow_remote_images,
                ..RenderCache::default()
            };
        }
    }

    /// Sanitized HTML for `html`, made with `sanitize` if it isn't cached.
    pub(crate) fn sanitized(&mut self, html: &str, sanitize: impl FnOnce() -> String) -> String {
        self.sanitized.get_or_make(html, sanitize)
    }

    /// Highlighted code for `code` in `language` (`None` if the language
    /// isn't known), made with `highlight` if it isn't cached.
    pub(crate) fn highlighted(
        &mut self,
        code: &str,
        language: &str,
        highlight: impl FnOnce() -> Option<String>,
    ) -> Option<String> {
        // A language name never contains a line break, so this key can't
        // come from two different (language, code) pairs.
        let key = format!("{language}\n{code}");
        self.highlighted.get_or_make(&key, highlight)
    }

    /// Ends a parse: anything it didn't use is dropped.
    pub(crate) fn finish(&mut self) {
        self.sanitized.finish();
        self.highlighted.finish();
    }

    /// How many results are kept (for tests).
    pub fn len(&self) -> usize {
        self.sanitized.previous.len() + self.highlighted.previous.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Results keyed by their exact input. `previous` holds what the last parse
/// used; `current` collects what this parse uses, and replaces `previous`
/// when it ends.
// Rust note: `Memo<V>` is generic over the value type, like a C++ class
// template: `Memo<String>` and `Memo<Option<String>>` are separate types
// generated from this one definition.
#[derive(Debug)]
struct Memo<V> {
    previous: HashMap<String, V>,
    current: HashMap<String, V>,
}

// Rust note: `#[derive(Default)]` on a generic struct would require `V:
// Default`; writing it by hand avoids that (empty maps need nothing from V).
impl<V> Default for Memo<V> {
    fn default() -> Self {
        Memo {
            previous: HashMap::new(),
            current: HashMap::new(),
        }
    }
}

// `V: Clone` says the value type must be copyable with `.clone()`: a result
// is handed out and kept.
impl<V: Clone> Memo<V> {
    fn get_or_make(&mut self, key: &str, make: impl FnOnce() -> V) -> V {
        if let Some(value) = self.current.get(key) {
            return value.clone();
        }
        let value = match self.previous.remove(key) {
            Some(value) => value,
            None => make(),
        };
        self.current.insert(key.to_string(), value.clone());
        value
    }

    fn finish(&mut self) {
        // Rust note: `mem::take` moves the map out and leaves an empty one
        // in its place, so nothing is copied.
        self.previous = std::mem::take(&mut self.current);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reuses_results_and_drops_unused_ones() {
        let mut cache = RenderCache::default();
        cache.start(false);
        assert_eq!(cache.sanitized("<p>a</p>", || "A".into()), "A");
        assert_eq!(cache.sanitized("<p>b</p>", || "B".into()), "B");
        cache.finish();
        assert_eq!(cache.len(), 2);

        // The next parse reuses `a` (the closure isn't called) and drops `b`.
        cache.start(false);
        let reused = cache.sanitized("<p>a</p>", || panic!("should be cached"));
        assert_eq!(reused, "A");
        cache.finish();
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn other_settings_start_empty() {
        let mut cache = RenderCache::default();
        cache.start(false);
        cache.sanitized("<img>", || "kept".into());
        cache.finish();
        cache.start(true);
        assert_eq!(cache.sanitized("<img>", || "new".into()), "new");
    }

    #[test]
    fn highlight_key_includes_the_language() {
        let mut cache = RenderCache::default();
        cache.start(false);
        assert_eq!(
            cache.highlighted("x", "rust", || Some("R".into())),
            Some("R".into())
        );
        assert_eq!(cache.highlighted("x", "python", || None), None);
        assert_eq!(
            cache.highlighted("x", "rust", || panic!("cached")),
            Some("R".into())
        );
    }
}
