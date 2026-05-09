//! Per-domain Tantivy fulltext index shards.
//!
//! Each domain gets its own Tantivy index under `.rsut_index/tantivy/<domain>/`,
//! enabling domain-granular lazy loading and idle reclamation.
//!
//! The `FulltextShardManager` routes `rebuild()` and `search()` across all
//! active (warm) shards. Cold shards are opened on demand via `warm_shard()`
//! and dropped via `cool_shard()`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tracing::{debug, info};

use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::index::fulltext::{FulltextIndex, SearchHit};

/// A warm (loaded) fulltext shard with its last-access timestamp.
struct WarmShard {
    index: FulltextIndex,
    last_access: Instant,
}

/// Manages per-domain Tantivy index shards with lazy loading.
///
/// Each domain is stored in a separate Tantivy index under
/// `<knowledge_base>/.rsut_index/tantivy/<domain>/`.
pub struct FulltextShardManager {
    knowledge_base: PathBuf,
    /// Warm shards currently in memory (keyed by domain).
    warm: HashMap<String, WarmShard>,
    /// Maximum number of shards to keep warm simultaneously.
    max_warm: usize,
    /// How long a shard can be idle before cooling.
    idle_ttl: Duration,
}

impl FulltextShardManager {
    /// Create a new shard manager for the given knowledge base.
    ///
    /// Does **not** open any shards — call `warm_shard()` on first access.
    pub fn new(knowledge_base: &Path) -> Self {
        Self {
            knowledge_base: knowledge_base.to_path_buf(),
            warm: HashMap::new(),
            max_warm: 5,
            idle_ttl: Duration::from_secs(300),
        }
    }

    /// Set warm shard limits.
    pub fn with_limits(mut self, max_warm: usize, idle_ttl: Duration) -> Self {
        self.max_warm = max_warm;
        self.idle_ttl = idle_ttl;
        self
    }

    /// Warm (load) a domain shard, returning a reference to the index.
    ///
    /// Creates a new Tantivy index for the domain if one doesn't exist.
    pub fn warm_shard(&mut self, domain: &str) -> PdfResult<()> {
        if self.warm.contains_key(domain) {
            if let Some(shard) = self.warm.get_mut(domain) {
                shard.last_access = Instant::now();
            }
            return Ok(());
        }

        // Evict coldest if at capacity
        self.evict_if_needed();

        let domain_index_dir = self
            .knowledge_base
            .join(".rsut_index")
            .join("tantivy")
            .join(domain);

        std::fs::create_dir_all(&domain_index_dir).map_err(|e| {
            PdfModuleError::Storage(format!(
                "Failed to create tantivy shard dir for '{}': {}",
                domain, e
            ))
        })?;

        // Use the knowledge_base-relative open, but with domain-specific path
        // FulltextIndex::open_or_create uses kb_path/.rsut_index/tantivy/ which is wrong for shards.
        // Instead we open at the domain-specific path by tricking the knowledge_base parameter.
        let index = FulltextIndex::open_at(&domain_index_dir)?;

        self.warm.insert(
            domain.to_string(),
            WarmShard {
                index,
                last_access: Instant::now(),
            },
        );

        debug!(domain = %domain, "Fulltext shard warmed");
        Ok(())
    }

    /// Cool (unload) a domain shard, releasing its memory.
    pub fn cool_shard(&mut self, domain: &str) {
        if self.warm.remove(domain).is_some() {
            debug!(domain = %domain, "Fulltext shard cooled");
        }
    }

    /// Check if a domain shard is currently warm.
    pub fn is_warm(&self, domain: &str) -> bool {
        self.warm.contains_key(domain)
    }

    /// List all warm domains.
    pub fn warm_domains(&self) -> Vec<String> {
        self.warm.keys().cloned().collect()
    }

    /// Number of warm shards.
    pub fn warm_count(&self) -> usize {
        self.warm.len()
    }

    /// Get the last access time for a warm shard.
    pub fn last_access(&self, domain: &str) -> Option<Instant> {
        self.warm.get(domain).map(|s| s.last_access)
    }

    /// Rebuild all shards from the wiki directory.
    ///
    /// Scans the wiki directory, discovers domains from subdirectories,
    /// and rebuilds each domain's Tantivy index from its Markdown files.
    pub fn rebuild_all(&mut self, wiki_dir: &Path) -> PdfResult<usize> {
        let domains = discover_domains(wiki_dir);
        let mut total = 0usize;

        for domain in &domains {
            self.warm_shard(domain)?;
            let domain_wiki = wiki_dir.join(domain);
            if domain_wiki.exists() {
                if let Some(shard) = self.warm.get(domain) {
                    let count = shard.index.rebuild(&domain_wiki)?;
                    total += count;
                    debug!(domain = %domain, count = count, "Shard rebuilt");
                }
            }
        }

        info!(
            domains = domains.len(),
            total = total,
            "All fulltext shards rebuilt"
        );
        Ok(total)
    }

