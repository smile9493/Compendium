//! Lazy loading coordinator for domain-sharded indexes.
//!
//! Orchestrates `MetadataStore`, `GraphPartitionStore`, and
//! `FulltextShardManager` to lazily warm domain shards on first access.
//!
//! Tracks last-access timestamps for idle reclamation and exposes
//! `ensure_warm(domain)` to be called before any search or query.

use std::path::Path;
use std::time::Instant;

use tracing::debug;

use crate::error::PdfResult;
use crate::knowledge::index::fulltext_shard::FulltextShardManager;
use crate::knowledge::index::graph_partition::GraphPartitionStore;
use crate::knowledge::index::MetadataStore;

/// Coordinates lazy loading across the three sharded stores.
pub struct LazyLoadingCoordinator {
    metadata: MetadataStore,
    graph: GraphPartitionStore,
    fulltext: FulltextShardManager,
    /// Last-access timestamps per domain (for idle reclamation).
    last_access: std::collections::HashMap<String, Instant>,
}

impl LazyLoadingCoordinator {
    /// Open all three stores for the given knowledge base.
    pub fn open(knowledge_base: &Path) -> PdfResult<Self> {
        let metadata = MetadataStore::open(knowledge_base)?;
        let graph = GraphPartitionStore::open(knowledge_base)?;
        let fulltext = FulltextShardManager::new(knowledge_base);

        Ok(Self {
            metadata,
            graph,
            fulltext,
            last_access: std::collections::HashMap::new(),
        })
    }

    /// Ensure a domain is warm in all three stores.
    ///
    /// Call before any search/query to guarantee the domain's shards
    /// are loaded into memory.
    pub fn ensure_warm(&mut self, domain: &str) -> PdfResult<()> {
        // Warm graph partition
        self.graph.load_domain(domain)?;

        // Warm fulltext shard
        self.fulltext.warm_shard(domain)?;

        // Record access time
        self.last_access.insert(domain.to_string(), Instant::now());

        debug!(domain = %domain, "Domain ensured warm");
        Ok(())
    }

    /// Cool a domain in all three stores.
    pub fn cool_domain(&mut self, domain: &str) {
        self.graph.cool_domain(domain);
        self.fulltext.cool_shard(domain);
        self.last_access.remove(domain);
        debug!(domain = %domain, "Domain cooled");
    }

    /// Get last-access time for a domain.
    pub fn last_access(&self, domain: &str) -> Option<Instant> {
        self.last_access.get(domain).copied()
    }

    /// List all domains known to the metadata store.
    pub fn all_domains(&self) -> PdfResult<Vec<String>> {
        self.metadata.all_domains()
    }

    /// List currently warm domains.
    pub fn warm_domains(&self) -> Vec<String> {
        let mut domains: Vec<String> = self.last_access.keys().cloned().collect();
        domains.sort();
        domains
    }

    /// Number of warm domains.
    pub fn warm_count(&self) -> usize {
        self.last_access.len()
    }

    /// Get a mutable reference to the metadata store.
    pub fn metadata(&self) -> &MetadataStore {
        &self.metadata
    }

    /// Get a mutable reference to the graph partition store.
    pub fn graph_mut(&mut self) -> &mut GraphPartitionStore {
        &mut self.graph
    }

    /// Get a mutable reference to the fulltext shard manager.
    pub fn fulltext_mut(&mut self) -> &mut FulltextShardManager {
        &mut self.fulltext
    }

    /// Get an immutable reference to the graph partition store.
    pub fn graph(&self) -> &GraphPartitionStore {
        &self.graph
    }

    /// Get an immutable reference to the fulltext shard manager.
    pub fn fulltext(&self) -> &FulltextShardManager {
        &self.fulltext
    }

    /// Flush all stores.
    pub fn flush(&self) -> PdfResult<()> {
        self.metadata.flush()?;
        self.graph.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_open_and_warm() {
        let dir = TempDir::new().unwrap();
        let mut coord = LazyLoadingCoordinator::open(dir.path()).unwrap();

        // Initially cold
        assert_eq!(coord.warm_count(), 0);

        // Warm a domain
        coord.ensure_warm("it").unwrap();

        // Now warm
        assert_eq!(coord.warm_count(), 1);
        assert!(coord.warm_domains().contains(&"it".to_string()));

        // Cool it
        coord.cool_domain("it");
        assert_eq!(coord.warm_count(), 0);
    }
}
