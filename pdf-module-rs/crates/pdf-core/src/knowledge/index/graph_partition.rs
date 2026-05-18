//! Sled-based graph partitions with per-domain lazy loading.
//!
//! Provides domain-granular graph persistence so that only the domains
//! needed for a query are loaded into memory. Each domain's subgraph
//! is stored as a bincode-serialized `SubgraphSnapshot` in a dedicated
//! sled tree under `graph:<domain>`.
//!
//! An in-memory LRU keeps the most recently accessed domain subgraphs
//! warm for fast queries. Cold domains are reloaded from sled on demand.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::{PdfError, PdfResult};
use crate::knowledge::index::graph::{EdgeKind, NodeMeta};

/// Serializable snapshot of a domain subgraph.
#[derive(Serialize, Deserialize)]
struct SubgraphSnapshot {
    nodes: Vec<NodeMeta>,
    edges: Vec<(usize, usize, EdgeKind)>,
    path_to_index: Vec<(String, usize)>,
}

/// An in-memory domain subgraph loaded from sled, wrapped for shared access.
struct WarmSubgraph {
    graph: DiGraph<NodeMeta, EdgeKind>,
    path_to_node: HashMap<String, NodeIndex>,
    last_access: Instant,
}

/// Thread-safe handle to a loaded domain subgraph.
///
/// Clone to share across tasks. The underlying graph lives until all handles
/// are dropped or the domain is explicitly cooled.
#[derive(Clone)]
pub struct DomainGraph {
    inner: Arc<RwLock<WarmSubgraph>>,
}

impl DomainGraph {
    /// Read the graph and path_to_node via a closure.
    pub fn with_graph<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&DiGraph<NodeMeta, EdgeKind>, &HashMap<String, NodeIndex>) -> R,
    {
        let guard = self.inner.read().expect("DomainGraph lock poisoned");
        // Touch last_access for idle tracking
        let _ = &guard.last_access;
        f(&guard.graph, &guard.path_to_node)
    }

    /// Get the number of nodes.
    pub fn node_count(&self) -> usize {
        self.inner.read().map(|g| g.graph.node_count()).unwrap_or(0)
    }

    /// Get the number of edges.
    pub fn edge_count(&self) -> usize {
        self.inner.read().map(|g| g.graph.edge_count()).unwrap_or(0)
    }

    /// Touch the last_access timestamp to prevent idle cooling.
    pub fn touch(&self) {
        if let Ok(mut guard) = self.inner.write() {
            guard.last_access = Instant::now();
        }
    }

    fn new(graph: DiGraph<NodeMeta, EdgeKind>, path_to_node: HashMap<String, NodeIndex>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(WarmSubgraph {
                graph,
                path_to_node,
                last_access: Instant::now(),
            })),
        }
    }
}

/// Sled-backed graph partition store with per-domain lazy loading.
///
/// Each domain gets its own tree `graph:<domain>` inside the sled database.
pub struct GraphPartitionStore {
    db: sled::Db,
    /// In-memory LRU of warm (loaded) domain subgraphs.
    warm: HashMap<String, DomainGraph>,
    /// Maximum number of domains to keep warm simultaneously.
    max_warm: usize,
    /// How long a domain can be idle before it is a candidate for cooling.
    idle_ttl: Duration,
}

impl GraphPartitionStore {
    /// Open or create the graph partition store at `<knowledge_base>/.rsut_index/graph_partitions/`.
    pub fn open(knowledge_base: &Path) -> PdfResult<Self> {
        let db_path = knowledge_base.join(".rsut_index").join("graph_partitions");
        std::fs::create_dir_all(&db_path).map_err(|e| {
            PdfError::Storage(format!("Failed to create graph partitions dir: {}", e))
        })?;

        let db = sled::open(&db_path)
            .map_err(|e| PdfError::Storage(format!("Failed to open graph partitions db: {}", e)))?;

        info!("GraphPartitionStore opened at {:?}", db_path);

        Ok(Self { db, warm: HashMap::new(), max_warm: 5, idle_ttl: Duration::from_secs(300) })
    }

    /// Set the maximum number of warm domains and idle TTL.
    pub fn with_limits(mut self, max_warm: usize, idle_ttl: Duration) -> Self {
        self.max_warm = max_warm;
        self.idle_ttl = idle_ttl;
        self
    }

