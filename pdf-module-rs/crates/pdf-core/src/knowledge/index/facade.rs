//! Unified index operations for all entry points (HTTP Wiki API, MCP tools, CLI).
//!
//! Ensures Tantivy fulltext search, TF-IDF vector search, petgraph knowledge graph,
//! and rebuild persistence behave identically regardless of caller.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use jieba_rs::Jieba;
use serde::Serialize;
use tracing::debug;

use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::entry::KnowledgeEntry;
use crate::knowledge::index::fulltext::SearchHit;
use crate::knowledge::index::vector::{VectorHit, VectorIndex};
use crate::knowledge::index::{FulltextIndex, GraphIndex};
use crate::knowledge::publish_gate::{is_searchable, GateConfig};

static JIEBA: LazyLock<Jieba> = LazyLock::new(Jieba::new);

const VECTOR_DIM: usize = 256;
const RRF_K: f32 = 60.0;

/// Search strategy for knowledge retrieval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchMode {
    /// Tantivy full-text only (CJK jieba tokenizer).
    Keyword,
    /// TF-IDF cosine similarity only.
    Semantic,
    /// Reciprocal Rank Fusion of keyword + semantic (default).
    #[default]
    Hybrid,
}

impl SearchMode {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "keyword" | "fulltext" => Self::Keyword,
            "semantic" | "vector" => Self::Semantic,
            _ => Self::Hybrid,
        }
    }
}

/// Statistics returned after a full index rebuild.
#[derive(Debug, Clone, Serialize)]
pub struct RebuildStats {
    pub fulltext_entries_indexed: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub vector_entries_indexed: usize,
}

/// Wiki directory under a knowledge base root.
pub fn wiki_dir(knowledge_base: &Path) -> PathBuf {
    knowledge_base.join("wiki")
}

/// Hybrid search (default): Tantivy CJK + TF-IDF vectors fused via RRF.
pub fn search(knowledge_base: &Path, query: &str, limit: usize) -> PdfResult<Vec<SearchHit>> {
    search_with_mode(knowledge_base, query, limit, SearchMode::Hybrid)
}

/// Search with an explicit mode.
pub fn search_with_mode(
    knowledge_base: &Path,
    query: &str,
    limit: usize,
    mode: SearchMode,
) -> PdfResult<Vec<SearchHit>> {
    let wd = wiki_dir(knowledge_base);
    if !wd.exists() || query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let hits = match mode {
        SearchMode::Keyword => search_keyword(knowledge_base, &wd, query, limit),
        SearchMode::Semantic => search_semantic(knowledge_base, query, limit),
        SearchMode::Hybrid => search_hybrid(knowledge_base, &wd, query, limit),
    }?;
    Ok(filter_searchable(knowledge_base, &wd, hits, limit))
}

fn filter_searchable(
    knowledge_base: &Path,
    wiki_dir: &Path,
    hits: Vec<SearchHit>,
    limit: usize,
) -> Vec<SearchHit> {
    let config = GateConfig::load(knowledge_base).unwrap_or_default();
    hits.into_iter()
        .filter(|h| {
            let full = wiki_dir.join(&h.path);
            if let Ok(content) = fs::read_to_string(&full) {
                if let Some(entry) = KnowledgeEntry::from_markdown(&content) {
                    return is_searchable(&entry, config.quality_min_score);
                }
            }
            false
        })
        .take(limit)
        .collect()
}

fn search_keyword(
    knowledge_base: &Path,
    wiki_dir: &Path,
    query: &str,
    limit: usize,
) -> PdfResult<Vec<SearchHit>> {
    let expanded = expand_query_for_tantivy(query);
    match search_tantivy(knowledge_base, wiki_dir, &expanded, limit) {
        Ok(hits) if !hits.is_empty() => Ok(hits),
        Ok(_) => Ok(fs_fallback_search(wiki_dir, query, limit)),
        Err(e) => {
            debug!(error = %e, "Tantivy search failed, using filesystem fallback");
            Ok(fs_fallback_search(wiki_dir, query, limit))
        }
    }
}

fn search_semantic(knowledge_base: &Path, query: &str, limit: usize) -> PdfResult<Vec<SearchHit>> {
    ensure_vector_index(knowledge_base)?;
    let index = load_vector_index(knowledge_base)?;
    let hits = index.search(query, limit);
    Ok(vector_hits_to_search_hits(hits, knowledge_base, query))
}

