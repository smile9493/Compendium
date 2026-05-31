//! Skeleton indexing — importance scoring for knowledge entries.
//!
//! Combines explicit importance, page count signals, and graph connectivity
//! to rank entries for skeleton selection (the high-value subset used for
//! fast overview and graph-first retrieval paths).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::entry::KnowledgeEntry;
use crate::knowledge::index::graph::GraphIndex;

/// Scorer that assigns importance scores to knowledge entries using
/// multiple signals: explicit front-matter value, page count, and graph degree.
pub struct ImportanceScorer {
    /// Minimum in-degree to count as a "hub" for connectivity scoring.
    hub_threshold: usize,
}

/// Per-entry importance breakdown.
#[derive(Debug, Clone, Serialize)]
pub struct ImportanceScore {
    /// Relative wiki path of the entry.
    pub path: String,
    /// Combined importance score (0.0–1.0).
    pub score: f32,
    /// Individual signal contributions.
    pub signals: ImportanceSignals,
}

/// Individual signals contributing to the importance score.
#[derive(Debug, Clone, Serialize)]
pub struct ImportanceSignals {
    /// 0.0–1.0 based on page count (longer sources score higher, capped at 200 pages).
    pub page_count_score: f32,
    /// Explicit importance from front matter (if present).
    pub explicit_importance: Option<f32>,
    /// Graph connectivity: `node_degree / max_degree` (0.0–1.0).
    pub connectivity_score: f32,
}

impl Default for ImportanceScorer {
    fn default() -> Self {
        Self { hub_threshold: 3 }
    }
}

impl ImportanceScorer {
    /// Create a scorer with a custom hub threshold.
    pub fn new(hub_threshold: usize) -> Self {
        Self { hub_threshold }
    }

    /// Score all wiki entries in the knowledge base.
    ///
    /// Walks the wiki directory, reads front matter, and combines signals
    /// from `importance`, `page` count, and graph connectivity.
    pub fn score_entries(
        &self,
        knowledge_base: &Path,
        graph: &GraphIndex,
    ) -> PdfResult<Vec<ImportanceScore>> {
        let wiki_dir = knowledge_base.join("wiki");
        if !wiki_dir.exists() {
            return Ok(Vec::new());
        }

        // Compute max degree for normalization.
        let max_degree = graph
            .all_paths()
            .iter()
            .map(|p| self.total_degree(graph, p))
            .max()
            .unwrap_or(1)
            .max(1) as f32;

        let mut scores = Vec::new();
        self.walk_and_score(&wiki_dir, &wiki_dir, graph, max_degree, &mut scores)?;

        scores.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scores)
    }

    /// Categorize scores into (high_importance_paths, low_importance_paths)
    /// using the top 30% threshold.
    pub fn categorize<'a>(scores: &'a [ImportanceScore]) -> (Vec<&'a str>, Vec<&'a str>) {
        if scores.is_empty() {
            return (Vec::new(), Vec::new());
        }
        let cutoff = (scores.len() as f64 * 0.3).ceil() as usize;
        let high: Vec<&str> = scores.iter().take(cutoff).map(|s| s.path.as_str()).collect();
        let low: Vec<&str> = scores.iter().skip(cutoff).map(|s| s.path.as_str()).collect();
        (high, low)
    }

    fn total_degree(&self, graph: &GraphIndex, path: &str) -> usize {
        graph.reference_in_degree(path) + self.out_degree(graph, path)
    }

    fn out_degree(&self, graph: &GraphIndex, path: &str) -> usize {
        let path_map = graph.path_to_node();
        let Some(&node) = path_map.get(path) else {
            return 0;
        };
        graph.graph().edges(node).count()
    }

    fn walk_and_score(
        &self,
        base: &Path,
        dir: &Path,
        graph: &GraphIndex,
        max_degree: f32,
        out: &mut Vec<ImportanceScore>,
    ) -> PdfResult<()> {
        if !dir.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(dir).map_err(|e| PdfModuleError::Storage(e.to_string()))? {
            let entry = entry.map_err(|e| PdfModuleError::Storage(e.to_string()))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "index.md" || name == "log.md" {
                continue;
            }
            if path.is_dir() {
                if name != ".versions" {
                    self.walk_and_score(base, &path, graph, max_degree, out)?;
                }
                continue;
            }
            if path.extension().is_none_or(|e| e != "md") {
                continue;
            }
            let rel = path.strip_prefix(base).unwrap_or(&path).to_string_lossy().to_string();
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            let Some(kb_entry) = KnowledgeEntry::from_markdown(&content) else {
                continue;
            };

            let explicit = kb_entry.importance;
            let page_count_score = page_count_to_score(kb_entry.page.as_deref());
            let degree = self.total_degree(graph, &rel) as f32;
            let connectivity_score = if max_degree > 0.0 { degree / max_degree } else { 0.0 };

            // Weighted combination: 40% explicit, 30% page count, 30% connectivity.
            let explicit_weight = explicit.unwrap_or(0.5);
            let score = explicit_weight * 0.4 + page_count_score * 0.3 + connectivity_score * 0.3;

            out.push(ImportanceScore {
                path: rel,
                score: score.clamp(0.0, 1.0),
                signals: ImportanceSignals {
                    page_count_score,
                    explicit_importance: explicit,
                    connectivity_score,
                },
            });
        }
        Ok(())
    }
}

