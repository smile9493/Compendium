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
use crate::knowledge::entry::{KnowledgeEntry, extract_markdown_body};
use crate::knowledge::index::fulltext::SearchHit;
use crate::knowledge::index::vector::{VectorHit, VectorIndex};
use crate::knowledge::index::{FulltextIndex, GraphIndex};
use crate::knowledge::publish_gate::{GateConfig, is_searchable};
use crate::management::config_manager::ConfigManager;

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
    /// Karpathy-style: index.md + graph neighbors, no Tantivy/vector.
    WikiFirst,
}

impl SearchMode {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "keyword" | "fulltext" => Self::Keyword,
            "semantic" | "vector" => Self::Semantic,
            "wiki_first" | "wiki-first" | "wiki" => Self::WikiFirst,
            "hybrid" => Self::Hybrid,
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

/// Options controlling search behavior across HTTP, MCP, and CLI.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// When true, use filesystem scan if Tantivy returns no hits or errors.
    pub allow_fs_fallback: bool,
    /// When true, rebuild Tantivy from wiki if the index is empty before searching.
    pub rebuild_if_empty: bool,
    /// Restrict keyword search to this domain (Tantivy filter).
    pub domain: Option<String>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        let allow_fs_fallback = std::env::var("RSUT_SEARCH_ALLOW_FS_FALLBACK")
            .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
        Self { allow_fs_fallback, rebuild_if_empty: false, domain: None }
    }
}

impl SearchOptions {
    /// HTTP / MCP defaults: Tantivy only, no implicit rebuild.
    pub fn for_api() -> Self {
        Self { allow_fs_fallback: false, rebuild_if_empty: false, domain: None }
    }

    /// CLI defaults: rebuild empty index on demand.
    pub fn for_cli() -> Self {
        Self { allow_fs_fallback: false, rebuild_if_empty: true, domain: None }
    }
}

/// Metadata returned alongside search hits.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SearchMeta {
    pub index_empty: bool,
    pub used_fallback: bool,
    pub mode: String,
}

/// Unified search response for HTTP and MCP.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub meta: SearchMeta,
}

/// Wiki directory under a knowledge base root.
pub fn wiki_dir(knowledge_base: &Path) -> PathBuf {
    knowledge_base.join("wiki")
}

/// Hybrid search (default): Tantivy CJK + TF-IDF vectors fused via RRF.
pub fn search(knowledge_base: &Path, query: &str, limit: usize) -> PdfResult<Vec<SearchHit>> {
    Ok(search_with_options(
        knowledge_base,
        query,
        limit,
        SearchMode::Hybrid,
        SearchOptions::default(),
    )?
    .hits)
}

/// Search with an explicit mode (hits only; default options).
pub fn search_with_mode(
    knowledge_base: &Path,
    query: &str,
    limit: usize,
    mode: SearchMode,
) -> PdfResult<Vec<SearchHit>> {
    Ok(search_with_options(knowledge_base, query, limit, mode, SearchOptions::default())?.hits)
}

/// Search with explicit mode and options.
pub fn search_with_options(
    knowledge_base: &Path,
    query: &str,
    limit: usize,
    mode: SearchMode,
    opts: SearchOptions,
) -> PdfResult<SearchResponse> {
    search_with_options_ft(knowledge_base, query, limit, mode, opts, None)
}

