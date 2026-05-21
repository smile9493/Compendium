//! Cognitive diversity metrics — detect over-concentration and near-duplicate results.
//!
//! - **Diversity index**: fraction of graph nodes that are not "over-referenced" hubs.
//! - **Diversity warnings**: high in-degree without balancing contradiction edges.
//! - **Search dedup**: penalize near-duplicate hits in hybrid ranking (TF-IDF similarity).

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::entry::{KnowledgeEntry, extract_markdown_body};
use crate::knowledge::index::fulltext::SearchHit;
use crate::knowledge::index::graph::GraphIndex;
use crate::knowledge::index::vector::{EmbeddingModel, TfidfModel, cosine_similarity};
use crate::knowledge::index::wiki_dir;

/// Fixed hub threshold when graph is empty or has fewer than 50 nodes.
pub const DEFAULT_HUB_IN_DEGREE: usize = 5;

/// Cosine similarity at or above which search hits are treated as near-duplicates.
pub const NEAR_DUPLICATE_THRESHOLD: f32 = 0.92;

/// Summary for quality / lint reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveDiversityReport {
    /// 0.0 = monoculture (many hubs), 1.0 = evenly distributed references.
    pub diversity_index: f32,
    pub total_entries: usize,
    pub hub_count: usize,
    pub near_duplicate_pairs: usize,
    pub warnings: Vec<DiversityWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiversityWarning {
    pub path: String,
    pub title: String,
    pub in_degree: usize,
    pub message: String,
}

/// Analyze reference concentration using the knowledge graph.
///
/// Prefer [`hub_threshold_for_kb`] for adaptive + config override.
pub fn analyze_cognitive_diversity(
    knowledge_base: &Path,
    graph: &GraphIndex,
    hub_threshold: usize,
) -> PdfResult<CognitiveDiversityReport> {
    let wd = wiki_dir(knowledge_base);
    let mut entries: Vec<(String, KnowledgeEntry)> = Vec::new();
    scan_wiki(&wd, &wd, &mut entries)?;
    let total = entries.len().max(1);
    let mut hub_count = 0usize;
    let mut warnings = Vec::new();

    for (rel, entry) in &entries {
        let in_deg = graph.reference_in_degree(rel);
        if in_deg >= hub_threshold {
            hub_count += 1;
            let has_contra_out = graph.path_to_node().contains_key(rel)
                && graph
                    .graph()
                    .edges_directed(
                        *graph.path_to_node().get(rel).expect("node"),
                        petgraph::Direction::Outgoing,
                    )
                    .any(|e| {
                        matches!(
                            e.weight(),
                            crate::knowledge::index::graph::EdgeKind::Contradiction
                        )
                    });
            if !has_contra_out {
                warnings.push(DiversityWarning {
                    path: rel.clone(),
                    title: entry.title.clone(),
                    in_degree: in_deg,
                    message: format!(
                        "Referenced by {in_deg} entries without outbound contradiction links — risk of viewpoint monoculture"
                    ),
                });
            }
        }
    }

    let diversity_index = 1.0 - (hub_count as f32 / total as f32);
    let near_duplicate_pairs = count_near_duplicate_pairs(&wd, &entries);

    warnings.sort_by_key(|b| std::cmp::Reverse(b.in_degree));

    Ok(CognitiveDiversityReport {
        diversity_index,
        total_entries: entries.len(),
        hub_count,
        near_duplicate_pairs,
        warnings,
    })
}

fn count_near_duplicate_pairs(wiki_dir: &Path, entries: &[(String, KnowledgeEntry)]) -> usize {
    if entries.len() < 2 {
        return 0;
    }
    let corpus: Vec<String> = entries
        .iter()
        .map(|(rel, e)| {
            let body = fs::read_to_string(wiki_dir.join(rel))
                .ok()
                .and_then(|c| extract_markdown_body(&c).map(str::to_string))
                .unwrap_or_default();
            format!("{} {}", e.title, body)
        })
        .collect();
    let mut model = TfidfModel::new(256);
    model.train(&corpus);
    let embeddings: Vec<Vec<f32>> = corpus.iter().map(|d| model.embed(d)).collect();
    let mut pairs = 0usize;
    for i in 0..embeddings.len() {
        for j in (i + 1)..embeddings.len() {
            if cosine_similarity(&embeddings[i], &embeddings[j]) >= NEAR_DUPLICATE_THRESHOLD {
                pairs += 1;
            }
        }
    }
    pairs
}

/// Remove near-duplicate hits from a ranked list (keeps higher score).
pub fn deduplicate_search_hits(hits: Vec<SearchHit>, knowledge_base: &Path) -> Vec<SearchHit> {
    if hits.len() < 2 {
        return hits;
    }
    let wd = wiki_dir(knowledge_base);
    let texts: Vec<String> = hits
        .iter()
        .map(|h| {
            let body = fs::read_to_string(wd.join(&h.path))
                .ok()
                .and_then(|c| extract_markdown_body(&c).map(str::to_string))
                .unwrap_or_default();
            format!("{} {} {}", h.title, h.snippet, body.chars().take(500).collect::<String>())
        })
        .collect();
    let mut model = TfidfModel::new(128);
    model.train(&texts);
    let embeddings: Vec<Vec<f32>> = texts.iter().map(|t| model.embed(t)).collect();

    let mut kept: Vec<SearchHit> = Vec::new();
    let mut kept_embeds: Vec<Vec<f32>> = Vec::new();

    for (hit, emb) in hits.into_iter().zip(embeddings) {
        let duplicate =
            kept_embeds.iter().any(|k| cosine_similarity(k, &emb) >= NEAR_DUPLICATE_THRESHOLD);
        if !duplicate {
            kept_embeds.push(emb);
            kept.push(hit);
        }
    }
    kept
}

fn scan_wiki(base: &Path, dir: &Path, out: &mut Vec<(String, KnowledgeEntry)>) -> PdfResult<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| PdfModuleError::Storage(format!("read dir: {e}")))? {
        let entry = entry.map_err(|e| PdfModuleError::Storage(format!("read entry: {e}")))?;
        let path = entry.path();
        if path.is_dir() {
            scan_wiki(base, &path, out)?;
        } else if path.extension().is_some_and(|e| e == "md") {
            let rel = path.strip_prefix(base).unwrap_or(&path).to_string_lossy().replace('\\', "/");
            if let Ok(content) = fs::read_to_string(&path)
                && let Some(meta) = KnowledgeEntry::from_markdown(&content)
            {
                out.push((rel, meta));
            }
        }
    }
    Ok(())
}
