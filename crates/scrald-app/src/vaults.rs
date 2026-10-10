//! A cache of Obsidian vault indexes (DESIGN.md §6.1), so opening, editing,
//! and re-parsing notes doesn't rescan the whole vault each time. An index
//! is reused for `MAX_AGE`, so files added to the vault are picked up within
//! that time without watching every folder in it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use scrald_core::obsidian::VaultIndex;

/// How long an index is reused before the vault is scanned again.
const MAX_AGE: Duration = Duration::from_secs(30);

/// Each vault root's index, and when it was built.
type Indexes = HashMap<PathBuf, (Arc<VaultIndex>, Instant)>;

/// Cheap to clone: clones share one cache, so a background parse can hold
/// its own handle.
// Rust note: `Arc<Mutex<...>>` is shared ownership plus a lock, like a
// `std::shared_ptr<std::mutex-guarded map>`; cloning the Arc shares the map.
#[derive(Clone, Default)]
pub struct VaultCache {
    inner: Arc<Mutex<Indexes>>,
}

impl VaultCache {
    /// The index for a document inside an Obsidian vault: cached if recent,
    /// otherwise scanned now (call from a background thread). `None` outside
    /// a vault, where core indexes just the document's folder itself.
    pub fn for_document(&self, doc_path: &Path) -> Option<Arc<VaultIndex>> {
        let (root, in_vault) = VaultIndex::scope_for(doc_path);
        if !in_vault {
            return None;
        }
        if let Some((index, built)) = self.lock().get(&root)
            && built.elapsed() < MAX_AGE
        {
            return Some(Arc::clone(index));
        }
        // Scan without holding the lock, so other windows aren't blocked.
        let started = Instant::now();
        let index = Arc::new(VaultIndex::build(&root, true));
        tracing::debug!(
            root = %root.display(),
            elapsed_ms = started.elapsed().as_millis() as u64,
            "indexed vault"
        );
        self.lock()
            .insert(root, (Arc::clone(&index), Instant::now()));
        Some(index)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Indexes> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_indexes_are_reused_and_folders_outside_vaults_skipped() {
        let root = std::env::temp_dir().join(format!("scrald-vault-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".obsidian")).unwrap();
        std::fs::create_dir_all(root.join("notes")).unwrap();
        std::fs::write(root.join("notes").join("a.md"), "x").unwrap();

        let cache = VaultCache::default();
        let first = cache
            .for_document(&root.join("notes").join("a.md"))
            .unwrap();
        let second = cache.for_document(&root.join("b.md")).unwrap();
        assert!(
            Arc::ptr_eq(&first, &second),
            "same vault, same cached index"
        );

        let outside = std::env::temp_dir()
            .join("scrald-no-vault-here")
            .join("c.md");
        assert!(cache.for_document(&outside).is_none());
    }
}
