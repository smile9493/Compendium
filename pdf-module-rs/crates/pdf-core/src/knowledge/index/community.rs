//! Community detection via Label Propagation Algorithm (LPA).
//!
//! A lightweight, deterministic community detection algorithm that replaces the
//! simple Jaccard threshold approach. LPA converges quickly on sparse knowledge
//! graphs and produces meaningful clusters for aggregation candidates.
//!
//! ## Algorithm
//!
//! 1. Each node starts with a unique community label (its index).
//! 2. Iteratively, each node adopts the most frequent label among its neighbors.
//! 3. Ties are broken deterministically by lowest label value.
//! 4. Convergence: stops when labels stabilize or max iterations reached.

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

use crate::knowledge::index::graph::{EdgeKind, NodeMeta};

/// Result of community detection: a list of communities, each containing
/// the paths of its member entries.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Community {
    /// The community label (arbitrary, for grouping).
    pub label: usize,
    /// Entry paths belonging to this community.
    pub members: Vec<String>,
}

/// Detect communities in the knowledge graph using Label Propagation.
///
/// Returns communities sorted by size (largest first), filtering out
/// single-member communities unless they are the only result.
///
/// # Arguments
/// * `graph` - The directed knowledge graph
/// * `max_iterations` - Maximum LPA iterations (default: 50)
pub fn detect_communities(
    graph: &DiGraph<NodeMeta, EdgeKind>,
    max_iterations: Option<usize>,
) -> Vec<Community> {
    let max_iter = max_iterations.unwrap_or(50);
    let node_count = graph.node_count();

    if node_count == 0 {
        return Vec::new();
    }

    // Initialize: each node has its own unique label
    let mut labels: HashMap<NodeIndex, usize> = HashMap::with_capacity(node_count);
    for (i, idx) in graph.node_indices().enumerate() {
        labels.insert(idx, i);
    }

    // LPA iterations
    for iteration in 0..max_iter {
        let mut changed = false;
        let nodes: Vec<NodeIndex> = graph.node_indices().collect();

        for &node in &nodes {
            // Collect neighbor labels with frequency
            let mut label_freq: HashMap<usize, usize> = HashMap::new();

            // Outgoing edges
            for edge in graph.edges(node) {
                let neighbor = edge.target();
                if let Some(&label) = labels.get(&neighbor) {
                    *label_freq.entry(label).or_insert(0) += 1;
                }
            }

            // Incoming edges (undirected behavior for community detection)
            for edge in graph.edges_directed(node, petgraph::Direction::Incoming) {
                let neighbor = edge.source();
                if let Some(&label) = labels.get(&neighbor) {
                    *label_freq.entry(label).or_insert(0) += 1;
                }
            }

            if label_freq.is_empty() {
                continue;
            }

            // Pick the label with highest frequency; break ties by lowest label value
            let best_label = label_freq
                .iter()
                .max_by(|(la, fa), (lb, fb)| fb.cmp(fa).then_with(|| lb.cmp(la)))
                .map(|(&label, _)| label)
                .expect("label_freq is non-empty");

            if labels[&node] != best_label {
                labels.insert(node, best_label);
                changed = true;
            }
        }

        if !changed {
            tracing::debug!(iterations = iteration + 1, "LPA converged");
            break;
        }
    }

    // Group nodes by label
    let mut groups: HashMap<usize, Vec<String>> = HashMap::new();
    for (idx, label) in &labels {
        let path = graph[*idx].path.clone();
        groups.entry(*label).or_default().push(path);
    }

    // Build communities, filter singles, sort by size descending
    let mut communities: Vec<Community> = groups
        .into_iter()
        .map(|(label, mut members)| {
            members.sort();
            Community { label, members }
        })
        .collect();

    communities.sort_by_key(|b| std::cmp::Reverse(b.members.len()));

    // Keep multi-member communities; if all are single-member, return empty
    if communities.iter().all(|c| c.members.len() < 2) {
        return Vec::new();
    }

    communities.into_iter().filter(|c| c.members.len() >= 2).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::DiGraph;

    fn make_node(path: &str) -> NodeMeta {
        NodeMeta {
            path: path.to_string(),
            title: path.to_string(),
            domain: "IT".to_string(),
            tags: vec![],
            level: "L1".to_string(),
        }
    }

    #[test]
    fn test_single_cluster_detected() {
        let mut graph = DiGraph::new();
        let a = graph.add_node(make_node("a"));
        let b = graph.add_node(make_node("b"));
        let c = graph.add_node(make_node("c"));

        graph.add_edge(a, b, EdgeKind::Related);
        graph.add_edge(b, c, EdgeKind::Related);
        graph.add_edge(c, a, EdgeKind::Related);

        let communities = detect_communities(&graph, None);
        // All three should merge into one community
        assert_eq!(communities.len(), 1);
        assert_eq!(communities[0].members.len(), 3);
    }

    #[test]
    fn test_two_separate_clusters() {
        let mut graph = DiGraph::new();
        let a = graph.add_node(make_node("a"));
        let b = graph.add_node(make_node("b"));
        let c = graph.add_node(make_node("c"));
        let d = graph.add_node(make_node("d"));

        // Cluster 1: a-b
        graph.add_edge(a, b, EdgeKind::Related);
        // Cluster 2: c-d
        graph.add_edge(c, d, EdgeKind::Related);

        let communities = detect_communities(&graph, None);
        assert_eq!(communities.len(), 2);
        for c in &communities {
            assert_eq!(c.members.len(), 2);
        }
    }

    #[test]
    fn test_empty_graph() {
        let graph: DiGraph<NodeMeta, EdgeKind> = DiGraph::new();
        let communities = detect_communities(&graph, None);
        assert!(communities.is_empty());
    }

    #[test]
    fn test_disconnected_nodes_no_community() {
        let mut graph = DiGraph::new();
        graph.add_node(make_node("a"));
        graph.add_node(make_node("b"));
        // No edges -> all single-member -> empty result

        let communities = detect_communities(&graph, None);
        assert!(communities.is_empty());
    }
}
