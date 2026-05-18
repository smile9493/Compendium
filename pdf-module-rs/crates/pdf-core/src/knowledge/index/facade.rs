//! Unified index operations for all entry points (HTTP Wiki API, MCP tools, CLI).
//!
//! Ensures Tantivy fulltext search, petgraph knowledge graph, and rebuild persistence
//! behave identically regardless of caller.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tracing::debug;

use crate::error::PdfResult;
use crate::knowledge::index::fulltext::SearchHit;
use crate::knowledge::index::{FulltextIndex, GraphIndex};

/// Statistics returned after a full index rebuild.
#[derive(Debug, Clone, Serialize)]
pub struct RebuildStats {
    pub fulltext_entries_indexed: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
}

/// Wiki directory under a knowledge base root.
pub fn wiki_dir(knowledge_base: &Path) -> PathBuf {
    knowledge_base.join("wiki")
}

/// Full-text search with Tantivy; falls back to filesystem scan if the index fails.
pub fn search(knowledge_base: &Path, query: &str, limit: usize) -> PdfResult<Vec<SearchHit>> {
    let wd = wiki_dir(knowledge_base);
    if !wd.exists() {
        return Ok(Vec::new());
    }

    match search_tantivy(knowledge_base, &wd, query, limit) {
        Ok(hits) if !hits.is_empty() => Ok(hits),
        Ok(_) => Ok(fs_fallback_search(&wd, query, limit)),
        Err(e) => {
            debug!(error = %e, "Tantivy search failed, using filesystem fallback");
            Ok(fs_fallback_search(&wd, query, limit))
        }
    }
}

fn search_tantivy(
    knowledge_base: &Path,
    wiki_dir: &Path,
    query: &str,
    limit: usize,
) -> PdfResult<Vec<SearchHit>> {
    let index = FulltextIndex::open_or_create(knowledge_base)?;
    if index.is_empty()? {
        index.rebuild(wiki_dir)?;
    }
    index.search(query, limit)
}

/// Load or rebuild the knowledge graph (persisted at `.rsut_index/graph.bin`).
pub fn graph(knowledge_base: &Path) -> PdfResult<GraphIndex> {
    let wd = wiki_dir(knowledge_base);
    if !wd.exists() {
        return Ok(GraphIndex::new());
    }
    let (g, _) = GraphIndex::load_from_disk_or_rebuild(knowledge_base, &wd)?;
    Ok(g)
}

/// Rebuild fulltext and graph indexes from `wiki/` and persist the graph to disk.
pub fn rebuild_all(knowledge_base: &Path) -> PdfResult<RebuildStats> {
    let wd = wiki_dir(knowledge_base);
    if !wd.exists() {
        return Ok(RebuildStats {
            fulltext_entries_indexed: 0,
            graph_nodes: 0,
            graph_edges: 0,
        });
    }

    let ft_idx = FulltextIndex::open_or_create(knowledge_base)?;
    let ft_count = ft_idx.rebuild(&wd)?;

    let mut g_idx = GraphIndex::new();
    let g_count = g_idx.rebuild(&wd)?;
    g_idx.save_to_disk(knowledge_base)?;

    Ok(RebuildStats {
        fulltext_entries_indexed: ft_count,
        graph_nodes: g_count,
        graph_edges: g_idx.edge_count(),
    })
}

// ── Filesystem fallback (used when Tantivy is empty or errors) ──