fn search_hybrid(
    knowledge_base: &Path,
    wiki_dir: &Path,
    query: &str,
    limit: usize,
) -> PdfResult<Vec<SearchHit>> {
    let fetch = limit.saturating_mul(2).max(limit);
    let keyword = search_keyword(knowledge_base, wiki_dir, query, fetch)?;
    let semantic = match ensure_vector_index(knowledge_base) {
        Ok(()) => {
            let index = load_vector_index(knowledge_base)?;
            index.search(query, fetch)
        }
        Err(e) => {
            debug!(error = %e, "Vector index unavailable for hybrid search");
            Vec::new()
        }
    };
    Ok(rrf_merge(keyword, semantic, limit))
}

fn expand_query_for_tantivy(query: &str) -> String {
    let words: Vec<String> = JIEBA
        .cut(query, false)
        .into_iter()
        .map(|w| w.trim().to_string())
        .filter(|w| !w.is_empty() && w.chars().count() > 1)
        .collect();
    if words.len() <= 1 {
        query.to_string()
    } else {
        words.join(" OR ")
    }
}

fn rrf_merge(keyword: Vec<SearchHit>, semantic: Vec<VectorHit>, limit: usize) -> Vec<SearchHit> {
    let mut by_path: HashMap<String, (f32, SearchHit)> = HashMap::new();

    for (rank, hit) in keyword.into_iter().enumerate() {
        let rrf = 1.0 / (RRF_K + rank as f32 + 1.0);
        let path = hit.path.clone();
        let snippet = hit.snippet.clone();
        by_path
            .entry(path)
            .and_modify(|(score, h)| {
                *score += rrf;
                if h.snippet.is_empty() && !snippet.is_empty() {
                    h.snippet.clone_from(&snippet);
                }
            })
            .or_insert((rrf, hit));
    }

    for (rank, vhit) in semantic.into_iter().enumerate() {
        let rrf = 1.0 / (RRF_K + rank as f32 + 1.0);
        by_path.entry(vhit.path.clone()).and_modify(|(score, _)| *score += rrf).or_insert_with(
            || {
                (
                    rrf,
                    SearchHit {
                        path: vhit.path,
                        title: vhit.title,
                        domain: vhit.domain,
                        score: 0.0,
                        snippet: String::new(),
                    },
                )
            },
        );
    }

    let mut merged: Vec<SearchHit> = by_path
        .into_iter()
        .map(|(_, (rrf_score, mut hit))| {
            hit.score = rrf_score;
            hit
        })
        .collect();

    merged.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    merged.truncate(limit);
    merged
}

fn vector_hits_to_search_hits(
    hits: Vec<VectorHit>,
    knowledge_base: &Path,
    query: &str,
) -> Vec<SearchHit> {
    let wd = wiki_dir(knowledge_base);
    hits.into_iter()
        .map(|h| {
            let snippet = read_snippet_for_path(&wd, &h.path, query);
            SearchHit { path: h.path, title: h.title, domain: h.domain, score: h.score, snippet }
        })
        .collect()
}