/// Search using an optional pre-opened fulltext index (from [`IndexCache`]).
pub fn search_with_options_ft(
    knowledge_base: &Path,
    query: &str,
    limit: usize,
    mode: SearchMode,
    opts: SearchOptions,
    ft_override: Option<&FulltextIndex>,
) -> PdfResult<SearchResponse> {
    let wd = wiki_dir(knowledge_base);
    let mode_str = format!("{:?}", mode).to_lowercase();
    if !wd.exists() || query.trim().is_empty() {
        return Ok(SearchResponse {
            hits: Vec::new(),
            meta: SearchMeta { mode: mode_str, ..Default::default() },
        });
    }

    let (hits, index_empty, used_fallback) = match mode {
        SearchMode::WikiFirst => {
            (search_wiki_first(knowledge_base, &wd, query, limit)?, false, false)
        }
        SearchMode::Keyword => {
            let kr = search_keyword(knowledge_base, &wd, query, limit, &opts, ft_override)?;
            (kr.hits, kr.index_empty, kr.used_fallback)
        }
        SearchMode::Semantic => (search_semantic(knowledge_base, query, limit)?, false, false),
        SearchMode::Hybrid => {
            let kr = search_keyword(
                knowledge_base,
                &wd,
                query,
                limit.saturating_mul(2).max(limit),
                &opts,
                ft_override,
            )?;
            let index_empty = kr.index_empty;
            let used_fallback = kr.used_fallback;
            let semantic = match ensure_vector_index(knowledge_base) {
                Ok(()) => {
                    let index = load_vector_index(knowledge_base)?;
                    index.search(query, limit.saturating_mul(2).max(limit))
                }
                Err(e) => {
                    debug!(error = %e, "Vector index unavailable for hybrid search");
                    Vec::new()
                }
            };
            (rrf_merge(kr.hits, semantic, limit), index_empty, used_fallback)
        }
    };

    let hits = filter_searchable(knowledge_base, &wd, hits, limit);
    let hits = crate::knowledge::cognitive_diversity::deduplicate_search_hits(hits, knowledge_base);
    Ok(SearchResponse { hits, meta: SearchMeta { index_empty, used_fallback, mode: mode_str } })
}

/// Resolve default search mode from `retrieval_mode` in kb config (`wiki_first` | `hybrid`).
pub fn default_search_mode(knowledge_base: &Path) -> SearchMode {
    let mut cm = ConfigManager::new(knowledge_base);
    if cm.load().is_ok()
        && let Some(mode) = cm.get("retrieval_mode")
        && mode.eq_ignore_ascii_case("wiki_first")
    {
        return SearchMode::WikiFirst;
    }
    SearchMode::Hybrid
}

fn search_wiki_first(
    knowledge_base: &Path,
    wiki_dir: &Path,
    query: &str,
    limit: usize,
) -> PdfResult<Vec<SearchHit>> {
    let q = query.to_lowercase();
    let mut hits = Vec::new();
    let index_path = wiki_dir.join("index.md");
    if index_path.exists()
        && let Ok(index_text) = fs::read_to_string(&index_path)
    {
        for line in index_text.lines() {
            if line.contains('|') && line.contains("[[") {
                let lower = line.to_lowercase();
                if lower.contains(&q)
                    && let Some(start) = line.find("[[")
                    && let Some(end) = line[start + 2..].find("]]")
                {
                    let path = line[start + 2..start + 2 + end].trim().to_string();
                    push_wiki_first_hit(wiki_dir, &path, query, 1.0, &mut hits);
                }
            }
        }
    }

    let mut graph = GraphIndex::new();
    let _ = graph.rebuild(wiki_dir);
    for entry_path in collect_wiki_paths(wiki_dir)? {
        let full = wiki_dir.join(&entry_path);
        let Ok(content) = fs::read_to_string(&full) else {
            continue;
        };
        let body = extract_markdown_body(&content).unwrap_or(&content);
        let title = KnowledgeEntry::from_markdown(&content).map(|e| e.title).unwrap_or_default();
        let searchable = format!("{} {}", title, body).to_lowercase();
        if searchable.contains(&q) {
            let score = if title.to_lowercase().contains(&q) { 0.95 } else { 0.75 };
            push_wiki_first_hit(wiki_dir, &entry_path, query, score, &mut hits);
        }
    }

    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    hits.dedup_by(|a, b| a.path == b.path);
    if hits.len() > limit {
        hits.truncate(limit);
    }

    if hits.is_empty() && graph.node_count() > 0 {
        for path in graph.find_orphans().into_iter().take(3) {
            push_wiki_first_hit(wiki_dir, &path, query, 0.1, &mut hits);
        }
    }

    let _ = knowledge_base;
    Ok(hits)
}

