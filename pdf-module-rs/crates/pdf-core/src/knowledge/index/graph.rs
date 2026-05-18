//! petgraph-based link graph for knowledge entries.
//!
//! Builds a directed graph from the `related` and `contradictions` fields
//! in entry front matter. Supports:
//! - N-hop neighbor discovery
//! - Orphan detection (no incoming or outgoing edges)
//! - Link suggestion (Jaccard similarity on tags)
//! - Concept map export (Mermaid.js format)
//! - Disk persistence via bincode serialization

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::entry::KnowledgeEntry;

/// Metadata stored at each graph node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMeta {
    pub path: String,
    pub title: String,
    pub domain: String,
    pub tags: Vec<String>,
    pub level: String,
}

/// Edge type in the knowledge graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeKind {
    Related,
    Contradiction,
    TagCooccurrence,
}

/// Result of a neighbor query.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NeighborInfo {
    pub path: String,
    pub title: String,
    pub domain: String,
    pub hops: u32,
    pub edge_kind: String,
}

/// Result of a link suggestion.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LinkSuggestion {
    pub from: String,
    pub to: String,
    pub score: f64,
    pub reason: String,
}

/// Serializable snapshot of the graph for disk persistence.
#[derive(Serialize, Deserialize)]
struct GraphSnapshot {
    nodes: Vec<NodeMeta>,
    edges: Vec<(usize, usize, EdgeKind)>,
    path_to_index: Vec<(String, usize)>,
}

/// Directed graph index over knowledge entries.
pub struct GraphIndex {
    graph: DiGraph<NodeMeta, EdgeKind>,
    /// Maps path → node index for fast lookup.
    path_to_node: HashMap<String, NodeIndex>,
}

impl GraphIndex {
    /// Create an empty graph index.
    pub fn new() -> Self {
        Self { graph: DiGraph::new(), path_to_node: HashMap::new() }
    }