fn read_snippet_for_path(wiki_dir: &Path, rel_path: &str, query: &str) -> String {
    let fp = wiki_dir.join(rel_path);
    let Ok(content) = fs::read_to_string(&fp) else {
        return String::new();
    };
    extract_snippet_fs(&content, &query.to_lowercase(), 120)
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

/// Rebuild fulltext, graph, and vector indexes from `wiki/`.
pub fn rebuild_all(knowledge_base: &Path) -> PdfResult<RebuildStats> {
    let wd = wiki_dir(knowledge_base);
    if !wd.exists() {
        return Ok(RebuildStats {
            fulltext_entries_indexed: 0,
            graph_nodes: 0,
            graph_edges: 0,
            vector_entries_indexed: 0,
        });
    }

    let ft_idx = FulltextIndex::open_or_create(knowledge_base)?;
    let ft_count = ft_idx.rebuild(&wd)?;

    let mut g_idx = GraphIndex::new();
    let g_count = g_idx.rebuild(&wd)?;
    g_idx.save_to_disk(knowledge_base)?;

    let vector_count = rebuild_vectors(knowledge_base)?;

    Ok(RebuildStats {
        fulltext_entries_indexed: ft_count,
        graph_nodes: g_count,
        graph_edges: g_idx.edge_count(),
        vector_entries_indexed: vector_count,
    })
}

/// Rebuild the TF-IDF vector index from all wiki entries.
pub fn rebuild_vectors(knowledge_base: &Path) -> PdfResult<usize> {
    let wd = wiki_dir(knowledge_base);
    if !wd.exists() {
        return Ok(0);
    }

    let config = GateConfig::load(knowledge_base).unwrap_or_default();
    let mut entries_data: Vec<(String, String, String, String)> = Vec::new();
    scan_wiki_for_embedding(&wd, &wd, knowledge_base, &config, &mut entries_data)?;

    if entries_data.is_empty() {
        return Ok(0);
    }

    let mut index = VectorIndex::open_or_create(knowledge_base, VECTOR_DIM)?;
    let docs: Vec<String> =
        entries_data.iter().map(|(_, title, _, body)| format!("{title} {body}")).collect();
    index.train_model(&docs);

    for (path, title, domain, body) in &entries_data {
        index.index_entry(path, title, domain, body);
    }

    let count = index.len();
    index.save()?;
    Ok(count)
}

/// Incrementally update indexes for a single wiki entry after a patch or save.
pub fn reindex_entry(knowledge_base: &Path, entry_path: &str) -> PdfResult<()> {
    let wd = wiki_dir(knowledge_base);
    let rel = entry_path.trim_start_matches("wiki/").trim_start_matches('/');
    let full_path = wd.join(rel);
    if !full_path.exists() {
        return Err(PdfModuleError::FileNotFound(full_path.to_string_lossy().to_string()));
    }

    let ft = FulltextIndex::open_or_create(knowledge_base)?;
    if ft.is_empty()? {
        ft.rebuild(&wd)?;
    } else {
        ft.upsert_entry(&wd, rel)?;
    }

    let content = fs::read_to_string(&full_path)
        .map_err(|e| PdfModuleError::Storage(format!("Failed to read entry: {e}")))?;
    let entry = KnowledgeEntry::from_markdown(&content)
        .ok_or_else(|| PdfModuleError::Storage("Failed to parse front matter".to_string()))?;
    let body = content.split("---").nth(2).unwrap_or("").to_string();

    let mut v_idx = load_vector_index(knowledge_base)
        .or_else(|_| VectorIndex::open_or_create(knowledge_base, VECTOR_DIM))?;
    if v_idx.is_empty() {
        let _ = rebuild_vectors(knowledge_base)?;
        v_idx = load_vector_index(knowledge_base)?;
    } else {
        v_idx.index_entry(rel, &entry.title, &entry.domain, &body);
        v_idx.save()?;
    }

    let mut g_idx = GraphIndex::new();
    let _ = g_idx.rebuild(&wd)?;
    g_idx.save_to_disk(knowledge_base)?;

    Ok(())
}

fn ensure_vector_index(knowledge_base: &Path) -> PdfResult<()> {
    let path = knowledge_base.join(".rsut_index").join("vectors").join("vectors.bin");
    if path.exists() {
        return Ok(());
    }
    rebuild_vectors(knowledge_base)?;
    Ok(())
}

fn load_vector_index(knowledge_base: &Path) -> PdfResult<VectorIndex> {
    VectorIndex::load(knowledge_base, VECTOR_DIM)?.ok_or_else(|| {
        PdfModuleError::Storage("Vector index not found; call rebuild_index first".to_string())
    })
}

#[allow(clippy::only_used_in_recursion)]
fn scan_wiki_for_embedding(
    base: &Path,
    dir: &Path,
    knowledge_base: &Path,
    config: &GateConfig,
    out: &mut Vec<(String, String, String, String)>,
) -> PdfResult<()> {
    let _ = knowledge_base;
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| PdfModuleError::Storage(e.to_string()))? {
        let entry = entry.map_err(|e| PdfModuleError::Storage(e.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            scan_wiki_for_embedding(base, &path, knowledge_base, config, out)?;
        } else if path.extension().is_none_or(|e| e == "md") {
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if filename == "index.md" || filename == "log.md" {
                continue;
            }
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some(entry) = KnowledgeEntry::from_markdown(&content) {
                    if !is_searchable(&entry, config.quality_min_score) {
                        continue;
                    }
                    let rel =
                        path.strip_prefix(base).unwrap_or(&path).to_string_lossy().to_string();
                    let body = content.split("---").nth(2).unwrap_or("").to_string();
                    out.push((rel, entry.title, entry.domain, body));
                }
            }
        }
    }
    Ok(())
}