fn push_wiki_first_hit(
    wiki_dir: &Path,
    path: &str,
    query: &str,
    score: f32,
    hits: &mut Vec<SearchHit>,
) {
    let full = wiki_dir.join(path);
    let (title, domain, snippet) = if let Ok(content) = fs::read_to_string(&full) {
        let entry = KnowledgeEntry::from_markdown(&content);
        let title = entry.as_ref().map(|e| e.title.clone()).unwrap_or_else(|| path.to_string());
        let domain = entry.as_ref().map(|e| e.domain.clone()).unwrap_or_default();
        let body = extract_markdown_body(&content).unwrap_or(&content);
        let snip = body.chars().take(200).collect::<String>();
        (title, domain, snip)
    } else {
        (path.to_string(), String::new(), String::new())
    };
    if hits.iter().any(|h| h.path == path) {
        return;
    }
    hits.push(SearchHit { path: path.to_string(), title, domain, score, snippet });
    let _ = query;
}

fn collect_wiki_paths(wiki_dir: &Path) -> PdfResult<Vec<String>> {
    let mut paths = Vec::new();
    collect_wiki_paths_rec(wiki_dir, wiki_dir, &mut paths)?;
    Ok(paths)
}

fn collect_wiki_paths_rec(base: &Path, dir: &Path, out: &mut Vec<String>) -> PdfResult<()> {
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
                collect_wiki_paths_rec(base, &path, out)?;
            }
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            let rel = path.strip_prefix(base).unwrap_or(&path).to_string_lossy().to_string();
            out.push(rel.replace('\\', "/"));
        }
    }
    Ok(())
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
            if let Ok(content) = fs::read_to_string(&full)
                && let Some(entry) = KnowledgeEntry::from_markdown(&content)
            {
                return is_searchable(&entry, config.quality_min_score);
            }
            false
        })
        .take(limit)
        .collect()
}

struct KeywordSearchResult {
    hits: Vec<SearchHit>,
    index_empty: bool,
    used_fallback: bool,
}

fn search_keyword(
    knowledge_base: &Path,
    wiki_dir: &Path,
    query: &str,
    limit: usize,
    opts: &SearchOptions,
    ft_override: Option<&FulltextIndex>,
) -> PdfResult<KeywordSearchResult> {
    let expanded = expand_query_for_tantivy(query);
    let domain = opts.domain.as_deref();
    match search_tantivy(knowledge_base, wiki_dir, &expanded, limit, domain, opts, ft_override) {
        Ok(result) if !result.hits.is_empty() => Ok(result),
        Ok(result) if result.index_empty => Ok(result),
        Ok(result) if opts.allow_fs_fallback => {
            let hits = fs_fallback_search(wiki_dir, query, limit, domain);
            Ok(KeywordSearchResult { hits, index_empty: result.index_empty, used_fallback: true })
        }
        Ok(result) => Ok(result),
        Err(e) if opts.allow_fs_fallback => {
            debug!(error = %e, "Tantivy search failed, using filesystem fallback");
            let hits = fs_fallback_search(wiki_dir, query, limit, domain);
            Ok(KeywordSearchResult { hits, index_empty: false, used_fallback: true })
        }
        Err(e) => Err(e),
    }
}

fn search_semantic(knowledge_base: &Path, query: &str, limit: usize) -> PdfResult<Vec<SearchHit>> {
    ensure_vector_index(knowledge_base)?;
    let index = load_vector_index(knowledge_base)?;
    let hits = index.search(query, limit);
    Ok(vector_hits_to_search_hits(hits, knowledge_base, query))
}