/// Convert page count string to a 0.0–1.0 score.
/// Accepts "12", "70-198", "70-198,200-210" formats.
fn page_count_to_score(page: Option<&str>) -> f32 {
    let Some(page_str) = page else {
        return 0.0;
    };
    let total_pages: u32 = page_str
        .split(',')
        .filter_map(|range| {
            let parts: Vec<&str> = range.trim().split('-').collect();
            match parts.len() {
                1 => parts[0].trim().parse::<u32>().ok(),
                2 => {
                    let start = parts[0].trim().parse::<u32>().ok()?;
                    let end = parts[1].trim().parse::<u32>().ok()?;
                    Some(end.saturating_sub(start).max(1))
                }
                _ => None,
            }
        })
        .sum();
    // Normalize: 0 pages → 0.0, 200+ pages → 1.0.
    (total_pages as f32 / 200.0).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_count_to_score_none() {
        assert_eq!(page_count_to_score(None), 0.0);
    }

    #[test]
    fn test_page_count_to_score_single() {
        let score = page_count_to_score(Some("100"));
        assert!((score - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_page_count_to_score_range() {
        let score = page_count_to_score(Some("70-198"));
        assert!((score - 0.64).abs() < 0.01);
    }

    #[test]
    fn test_page_count_to_score_composite() {
        let score = page_count_to_score(Some("1-10,20-30"));
        assert!((score - 0.1).abs() < 0.01);
    }

    #[test]
    fn test_page_count_capped() {
        let score = page_count_to_score(Some("1-500"));
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_categorize_empty() {
        let (high, low) = ImportanceScorer::categorize(&[]);
        assert!(high.is_empty());
        assert!(low.is_empty());
    }

    #[test]
    fn test_categorize_split() {
        let scores = vec![
            ImportanceScore {
                path: "a".into(),
                score: 0.9,
                signals: ImportanceSignals {
                    page_count_score: 0.0,
                    explicit_importance: None,
                    connectivity_score: 0.0,
                },
            },
            ImportanceScore {
                path: "b".into(),
                score: 0.5,
                signals: ImportanceSignals {
                    page_count_score: 0.0,
                    explicit_importance: None,
                    connectivity_score: 0.0,
                },
            },
            ImportanceScore {
                path: "c".into(),
                score: 0.1,
                signals: ImportanceSignals {
                    page_count_score: 0.0,
                    explicit_importance: None,
                    connectivity_score: 0.0,
                },
            },
        ];
        let (high, low) = ImportanceScorer::categorize(&scores);
        assert_eq!(high.len(), 1);
        assert_eq!(high[0], "a");
        assert_eq!(low.len(), 2);
    }
}