    /// Save the graph to disk using bincode serialization.
    ///
    /// The graph is saved to `<knowledge_base>/.rsut_index/graph.bin`.
    /// If the directory does not exist, it is created.
    pub fn save_to_disk(&self, knowledge_base: &Path) -> PdfResult<()> {
        let index_dir = knowledge_base.join(".rsut_index");
        fs::create_dir_all(&index_dir)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to create index dir: {}", e)))?;

        let snapshot = self.to_snapshot();
        let bytes = bincode::serialize(&snapshot)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to serialize graph: {}", e)))?;

        let path = index_dir.join("graph.bin");
        fs::write(&path, &bytes).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to write graph to disk: {}", e))
        })?;

        info!(nodes = self.graph.node_count(), edges = self.graph.edge_count(), path = ?path, "Graph saved to disk");
        Ok(())
    }

    /// Load the graph from disk, falling back to a fresh rebuild if the file
    /// is missing or corrupt.
    ///
    /// Returns the loaded graph and a boolean indicating whether a rebuild was needed.
    pub fn load_from_disk_or_rebuild(
        knowledge_base: &Path,
        wiki_dir: &Path,
    ) -> PdfResult<(Self, bool)> {
        let graph_path = knowledge_base.join(".rsut_index").join("graph.bin");

        if graph_path.exists() {
            match Self::load_from_disk(&graph_path) {
                Ok(graph) => {
                    info!(nodes = graph.node_count(), "Graph loaded from disk cache");
                    return Ok((graph, false));
                }
                Err(e) => {
                    debug!(error = %e, "Graph cache corrupt, falling back to rebuild");
                }
            }
        }

        let mut graph = Self::new();
        graph.rebuild(wiki_dir)?;
        graph.save_to_disk(knowledge_base)?;
        Ok((graph, true))
    }

    /// Load graph from a bincode snapshot file.
    fn load_from_disk(path: &Path) -> PdfResult<Self> {
        let bytes = fs::read(path)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to read graph cache: {}", e)))?;
        let snapshot: GraphSnapshot = bincode::deserialize(&bytes)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to deserialize graph: {}", e)))?;
        Ok(Self::from_snapshot(snapshot))
    }

    fn to_snapshot(&self) -> GraphSnapshot {
        let nodes: Vec<NodeMeta> =
            self.graph.node_indices().map(|idx| self.graph[idx].clone()).collect();

        let mut node_idx_to_usize = HashMap::new();
        for (i, idx) in self.graph.node_indices().enumerate() {
            node_idx_to_usize.insert(idx, i);
        }

        let edges: Vec<(usize, usize, EdgeKind)> = self
            .graph
            .edge_references()
            .map(|e| {
                (node_idx_to_usize[&e.source()], node_idx_to_usize[&e.target()], e.weight().clone())
            })
            .collect();

        let path_to_index: Vec<(String, usize)> = self
            .path_to_node
            .iter()
            .map(|(path, idx)| (path.clone(), node_idx_to_usize[idx]))
            .collect();

        GraphSnapshot { nodes, edges, path_to_index }
    }

    fn from_snapshot(snapshot: GraphSnapshot) -> Self {
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

        Self { graph, path_to_node }
    }

    /// Rebuild the graph by scanning all wiki entries.
    pub fn rebuild(&mut self, wiki_dir: &Path) -> PdfResult<usize> {
        self.graph.clear();
        self.path_to_node.clear();

        let mut entries = Vec::new();
        Self::scan_entries(wiki_dir, wiki_dir, &mut entries)?;

        // Add all nodes first
        for (path, entry) in &entries {
            let rel = path.strip_prefix(wiki_dir).unwrap_or(path).to_string_lossy().to_string();

            let meta = NodeMeta {
                path: rel.clone(),
                title: entry.title.clone(),
                domain: entry.domain.clone(),
                tags: entry.tags.clone(),
                level: format!("{}", entry.level),
            };
            let idx = self.graph.add_node(meta);
            self.path_to_node.insert(rel, idx);
        }

        // Add edges from related and contradictions
        for (path, entry) in &entries {
            let rel = path.strip_prefix(wiki_dir).unwrap_or(path).to_string_lossy().to_string();

            let from_idx = match self.path_to_node.get(&rel) {
                Some(idx) => *idx,
                None => continue,
            };

            for related in &entry.related {
                let related_lower = related.to_lowercase();
                if let Some(to_idx) = self
                    .path_to_node
                    .iter()
                    .find(|(k, _)| k.to_lowercase() == related_lower)
                    .map(|(_, &v)| v)
                {
                    self.graph.add_edge(from_idx, to_idx, EdgeKind::Related);
                }
            }

            for contra in &entry.contradictions {
                let contra_lower = contra.to_lowercase();
                if let Some(to_idx) = self
                    .path_to_node
                    .iter()
                    .find(|(k, _)| k.to_lowercase() == contra_lower)
                    .map(|(_, &v)| v)
                {
                    self.graph.add_edge(from_idx, to_idx, EdgeKind::Contradiction);
                }
            }
        }

        // Add tag co-occurrence edges (weak relations)
        let node_indices: Vec<NodeIndex> = self.graph.node_indices().collect();
        for i in 0..node_indices.len() {
            for j in (i + 1)..node_indices.len() {
                let ni = node_indices[i];
                let nj = node_indices[j];
                let tags_a: HashSet<&str> =
                    self.graph[ni].tags.iter().map(|s| s.as_str()).collect();
                let tags_b: HashSet<&str> =
                    self.graph[nj].tags.iter().map(|s| s.as_str()).collect();
                let intersection = tags_a.intersection(&tags_b).count();
                let union = tags_a.union(&tags_b).count();
                if union > 0 {
                    let jaccard = intersection as f64 / union as f64;
                    if jaccard >= 0.3 {
                        self.graph.add_edge(ni, nj, EdgeKind::TagCooccurrence);
                        self.graph.add_edge(nj, ni, EdgeKind::TagCooccurrence);
                    }
                }
            }
        }

        let count = self.graph.node_count();
        Ok(count)
    }

    /// Get N-hop neighbors of an entry.
    pub fn get_neighbors(&self, path: &str, max_hops: u32) -> Vec<NeighborInfo> {
        let start = match self.path_to_node.get(path) {
            Some(idx) => *idx,
            None => return Vec::new(),
        };

        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((start, 0u32));
        visited.insert(start);

        while let Some((node, hops)) = queue.pop_front() {
            if hops >= max_hops {
                continue;
            }

            for edge in self.graph.edges(node) {
                let target = edge.target();
                if visited.insert(target) {
                    let meta = &self.graph[target];
                    let edge_kind = match edge.weight() {
                        EdgeKind::Related => "related",
                        EdgeKind::Contradiction => "contradiction",
                        EdgeKind::TagCooccurrence => "tag_cooccurrence",
                    };
                    result.push(NeighborInfo {
                        path: meta.path.clone(),
                        title: meta.title.clone(),
                        domain: meta.domain.clone(),
                        hops: hops + 1,
                        edge_kind: edge_kind.to_string(),
                    });
                    queue.push_back((target, hops + 1));
                }
            }
        }

        result
    }

    /// Find orphan entries (no incoming or outgoing edges of type Related or Contradiction).
    pub fn find_orphans(&self) -> Vec<String> {
        self.graph
            .node_indices()
            .filter(|&idx| {
                let has_relation = self
                    .graph
                    .edges(idx)
                    .any(|e| matches!(e.weight(), EdgeKind::Related | EdgeKind::Contradiction));
                let has_incoming = self
                    .graph
                    .edges_directed(idx, petgraph::Direction::Incoming)
                    .any(|e| matches!(e.weight(), EdgeKind::Related | EdgeKind::Contradiction));
                !has_relation && !has_incoming
            })
            .map(|idx| self.graph[idx].path.clone())
            .collect()
    }

    /// Suggest potential links based on tag similarity.
    pub fn suggest_links(&self, path: &str, top_k: usize) -> Vec<LinkSuggestion> {
        let start = match self.path_to_node.get(path) {
            Some(idx) => *idx,
            None => return Vec::new(),
        };

        let tags_a: HashSet<&str> = self.graph[start].tags.iter().map(|s| s.as_str()).collect();
        if tags_a.is_empty() {
            return Vec::new();
        }

        // Find existing direct connections
        let existing: HashSet<NodeIndex> = self.graph.edges(start).map(|e| e.target()).collect();

        let mut suggestions: Vec<LinkSuggestion> = self
            .graph
            .node_indices()
            .filter(|&idx| idx != start && !existing.contains(&idx))
            .filter_map(|idx| {
                let tags_b: HashSet<&str> =
                    self.graph[idx].tags.iter().map(|s| s.as_str()).collect();
                let intersection = tags_a.intersection(&tags_b).count();
                let union = tags_a.union(&tags_b).count();
                if union == 0 || intersection == 0 {
                    return None;
                }
                let jaccard = intersection as f64 / union as f64;
                if jaccard < 0.1 {
                    return None;
                }
                let shared: Vec<String> =
                    tags_a.intersection(&tags_b).map(|s| s.to_string()).collect();
                Some(LinkSuggestion {
                    from: self.graph[start].path.clone(),
                    to: self.graph[idx].path.clone(),
                    score: jaccard,
                    reason: format!("Shared tags: {}", shared.join(", ")),
                })
            })
            .collect();

        suggestions
            .sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        suggestions.truncate(top_k);
        suggestions
    }

    /// Suggest links for a new entry not yet in the graph, using only tags.
    /// Compares the given tag set against all existing nodes via Jaccard similarity.
    pub fn suggest_links_by_tags(&self, tags: &[String], top_k: usize) -> Vec<LinkSuggestion> {
        let tags_a: HashSet<&str> = tags.iter().map(|s| s.as_str()).collect();
        if tags_a.is_empty() {
            return Vec::new();
        }

        let mut suggestions: Vec<LinkSuggestion> = self
            .graph
            .node_indices()
            .filter_map(|idx| {
                let tags_b: HashSet<&str> =
                    self.graph[idx].tags.iter().map(|s| s.as_str()).collect();
                let intersection = tags_a.intersection(&tags_b).count();
                let union = tags_a.union(&tags_b).count();
                if union == 0 || intersection == 0 {
                    return None;
                }
                let jaccard = intersection as f64 / union as f64;
                if jaccard < 0.1 {
                    return None;
                }
                let shared: Vec<String> =
                    tags_a.intersection(&tags_b).map(|s| s.to_string()).collect();
                Some(LinkSuggestion {
                    from: String::new(),
                    to: self.graph[idx].path.clone(),
                    score: jaccard,
                    reason: format!("Shared tags: {}", shared.join(", ")),
                })
            })
            .collect();

        suggestions
            .sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        suggestions.truncate(top_k);
        suggestions
    }

    /// Export a local concept map around a given entry as Mermaid.js text.
    pub fn export_concept_map(&self, center_path: &str, depth: u32) -> String {
        let start = match self.path_to_node.get(center_path) {
            Some(idx) => *idx,
            None => {
                return format!("graph LR\n    error[\"Entry not found: {}\"]\n", center_path);
            }
        };

        // Collect all nodes within `depth` hops
        let mut nodes = HashSet::new();
        let mut edges = Vec::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((start, 0u32));
        nodes.insert(start);

        while let Some((node, hops)) = queue.pop_front() {
            if hops >= depth {
                continue;
            }
            for edge in self.graph.edges(node) {
                let target = edge.target();
                let is_new = nodes.insert(target);
                let kind = match edge.weight() {
                    EdgeKind::Related => "relates",
                    EdgeKind::Contradiction => "contradicts",
                    EdgeKind::TagCooccurrence => "co-tags",
                };
                edges.push((node, target, kind));
                if is_new {
                    queue.push_back((target, hops + 1));
                }
            }
            // Also check incoming edges
            for edge in self.graph.edges_directed(node, petgraph::Direction::Incoming) {
                let source = edge.source();
                let is_new = nodes.insert(source);
                let kind = match edge.weight() {
                    EdgeKind::Related => "relates",
                    EdgeKind::Contradiction => "contradicts",
                    EdgeKind::TagCooccurrence => "co-tags",
                };
                edges.push((source, node, kind));
                if is_new {
                    queue.push_back((source, hops + 1));
                }
            }
        }

        // Build Mermaid diagram
        let mut mermaid = String::from("graph LR\n");
        let mut node_ids = HashMap::new();

        for (counter, idx) in nodes.iter().enumerate() {
            let meta = &self.graph[*idx];
            let safe_id = format!("n{}", counter);
            let label = meta.title.replace('"', "'");
            let style = if *idx == start {
                format!("    {}[\"{}\"]:::center\n", safe_id, label)
            } else {
                format!("    {}[\"{}\"]\n", safe_id, label)
            };
            mermaid.push_str(&style);
            node_ids.insert(*idx, safe_id);
        }

        // Deduplicate edges
        let mut seen_edges = HashSet::new();
        for (from, to, kind) in &edges {
            let from_id = node_ids.get(from).expect("node should exist in map");
            let to_id = node_ids.get(to).expect("node should exist in map");
            let key = (from_id.clone(), to_id.clone(), *kind);
            if seen_edges.insert(key) {
                let arrow = match *kind {
                    "contradicts" => " --x ",
                    "co-tags" => " -.-> ",
                    _ => " --> ",
                };
                mermaid.push_str(&format!("    {} {}|{}| {}\n", from_id, arrow, kind, to_id));
            }
        }

        mermaid.push_str("    classDef center fill:#f96,stroke:#333,stroke-width:2px\n");
        mermaid
    }

    /// Get all entry paths in the graph.
    pub fn all_paths(&self) -> Vec<String> {
        self.path_to_node.keys().cloned().collect()
    }

    /// Get node count.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get edge count.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Get a reference to the internal graph (for community detection).
    pub fn graph(&self) -> &DiGraph<NodeMeta, EdgeKind> {
        &self.graph
    }

    /// Get a reference to the path-to-node mapping.
    pub fn path_to_node(&self) -> &HashMap<String, NodeIndex> {
        &self.path_to_node
    }

    #[allow(clippy::only_used_in_recursion)]
    fn scan_entries(
        _base: &Path,
        dir: &Path,
        entries: &mut Vec<(PathBuf, KnowledgeEntry)>,
    ) -> PdfResult<()> {
        if !dir.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(dir)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to read dir: {}", e)))?
        {
            let entry = entry
                .map_err(|e| PdfModuleError::Storage(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            if path.is_dir() {
                Self::scan_entries(_base, &path, entries)?;
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if filename == "index.md" || filename == "log.md" {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Some(entry) = KnowledgeEntry::from_markdown(&content) {
                        entries.push((path, entry));
                    }
                }
            }
        }
        Ok(())
    }
}

impl Default for GraphIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_test_graph() -> (TempDir, GraphIndex) {
        let dir = TempDir::new().unwrap();
        let wiki = dir.path().join("wiki").join("it");
        fs::create_dir_all(&wiki).unwrap();

        let md1 = r#"---
title: "Entry A"
domain: "IT"
tags: ["rust", "systems"]
level: l1
status: compiled
created: 2026-01-01T00:00:00Z
updated: 2026-01-01T00:00:00Z
related: ["it/[IT] Entry_B.md"]
---

Body A"#;
        let md2 = r#"---
title: "Entry B"
domain: "IT"
tags: ["rust", "async"]
level: l1
status: compiled
created: 2026-01-01T00:00:00Z
updated: 2026-01-01T00:00:00Z
related: ["it/[IT] Entry_A.md"]
---

Body B"#;
        fs::write(wiki.join("[IT] Entry_A.md"), md1).unwrap();
        fs::write(wiki.join("[IT] Entry_B.md"), md2).unwrap();

        let mut graph = GraphIndex::new();
        graph.rebuild(dir.path().join("wiki").as_path()).unwrap();
        (dir, graph)
    }

    #[test]
    fn test_graph_save_load_roundtrip() {
        let (dir, graph) = make_test_graph();
        assert_eq!(graph.node_count(), 2);
        assert!(graph.edge_count() > 0);

        // Save
        graph.save_to_disk(dir.path()).unwrap();

        // Load
        let wiki_dir = dir.path().join("wiki");
        let (loaded, rebuilt) =
            GraphIndex::load_from_disk_or_rebuild(dir.path(), &wiki_dir).unwrap();
        assert!(!rebuilt, "should load from cache, not rebuild");
        assert_eq!(loaded.node_count(), 2);
        assert_eq!(loaded.edge_count(), graph.edge_count());
    }

    #[test]
    fn test_graph_rebuild_on_corrupt_cache() {
        let (dir, graph) = make_test_graph();
        graph.save_to_disk(dir.path()).unwrap();

        // Corrupt the cache
        let graph_path = dir.path().join(".rsut_index").join("graph.bin");
        fs::write(&graph_path, b"corrupted data").unwrap();

        let wiki_dir = dir.path().join("wiki");
        let (loaded, rebuilt) =
            GraphIndex::load_from_disk_or_rebuild(dir.path(), &wiki_dir).unwrap();
        assert!(rebuilt, "should rebuild on corrupt cache");
        assert_eq!(loaded.node_count(), 2);
    }
}