fn expand_query_for_tantivy(query: &str) -> String {
    let words: Vec<String> = JIEBA
        .cut(query, false)
        .into_iter()
        .map(|w| w.trim().to_string())
        .filter(|w| !w.is_empty() && w.chars().count() > 1)
        .collect();
    if words.len() <= 1 { query.to_string() } else { words.join(" OR ") }
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
    domain: Option<&str>,
    opts: &SearchOptions,
    ft_override: Option<&FulltextIndex>,
) -> PdfResult<KeywordSearchResult> {
    if let Some(ft) = ft_override {
        return search_tantivy_with_index(ft, wiki_dir, query, limit, domain, opts);
    }
    let owned = FulltextIndex::open_or_create(knowledge_base)?;
    search_tantivy_with_index(&owned, wiki_dir, query, limit, domain, opts)
}

fn search_tantivy_with_index(
    index: &FulltextIndex,
    wiki_dir: &Path,
    query: &str,
    limit: usize,
    domain: Option<&str>,
    opts: &SearchOptions,
) -> PdfResult<KeywordSearchResult> {
    let empty = index.is_empty()?;
    if empty {
        if opts.rebuild_if_empty {
            index.rebuild(wiki_dir)?;
        } else {
            return Ok(KeywordSearchResult {
                hits: Vec::new(),
                index_empty: true,
                used_fallback: false,
            });
        }
    }
    let hits = index.search(query, limit, domain)?;
    Ok(KeywordSearchResult { hits, index_empty: empty, used_fallback: false })
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

/// Rebuild fulltext, graph, and vector indexes from `wiki/` (default propagation policy).
pub fn rebuild_all(knowledge_base: &Path) -> PdfResult<RebuildStats> {
    rebuild_all_with_policy(
        knowledge_base,
        &crate::knowledge::confidence_propagation::PropagationPolicy::default(),
    )
    .map(|(stats, _)| stats)
}

/// Rebuild indexes and run confidence propagation with the given policy.
pub fn rebuild_all_with_policy(
    knowledge_base: &Path,
    propagation_policy: &crate::knowledge::confidence_propagation::PropagationPolicy,
) -> PdfResult<(
    RebuildStats,
    Option<crate::knowledge::confidence_propagation::ConfidencePropagationReport>,
)> {
    let wd = wiki_dir(knowledge_base);
    if !wd.exists() {
        return Ok((
            RebuildStats {
                fulltext_entries_indexed: 0,
                graph_nodes: 0,
                graph_edges: 0,
                vector_entries_indexed: 0,
            },
            None,
        ));
    }

    let ft_idx = FulltextIndex::open_or_create(knowledge_base)?;
    let ft_count = ft_idx.rebuild(&wd)?;

    let mut g_idx = GraphIndex::new();
    let g_count = g_idx.rebuild(&wd)?;
    g_idx.save_to_disk(knowledge_base)?;

    let vector_count = rebuild_vectors(knowledge_base)?;

    let propagation_report = Some(crate::knowledge::confidence_propagation::run_propagation(
        knowledge_base,
        &g_idx,
        propagation_policy,
    )?);

    Ok((
        RebuildStats {
            fulltext_entries_indexed: ft_count,
            graph_nodes: g_count,
            graph_edges: g_idx.edge_count(),
            vector_entries_indexed: vector_count,
        },
        propagation_report,
    ))
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
    let body = extract_markdown_body(&content).unwrap_or("").to_string();

    let mut v_idx = {
        let idx = load_vector_index(knowledge_base)
            .or_else(|_| VectorIndex::open_or_create(knowledge_base, VECTOR_DIM))?;
        if idx.is_empty() {
            let _ = rebuild_vectors(knowledge_base)?;
            load_vector_index(knowledge_base)
                .or_else(|_| VectorIndex::open_or_create(knowledge_base, VECTOR_DIM))?
        } else {
            idx
        }
    };
    v_idx.index_entry(rel, &entry.title, &entry.domain, &body);
    v_idx.save()?;

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
            if let Ok(content) = fs::read_to_string(&path)
                && let Some(entry) = KnowledgeEntry::from_markdown(&content)
            {
                if !is_searchable(&entry, config.quality_min_score) {
                    continue;
                }
                let rel = path.strip_prefix(base).unwrap_or(&path).to_string_lossy().to_string();
                let body = extract_markdown_body(&content).unwrap_or("").to_string();
                out.push((rel, entry.title, entry.domain, body));
            }
        }
    }
    Ok(())
}