// ── Filesystem fallback (used when Tantivy is empty or errors) ──

fn fs_fallback_search(wiki_dir: &Path, query: &str, limit: usize) -> Vec<SearchHit> {
    let mut results: Vec<SearchHit> = Vec::new();
    let lower_q = query.to_lowercase();
    let terms: Vec<String> = JIEBA
        .cut(query, false)
        .into_iter()
        .map(|w| w.trim().to_lowercase())
        .filter(|w| !w.is_empty())
        .collect();

    let Ok(domain_entries) = fs::read_dir(wiki_dir) else {
        return results;
    };

    for de in domain_entries.flatten() {
        let dp = de.path();
        if !dp.is_dir() {
            continue;
        }
        let domain = dp.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if domain.starts_with('.') {
            continue;
        }

        let Ok(files) = fs::read_dir(&dp) else {
            continue;
        };
        for f in files.flatten() {
            let fp = f.path();
            if fp.extension().is_none_or(|e| e != "md") {
                continue;
            }
            let rel_path = format!("{}/{}", domain, fp.file_name().unwrap().to_string_lossy());
            let title = fp.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

            let Ok(content) = fs::read_to_string(&fp) else {
                continue;
            };
            let lower_content = content.to_lowercase();
            let count = if terms.len() > 1 {
                terms.iter().filter(|t| lower_content.contains(t.as_str())).count()
            } else {
                lower_content.matches(&lower_q).count()
            };
            if count == 0 {
                continue;
            }

            let score = (count as f32 / (content.len().max(1) as f32).sqrt()) * 100.0;
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

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    results
}

fn extract_snippet_fs(content: &str, lower_q: &str, window: usize) -> String {
    let lower_content = content.to_lowercase();
    if let Some(byte_pos) = lower_content.find(lower_q) {
        let pre = if byte_pos > 0 { "..." } else { "" };
        let post = if byte_pos + lower_q.len() + window < content.len() { "..." } else { "" };

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
            "---\ntitle: \"{name}\"\ndomain: \"{domain}\"\ntags: [test]\nlevel: L1\nstatus: compiled\npublish_status: published\nquality_score: 0.9\ncreated: 2026-01-01\nupdated: 2026-01-01\n---\n\n{body}"
        );
        fs::write(dir.join(format!("{name}.md")), content).unwrap();
    }

    #[test]
    fn test_rebuild_all_persists_graph_and_vectors() {
        let dir = tempfile::tempdir().unwrap();
        let kb = dir.path();
        write_entry(kb, "IT", "rust_basics", "Rust ownership and borrowing");
        write_entry(kb, "IT", "http2", "HTTP/2 multiplexing streams");

        let stats = rebuild_all(kb).unwrap();
        assert!(stats.fulltext_entries_indexed >= 2);
        assert!(stats.graph_nodes >= 2);
        assert!(stats.vector_entries_indexed >= 2);

        let vector_path = kb.join(".rsut_index").join("vectors").join("vectors.bin");
        assert!(vector_path.exists());

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

    #[test]
    fn test_chinese_hybrid_search() {
        let dir = tempfile::tempdir().unwrap();
        let kb = dir.path();
        write_entry(kb, "IT", "nginx_proxy", "Nginx 反向代理与负载均衡配置详解");
        rebuild_all(kb).unwrap();

        let hybrid = search_with_mode(kb, "反向代理", 5, SearchMode::Hybrid).unwrap();
        assert!(
            hybrid.iter().any(|h| h.path.contains("nginx_proxy")),
            "hybrid should find Chinese content"
        );
    }

    #[test]
    fn test_reindex_entry_updates_search() {
        let dir = tempfile::tempdir().unwrap();
        let kb = dir.path();
        write_entry(kb, "IT", "patch_me", "original content");
        rebuild_all(kb).unwrap();

        let path = kb.join("wiki/IT/patch_me.md");
        let mut content = fs::read_to_string(&path).unwrap();
        content.push_str("\n\nUniquePhase2TokenXYZ");
        fs::write(&path, content).unwrap();

        reindex_entry(kb, "IT/patch_me.md").unwrap();
        let hits = search(kb, "UniquePhase2TokenXYZ", 5).unwrap();
        assert!(hits.iter().any(|h| h.path.contains("patch_me")));
    }
}
