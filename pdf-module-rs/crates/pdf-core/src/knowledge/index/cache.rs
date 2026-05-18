//! In-process cache for per–knowledge-base fulltext and graph indexes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::RwLock;

use crate::error::PdfResult;
use crate::knowledge::index::{
    graph, search_with_options_ft, wiki_dir, FulltextIndex, GraphIndex, SearchMode, SearchOptions,
    SearchResponse,
};

/// Loaded indexes for one knowledge base.
pub struct KbIndexes {
    pub fulltext: FulltextIndex,
    pub graph: GraphIndex,
    generation: u64,
}

/// Process-wide cache keyed by canonical knowledge-base path.
pub struct IndexCache {
    inner: RwLock<HashMap<String, Arc<KbIndexes>>>,
    next_generation: AtomicU64,
}

impl Default for IndexCache {
    fn default() -> Self {
        Self::new()
    }
}

impl IndexCache {
    pub fn new() -> Self {
        Self { inner: RwLock::new(HashMap::new()), next_generation: AtomicU64::new(1) }
    }

    fn cache_key(knowledge_base: &Path) -> PathBuf {
        knowledge_base.canonicalize().unwrap_or_else(|_| knowledge_base.to_path_buf())
    }

    pub fn get_or_load(&self, knowledge_base: &Path) -> PdfResult<Arc<KbIndexes>> {
        let key = Self::cache_key(knowledge_base).to_string_lossy().to_string();
        if let Some(entry) = self.inner.read().get(&key) {
            return Ok(Arc::clone(entry));
        }

        let generation = self.next_generation.fetch_add(1, Ordering::Relaxed);
        let wd = wiki_dir(knowledge_base);
        let fulltext = FulltextIndex::open_or_create(knowledge_base)?;
        let graph_idx = if wd.exists() { graph(knowledge_base)? } else { GraphIndex::new() };

        let entry = Arc::new(KbIndexes { fulltext, graph: graph_idx, generation });
        self.inner.write().insert(key, Arc::clone(&entry));
        Ok(entry)
    }

    pub fn invalidate(&self, knowledge_base: &Path) {
        let key = Self::cache_key(knowledge_base).to_string_lossy().to_string();
        self.inner.write().remove(&key);
    }

    pub fn search(
        &self,
        knowledge_base: &Path,
        query: &str,
        limit: usize,
        mode: SearchMode,
        opts: SearchOptions,
    ) -> PdfResult<SearchResponse> {
        let indexes = self.get_or_load(knowledge_base)?;
search_with_options_ft(
            knowledge_base,
            query,
            limit,
            mode,
            opts,
            Some(&indexes.fulltext),
        )
    }

    pub fn graph(&self, knowledge_base: &Path) -> PdfResult<Arc<KbIndexes>> {
        self.get_or_load(knowledge_base)
    }
}

impl KbIndexes {
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::index::{rebuild_all, SearchMode};

    fn write_entry(kb: &Path, domain: &str, name: &str, body: &str) {
        let dir = kb.join("wiki").join(domain);
        std::fs::create_dir_all(&dir).unwrap();
        let content = format!(
            "---\ntitle: \"{name}\"\ndomain: \"{domain}\"\ntags: [test]\nlevel: L1\nstatus: compiled\npublish_status: published\nquality_score: 0.9\ncreated: 2026-01-01\nupdated: 2026-01-01\n---\n\n{body}"
        );
        std::fs::write(dir.join(format!("{name}.md")), content).unwrap();
    }

    #[test]
    fn test_cache_reuses_generation_until_invalidate() {
        let dir = tempfile::tempdir().unwrap();
        let kb = dir.path();
        write_entry(kb, "IT", "cache_test", "unique cache token");
        rebuild_all(kb).unwrap();

        let cache = IndexCache::new();
        let a = cache.get_or_load(kb).unwrap();
        let b = cache.get_or_load(kb).unwrap();
        assert_eq!(a.generation(), b.generation());

        cache.invalidate(kb);
        let c = cache.get_or_load(kb).unwrap();
        assert!(c.generation() > a.generation());
    }

    #[test]
    fn test_cached_search_finds_content() {
        let dir = tempfile::tempdir().unwrap();
        let kb = dir.path();
        write_entry(kb, "IT", "findme", "needle in haystack");
        rebuild_all(kb).unwrap();

        let cache = IndexCache::new();
        let opts = SearchOptions::for_api();
        let resp = cache.search(kb, "needle", 5, SearchMode::Keyword, opts).unwrap();
        assert!(resp.hits.iter().any(|h| h.path.contains("findme")));
    }
}