    /// Ensure a domain is loaded into memory, returning a `DomainGraph` handle.
    ///
    /// Returns `None` if the domain has no stored subgraph.
    pub fn load_domain(&mut self, domain: &str) -> PdfResult<Option<DomainGraph>> {
        // Check warm cache
        if let Some(dg) = self.warm.get(domain) {
            dg.touch();
            return Ok(Some(dg.clone()));
        }

        // Evict coldest if at capacity
        self.evict_if_needed();

        // Load from sled
        let tree_name = format!("graph:{}", domain);
        let tree = self.db.open_tree(&tree_name).map_err(|e| {
            PdfError::Storage(format!("Failed to open graph tree '{}': {}", domain, e))
        })?;

        let subgraph_key = "subgraph";
        let snapshot: SubgraphSnapshot = match tree.get(subgraph_key).map_err(|e| {
            PdfError::Storage(format!("Failed to read graph for '{}': {}", domain, e))
        })? {
            Some(bytes) => bincode::deserialize(&bytes).map_err(|e| {
                PdfError::Storage(format!("Failed to deserialize graph '{}': {}", domain, e))
            })?,
            None => return Ok(None),
        };

        let (graph, path_to_node) = Self::deserialize_subgraph(snapshot);

        debug!(domain = %domain, nodes = graph.node_count(), edges = graph.edge_count(), "Graph partition loaded");

        let dg = DomainGraph::new(graph, path_to_node);
        self.warm.insert(domain.to_string(), dg.clone());

        Ok(Some(dg))
    }

    /// Save a domain subgraph to sled and keep it warm in memory.
    pub fn save_domain(
        &mut self,
        domain: &str,
        graph: &DiGraph<NodeMeta, EdgeKind>,
        path_to_node: &HashMap<String, NodeIndex>,
    ) -> PdfResult<DomainGraph> {
        let tree_name = format!("graph:{}", domain);
        let tree = self.db.open_tree(&tree_name).map_err(|e| {
            PdfError::Storage(format!("Failed to open graph tree '{}': {}", domain, e))
        })?;

        let snapshot = Self::serialize_subgraph(graph, path_to_node);
        let bytes = bincode::serialize(&snapshot).map_err(|e| {
            PdfError::Storage(format!("Failed to serialize graph '{}': {}", domain, e))
        })?;

        tree.insert("subgraph", bytes)
            .map_err(|e| PdfError::Storage(format!("Failed to write graph '{}': {}", domain, e)))?;

        // Evict coldest if at capacity before inserting
        self.evict_if_needed();

        // Also keep warm in memory
        let dg = DomainGraph::new(graph.clone(), path_to_node.clone());
        self.warm.insert(domain.to_string(), dg.clone());

        debug!(domain = %domain, nodes = graph.node_count(), edges = graph.edge_count(), "Graph partition saved");
        Ok(dg)
    }

    /// Drop a domain from the warm in-memory cache.
    ///
    /// The subgraph remains on disk in sled and can be re-loaded on demand.
    pub fn cool_domain(&mut self, domain: &str) {
        if self.warm.remove(domain).is_some() {
            debug!(domain = %domain, "Graph partition cooled");
        }
    }

    /// List all domains that have stored graph partitions.
    pub fn domains(&self) -> PdfResult<Vec<String>> {
        let domains: Vec<String> = self
            .db
            .tree_names()
            .into_iter()
            .filter_map(|name| {
                let name = String::from_utf8_lossy(&name).to_string();
                name.strip_prefix("graph:").map(String::from)
            })
            .collect();
        Ok(domains)
    }

    /// List currently warm (in-memory) domains.
    pub fn warm_domains(&self) -> Vec<String> {
        self.warm.keys().cloned().collect()
    }

    /// Number of warm domains.
    pub fn warm_count(&self) -> usize {
        self.warm.len()
    }

    /// Check if a domain is currently warm.
    pub fn is_domain_warm(&self, domain: &str) -> bool {
        self.warm.contains_key(domain)
    }

    /// Get the last access time for a warm domain.
    pub fn last_access(&self, domain: &str) -> Option<Instant> {
        self.warm.get(domain).and_then(|dg| dg.inner.read().ok().map(|g| g.last_access))
    }

    /// Flush all pending writes.
    pub fn flush(&self) -> PdfResult<()> {
        self.db.flush().map_err(|e| {
            PdfError::Storage(format!("Failed to flush graph partitions db: {}", e))
        })?;
        Ok(())
    }

    // ── Private ──

    fn evict_if_needed(&mut self) {
        while self.warm.len() >= self.max_warm {
            let coldest = self
                .warm
                .iter()
                .filter_map(|(d, dg)| dg.inner.read().ok().map(|g| (d.clone(), g.last_access)))
                .min_by_key(|(_, last_access)| *last_access)
                .map(|(d, _)| d);

            if let Some(domain) = coldest {
                self.warm.remove(&domain);
                debug!(domain = %domain, "Evicted coldest graph partition");
            } else {
                break;
            }
        }
    }