    /// Search across all warm shards. Returns combined results sorted by score.
    pub fn search(&mut self, query: &str, limit: usize) -> PdfResult<Vec<SearchHit>> {
        // First, ensure at least some shards are warm. If none, discover and warm all.
        if self.warm.is_empty() {
            let wiki_dir = self.knowledge_base.join("wiki");
            if wiki_dir.exists() {
                let domains = discover_domains(&wiki_dir);
                for domain in &domains {
                    self.warm_shard(domain)?;
                }
            }
        }

        let mut all_hits: Vec<SearchHit> = Vec::new();

        let domain_list: Vec<String> = self.warm.keys().cloned().collect();
        for domain in &domain_list {
            if let Some(shard) = self.warm.get_mut(domain) {
                shard.last_access = Instant::now();
                match shard.index.search(query, limit) {
                    Ok(hits) => all_hits.extend(hits),
                    Err(e) => {
                        debug!(domain = %domain, error = %e, "Search failed on shard, skipping");
                    }
                }
            }
        }

        // Sort by score descending
        all_hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all_hits.truncate(limit);

        Ok(all_hits)
    }

    /// Get a mutable reference to a warm shard's index for targeted operations.
    pub fn with_shard<F, R>(&mut self, domain: &str, f: F) -> PdfResult<R>
    where
        F: FnOnce(&mut FulltextIndex) -> PdfResult<R>,
    {
        self.warm_shard(domain)?;
        if let Some(shard) = self.warm.get_mut(domain) {
            shard.last_access = Instant::now();
            f(&mut shard.index)
        } else {
            Err(PdfModuleError::Storage(format!(
                "Shard '{}' not available after warm",
                domain
            )))
        }
    }

    /// List all domain shards (both warm and cold) by scanning the tantivy directory.
    pub fn all_shards_on_disk(&self) -> PdfResult<Vec<String>> {
        let tantivy_dir = self.knowledge_base.join(".rsut_index").join("tantivy");
        if !tantivy_dir.exists() {
            return Ok(Vec::new());
        }

        let mut domains = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&tantivy_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name() {
                        let name = name.to_string_lossy().to_string();
                        if !name.starts_with('.') && path.join("meta.json").exists() {
                            domains.push(name);
                        }
                    }
                }
            }
        }
        Ok(domains)
    }

    // ── Private ──

    fn evict_if_needed(&mut self) {
        while self.warm.len() >= self.max_warm {
            let coldest = self
                .warm
                .iter()
                .min_by_key(|(_, s)| s.last_access)
                .map(|(d, _)| d.clone());

            if let Some(domain) = coldest {
                self.warm.remove(&domain);
                debug!(domain = %domain, "Evicted coldest fulltext shard");
            } else {
                break;
            }
        }
    }
}

/// Discover domain directories under the wiki root.
fn discover_domains(wiki_dir: &Path) -> Vec<String> {
    let mut domains = Vec::new();
    if !wiki_dir.exists() {
        return domains;
    }
    if let Ok(entries) = std::fs::read_dir(wiki_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    let name = name.to_string_lossy().to_string();
                    if !name.starts_with('.') && name != ".versions" {
                        domains.push(name);
                    }
                }
            }
        }
    }
    domains.sort();
    domains
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_wiki(dir: &Path, domain: &str) {
        let domain_dir = dir.join("wiki").join(domain);
        std::fs::create_dir_all(&domain_dir).unwrap();

        let md = format!(
            r#"---
title: "Test Entry"
domain: "{}"
tags: ["test"]
level: l1
status: compiled
created: 2026-01-01T00:00:00Z
updated: 2026-01-01T00:00:00Z
---
# Test Entry
Content here."#,
            domain
        );

        std::fs::write(domain_dir.join("test_entry.md"), md).unwrap();
    }

    #[test]
    fn test_warm_and_cool_shard() {
        let dir = TempDir::new().unwrap();
        create_test_wiki(dir.path(), "it");

        let mut mgr = FulltextShardManager::new(dir.path());
        assert!(!mgr.is_warm("it"));

        mgr.warm_shard("it").unwrap();
        assert!(mgr.is_warm("it"));

        mgr.cool_shard("it");
        assert!(!mgr.is_warm("it"));
    }

    #[test]
    fn test_rebuild_and_search() {
        let dir = TempDir::new().unwrap();
        create_test_wiki(dir.path(), "rust");

        let mut mgr = FulltextShardManager::new(dir.path());
        mgr.warm_shard("rust").unwrap();
        mgr.with_shard("rust", |idx| {
            idx.rebuild(&dir.path().join("wiki").join("rust"))
        })
        .unwrap();

        let results = mgr.search("test", 10).unwrap();
        assert!(!results.is_empty(), "Should find at least one result");
    }
}