fn fs_fallback_search(wiki_dir: &Path, query: &str, limit: usize) -> Vec<SearchHit> {
    let mut results: Vec<SearchHit> = Vec::new();
    let lower_q = query.to_lowercase();

    let Ok(domain_entries) = std::fs::read_dir(wiki_dir) else {
        return results;
    };

    for de in domain_entries.flatten() {
        let dp = de.path();
        if !dp.is_dir() {
            continue;
        }
        let domain = dp
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if domain.starts_with('.') {
            continue;
        }

        let Ok(files) = std::fs::read_dir(&dp) else {
            continue;
        };
        for f in files.flatten() {
            let fp = f.path();
            if fp.extension().is_none_or(|e| e != "md") {
                continue;
            }
            let rel_path = format!(
                "{}/{}",
                domain,
                fp.file_name().unwrap().to_string_lossy()
            );
            let title = fp
                .file_stem()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let Ok(content) = std::fs::read_to_string(&fp) else {
                continue;
            };
            let lower_content = content.to_lowercase();
            let count = lower_content.matches(&lower_q).count();
            if count == 0 {
                continue;
            }

            let score =
                (count as f32 / (content.len().max(1) as f32).sqrt()) * 100.0;
            let snippet = extract_snippet_fs(&content, &lower_q, 80);
            results.push(SearchHit {
                path: rel_path,
                title,
                domain: domain.clone(),
                score,
                snippet,
            });
        }
    }

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(limit);
    results
}

fn extract_snippet_fs(content: &str, lower_q: &str, window: usize) -> String {
    let lower_content = content.to_lowercase();
    if let Some(byte_pos) = lower_content.find(lower_q) {
        let pre = if byte_pos > 0 { "..." } else { "" };
        let post = if byte_pos + lower_q.len() + window < content.len() {
            "..."
        } else {
            ""
        };

        let byte_start = byte_pos.saturating_sub(window / 2);
        let begin = floor_char_boundary(content, byte_start);

        let byte_end = (byte_pos + lower_q.len() + window).min(content.len());
        let end = ceil_char_boundary(content, byte_end);

        format!("{}{}{}", pre, &content[begin..end], post)
    } else {
        let s: String = content.chars().take(window).collect();
        format!("{s}...")
    }
}

fn floor_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos.min(s.len());
    while p > 0 && !s.is_char_boundary(p) {
        p -= 1;
    }
    p
}

fn ceil_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos.min(s.len());
    while p < s.len() && !s.is_char_boundary(p) {
        p += 1;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_entry(kb: &Path, domain: &str, name: &str, body: &str) {
        let dir = kb.join("wiki").join(domain);
        fs::create_dir_all(&dir).unwrap();
        let content = format!(
            "---\ntitle: \"{name}\"\ndomain: \"{domain}\"\ntags: [test]\nlevel: L1\nstatus: compiled\nquality_score: 0.9\n---\n\n{body}"
        );
        fs::write(dir.join(format!("{name}.md")), content).unwrap();
    }

    #[test]
    fn test_rebuild_all_persists_graph() {
        let dir = tempfile::tempdir().unwrap();
        let kb = dir.path();
        write_entry(kb, "IT", "rust_basics", "Rust ownership and borrowing");
        write_entry(kb, "IT", "http2", "HTTP/2 multiplexing streams");

        let stats = rebuild_all(kb).unwrap();
        assert!(stats.fulltext_entries_indexed >= 2);
        assert!(stats.graph_nodes >= 2);

        let graph_path = kb.join(".rsut_index").join("graph.bin");
        assert!(graph_path.exists());

        let hits = search(kb, "Rust", 10).unwrap();
        assert!(hits.iter().any(|h| h.path.contains("rust_basics")));
    }

    #[test]
    fn test_search_consistent_after_rebuild() {
        let dir = tempfile::tempdir().unwrap();
        let kb = dir.path();
        write_entry(kb, "IT", "alpha", "alpha bravo content");
        rebuild_all(kb).unwrap();

        let a = search(kb, "alpha", 5).unwrap();
        let b = search(kb, "alpha", 5).unwrap();
        let paths_a: Vec<_> = a.iter().map(|h| h.path.as_str()).collect();
        let paths_b: Vec<_> = b.iter().map(|h| h.path.as_str()).collect();
        assert_eq!(paths_a, paths_b);
    }
}