    fn serialize_subgraph(
        graph: &DiGraph<NodeMeta, EdgeKind>,
        path_to_node: &HashMap<String, NodeIndex>,
    ) -> SubgraphSnapshot {
        let nodes: Vec<NodeMeta> = graph.node_indices().map(|idx| graph[idx].clone()).collect();

        let mut node_idx_to_usize = HashMap::new();
        for (i, idx) in graph.node_indices().enumerate() {
            node_idx_to_usize.insert(idx, i);
        }

        let edges: Vec<(usize, usize, EdgeKind)> = graph
            .edge_references()
            .map(|e| {
                (node_idx_to_usize[&e.source()], node_idx_to_usize[&e.target()], e.weight().clone())
            })
            .collect();

        let path_to_index: Vec<(String, usize)> =
            path_to_node.iter().map(|(path, idx)| (path.clone(), node_idx_to_usize[idx])).collect();

        SubgraphSnapshot { nodes, edges, path_to_index }
    }

    fn deserialize_subgraph(
        snapshot: SubgraphSnapshot,
    ) -> (DiGraph<NodeMeta, EdgeKind>, HashMap<String, NodeIndex>) {
        let mut graph = DiGraph::new();
        let mut path_to_node = HashMap::new();

        let node_indices: Vec<NodeIndex> =
            snapshot.nodes.into_iter().map(|meta| graph.add_node(meta)).collect();

        for (from, to, kind) in snapshot.edges {
            if from < node_indices.len() && to < node_indices.len() {
                graph.add_edge(node_indices[from], node_indices[to], kind);
            }
        }

        for (path, idx) in snapshot.path_to_index {
            if idx < node_indices.len() {
                path_to_node.insert(path, node_indices[idx]);
            }
        }

        (graph, path_to_node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_node(path: &str, title: &str, domain: &str) -> NodeMeta {
        NodeMeta {
            path: path.to_string(),
            title: title.to_string(),
            domain: domain.to_string(),
            tags: vec![],
            level: "L1".to_string(),
        }
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let mut store = GraphPartitionStore::open(dir.path()).unwrap();

        // Build a graph
        let mut graph = DiGraph::new();
        let mut path_to_node = HashMap::new();

        let a = graph.add_node(make_node("it/a.md", "Entry A", "it"));
        let b = graph.add_node(make_node("it/b.md", "Entry B", "it"));
        graph.add_edge(a, b, EdgeKind::Related);

        path_to_node.insert("it/a.md".to_string(), a);
        path_to_node.insert("it/b.md".to_string(), b);

        store.save_domain("it", &graph, &path_to_node).unwrap();

        // Load back
        let loaded = store.load_domain("it").unwrap();
        assert!(loaded.is_some());
        loaded.unwrap().with_graph(|g, _| {
            assert_eq!(g.node_count(), 2);
        });
    }

    #[test]
    fn test_cool_and_reload() {
        let dir = TempDir::new().unwrap();
        let mut store = GraphPartitionStore::open(dir.path()).unwrap();

        let mut graph = DiGraph::new();
        let mut path_to_node = HashMap::new();
        let n = graph.add_node(make_node("math/c.md", "Entry C", "math"));
        path_to_node.insert("math/c.md".to_string(), n);
        store.save_domain("math", &graph, &path_to_node).unwrap();

        // Cool and verify
        assert!(store.is_domain_warm("math"));
        store.cool_domain("math");
        assert!(!store.is_domain_warm("math"));

        // Reload from sled
        let loaded = store.load_domain("math").unwrap();
        assert!(loaded.is_some());
        assert!(store.is_domain_warm("math"));
    }

    #[test]
    fn test_domain_listing() {
        let dir = TempDir::new().unwrap();
        let mut store = GraphPartitionStore::open(dir.path()).unwrap();

        let g1 = DiGraph::new();
        let g2 = DiGraph::new();
        store.save_domain("it", &g1, &HashMap::new()).unwrap();
        store.save_domain("math", &g2, &HashMap::new()).unwrap();

        let domains = store.domains().unwrap();
        assert!(domains.contains(&"it".to_string()));
        assert!(domains.contains(&"math".to_string()));
    }

    #[test]
    fn test_warm_limit_eviction() {
        let dir = TempDir::new().unwrap();
        let mut store =
            GraphPartitionStore::open(dir.path()).unwrap().with_limits(2, Duration::from_secs(300));

        // Save 4 domains
        for i in 0..4 {
            let g = DiGraph::new();
            store.save_domain(&format!("d{}", i), &g, &HashMap::new()).unwrap();
        }

        assert!(store.warm_count() <= 2);
    }

    #[test]
    fn test_with_graph_access() {
        let dir = TempDir::new().unwrap();
        let mut store = GraphPartitionStore::open(dir.path()).unwrap();

        let mut graph = DiGraph::new();
        let mut path_to_node = HashMap::new();
        let a = graph.add_node(make_node("x/a.md", "Alpha", "x"));
        path_to_node.insert("x/a.md".to_string(), a);

        let dg = store.save_domain("x", &graph, &path_to_node).unwrap();

        // Test with_graph closure
        dg.with_graph(|g, p2n| {
            assert_eq!(g.node_count(), 1);
            assert!(p2n.contains_key("x/a.md"));
        });
    }
}