// ── Filesystem fallback (used when Tantivy is empty or errors) ──

fn fs_fallback_search(
    wiki_dir: &Path,
    query: &str,
    limit: usize,
    domain_filter: Option<&str>,
) -> Vec<SearchHit> {
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
        if domain_filter.is_some_and(|d| d != domain) {
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
            let Some(name) = fp.file_name() else {
                continue;
            };
            let rel_path = format!("{}/{}", domain, name.to_string_lossy());
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

        let resp =
            search_with_options(kb, "反向代理", 5, SearchMode::Hybrid, SearchOptions::for_api())
                .unwrap();
        assert!(
            resp.hits.iter().any(|h| h.path.contains("nginx_proxy")),
            "hybrid should find Chinese content"
        );
        assert_eq!(resp.meta.mode, "hybrid");
        assert!(!resp.meta.used_fallback);
        assert!(!resp.meta.index_empty);
    }

    fn write_draft_entry(kb: &Path, domain: &str, name: &str, body: &str) {
        let dir = kb.join("wiki").join(domain);
        fs::create_dir_all(&dir).unwrap();
        let content = format!(
            "---\ntitle: \"{name}\"\ndomain: \"{domain}\"\ntags: [test]\nlevel: L1\nstatus: compiled\npublish_status: draft\nquality_score: 0.9\ncreated: 2026-01-01\nupdated: 2026-01-01\n---\n\n{body}"
        );
        fs::write(dir.join(format!("{name}.md")), content).unwrap();
    }

    #[test]
    fn test_draft_not_indexed_in_tantivy() {
        let dir = tempfile::tempdir().unwrap();
        let kb = dir.path();
        write_draft_entry(kb, "IT", "secret_draft", "UniqueDraftTokenXYZ");
        rebuild_all(kb).unwrap();

        let opts = SearchOptions::for_api();
        let resp =
            search_with_options(kb, "UniqueDraftTokenXYZ", 5, SearchMode::Keyword, opts).unwrap();
        assert!(
            !resp.hits.iter().any(|h| h.path.contains("secret_draft")),
            "draft entries must not appear in search"
        );
    }

    #[test]
    fn test_empty_index_returns_meta() {
        let dir = tempfile::tempdir().unwrap();
        let kb = dir.path();
        fs::create_dir_all(kb.join("wiki")).unwrap();
        let opts = SearchOptions::for_api();
        let resp = search_with_options(kb, "anything", 5, SearchMode::Keyword, opts).unwrap();
        assert!(resp.meta.index_empty);
        assert!(!resp.meta.used_fallback);
        assert!(resp.hits.is_empty());
    }

    #[test]
    fn test_domain_filter_tantivy() {
        let dir = tempfile::tempdir().unwrap();
        let kb = dir.path();
        write_entry(kb, "IT", "it_only", "DomainFilterToken");
        write_entry(kb, "HR", "hr_only", "DomainFilterToken");
        rebuild_all(kb).unwrap();

        let mut opts = SearchOptions::for_api();
        opts.domain = Some("IT".to_string());
        let resp =
            search_with_options(kb, "DomainFilterToken", 10, SearchMode::Keyword, opts).unwrap();
        assert!(resp.hits.iter().all(|h| h.domain == "IT"));
        assert!(resp.hits.iter().any(|h| h.path.contains("it_only")));
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
