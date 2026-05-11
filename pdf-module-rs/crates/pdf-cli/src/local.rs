//! # Local Mode
//!
//! Direct pdf-core integration for local knowledge base operations.
//! Skips network entirely; operates on local filesystem.
//!
//! Many functions are public API surface available for future CLI commands.
#![allow(dead_code)]

use anyhow::{Context, Result};
use pdf_core::knowledge::index::FulltextIndex;
use pdf_core::management::{ConfigManager, HealthReporter};
use pdf_core::{GraphIndex, KnowledgeEngine, McpPdfPipeline, ServerConfig};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

/// Create a properly initialized KnowledgeEngine for the given kb path.
fn create_engine(kb_path: &Path) -> Result<KnowledgeEngine> {
    let config = ServerConfig::from_env().unwrap_or_default();
    let pipeline = Arc::new(
        McpPdfPipeline::new(&config).context("Failed to create PDF pipeline")?,
    );
    KnowledgeEngine::new(pipeline, kb_path).context("Failed to create knowledge engine")
}

/// Compile a single PDF into the knowledge base.
pub async fn compile_to_wiki(
    kb_path: &Path,
    pdf_path: &Path,
    domain: Option<&str>,
) -> Result<Value> {
    let engine = create_engine(kb_path)?;
    let result = engine.compile_to_wiki(pdf_path, domain).await?;
    Ok(serde_json::to_value(result)?)
}

/// Micro-compile: extract text from a PDF without saving to wiki.
pub async fn micro_compile(
    _kb_path: &Path,
    pdf_path: &Path,
    page_range: Option<&str>,
) -> Result<Value> {
    let config = ServerConfig::from_env().unwrap_or_default();
    let pipeline = Arc::new(McpPdfPipeline::new(&config)?);
    let options = pdf_core::dto::ExtractOptions::default();

    let result = pipeline
        .extract_structured(pdf_path, &options)
        .await
        .context("PDF extraction failed")?;

    let text = if let Some(range) = page_range {
        let pages_to_include = parse_page_range(range, result.page_count);
        let filtered: Vec<String> = result
            .pages
            .iter()
            .filter(|p| pages_to_include.contains(&p.page_number))
            .map(|p| format!("## Page {}\n\n{}", p.page_number, p.text))
            .collect();
        filtered.join("\n\n")
    } else {
        result.extracted_text.clone()
    };

    let source_name = pdf_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    let output = format!(
        r#"# Micro-Compile: {}

> Note: This content is for current session only. It is NOT saved to the wiki.
> Use `compile` for persistent storage.

- Pages: {}{}

---

{}
"#,
        source_name,
        result.page_count,
        page_range
            .map(|r| format!("\n- Range: {}", r))
            .unwrap_or_default(),
        text
    );

    Ok(serde_json::json!({
        "source": source_name,
        "page_count": result.page_count,
        "page_range": page_range,
        "output": output,
    }))
}

/// Recompile an existing wiki entry.
pub fn recompile_entry(kb_path: &Path, entry_path: &Path) -> Result<Value> {
    let engine = create_engine(kb_path)?;
    let result = engine.recompile_entry(entry_path)?;
    Ok(serde_json::to_value(result)?)
}

/// Run incremental compilation on the raw/ directory.
pub async fn incremental_compile(kb_path: &Path) -> Result<Value> {
    let engine = create_engine(kb_path)?;
    let raw_dir = engine.raw_dir();
    let result = engine.incremental_compile(&raw_dir).await?;
    Ok(serde_json::to_value(result)?)
}

/// Full-text search across wiki entries.
pub fn search(kb_path: &Path, query: &str, limit: usize) -> Result<Value> {
    let index = FulltextIndex::open_or_create(kb_path).context("Failed to open fulltext index")?;

    let wiki_dir = kb_path.join("wiki");
    if wiki_dir.exists() && index.is_empty().unwrap_or(true) {
        index.rebuild(&wiki_dir)?;
    }

    let hits = index.search(query, limit)?;
    Ok(serde_json::to_value(hits)?)
}

/// Get context/neighbors for an entry.
pub fn get_entry_context(kb_path: &Path, entry_path: &str, hops: u32) -> Result<Value> {
    let wiki_dir = kb_path.join("wiki");
    let mut graph = GraphIndex::new();
    graph.rebuild(&wiki_dir)?;

    let neighbors = graph.get_neighbors(entry_path, hops);
    Ok(serde_json::json!({
        "entry": entry_path,
        "hops": hops,
        "neighbors": neighbors,
        "total": neighbors.len()
    }))
}

/// Export concept map as Mermaid.js text.
pub fn export_concept_map(kb_path: &Path, entry_path: &str, depth: u32) -> Result<Value> {
    let wiki_dir = kb_path.join("wiki");
    let mut graph = GraphIndex::new();
    graph.rebuild(&wiki_dir)?;

    let mermaid = graph.export_concept_map(entry_path, depth);
    Ok(serde_json::json!({
        "entry": entry_path,
        "depth": depth,
        "mermaid": mermaid,
    }))
}

/// List orphan entries (no links).
pub fn find_orphans(kb_path: &Path) -> Result<Value> {
    let wiki_dir = kb_path.join("wiki");
    let mut graph = GraphIndex::new();
    graph.rebuild(&wiki_dir)?;

    let orphans = graph.find_orphans();
    Ok(serde_json::json!({
        "orphan_count": orphans.len(),
        "entries": orphans,
    }))
}

/// Knowledge base statistics.
pub fn stats(kb_path: &Path) -> Result<Value> {
    let reporter = HealthReporter::new(kb_path);
    let report = reporter.report()?;

    Ok(serde_json::json!({
        "total_entries": report.total_entries,
        "orphan_count": report.orphan_count,
        "contradiction_count": report.contradiction_count,
        "broken_link_count": report.broken_link_count,
        "index_size_bytes": report.index_size_bytes,
        "graph_node_count": report.graph_node_count,
        "graph_edge_count": report.graph_edge_count,
        "avg_quality_score": format!("{:.1}%", report.avg_quality_score * 100.0),
        "domains": report.domains,
        "last_compile": report.last_compile.map(|t| t.to_rfc3339()),
    }))
}

/// Health report.
pub fn health(kb_path: &Path) -> Result<Value> {
    let reporter = HealthReporter::new(kb_path);
    let report = reporter.report()?;

    Ok(serde_json::json!({
        "total_entries": report.total_entries,
        "orphan_count": report.orphan_count,
        "contradiction_count": report.contradiction_count,
        "broken_link_count": report.broken_link_count,
        "index_size_mb": report.index_size_bytes / 1024 / 1024,
        "graph_nodes": report.graph_node_count,
        "graph_edges": report.graph_edge_count,
        "avg_quality_score": format!("{:.1}%", report.avg_quality_score * 100.0),
        "domains": report.domains,
        "last_compile": report.last_compile.map(|t| t.to_rfc3339()),
        "generated_at": report.generated_at.to_rfc3339(),
        "report_text": report.to_string(),
    }))
}

/// Get config values.
pub fn get_config(kb_path: &Path) -> Result<Value> {
    let mut cm = ConfigManager::new(kb_path);
    cm.load()?;
    let data = cm.all().clone();
    Ok(serde_json::json!({
        "config": data,
        "total_keys": data.len(),
    }))
}

/// Set config value.
pub fn set_config(kb_path: &Path, key: &str, value: &str) -> Result<Value> {
    let mut cm = ConfigManager::new(kb_path);
    cm.load()?;
    cm.set(key, value)?;
    Ok(serde_json::json!({
        "status": "ok",
        "key": key,
        "value": value,
    }))
}

/// Remove config key.
pub fn remove_config(kb_path: &Path, key: &str) -> Result<Value> {
    let mut cm = ConfigManager::new(kb_path);
    cm.load()?;
    cm.remove(key)?;
    Ok(serde_json::json!({
        "status": "ok",
        "removed": key,
    }))
}

/// Rebuild fulltext and graph indexes.
pub fn rebuild_index(kb_path: &Path) -> Result<Value> {
    let wiki_dir = kb_path.join("wiki");
    if !wiki_dir.exists() {
        anyhow::bail!("Wiki directory not found: {}", wiki_dir.display());
    }

    let ft_idx = FulltextIndex::open_or_create(kb_path)?;
    let ft_count = ft_idx.rebuild(&wiki_dir)?;

    let mut g_idx = GraphIndex::new();
    let g_count = g_idx.rebuild(&wiki_dir)?;

    Ok(serde_json::json!({
        "fulltext_entries_indexed": ft_count,
        "graph_nodes": g_count,
        "graph_edges": g_idx.edge_count(),
    }))
}

/// Run quality analysis.
pub fn check_quality(kb_path: &Path) -> Result<Value> {
    let wiki_dir = kb_path.join("wiki");
    let report = pdf_core::knowledge::quality::analyze_wiki(&wiki_dir)?;

    Ok(serde_json::json!({
        "total_entries": report.total_entries,
        "avg_quality_score": format!("{:.1}%", report.avg_quality_score * 100.0),
        "domains": report.domains.iter().collect::<Vec<_>>(),
        "issues_count": report.issues.len(),
        "orphan_count": report.orphan_entries.len(),
        "broken_links_count": report.broken_links.len(),
        "report_markdown": report.to_markdown(),
        "has_errors": report.has_errors(),
        "has_warnings": report.has_warnings(),
    }))
}

/// Aggregate entries into L2 candidates.
pub fn aggregation_candidates(kb_path: &Path) -> Result<Value> {
    let engine = create_engine(kb_path)?;
    let candidates = engine.identify_aggregation_candidates()?;
    Ok(serde_json::to_value(candidates)?)
}

/// Find contradiction pairs.
pub fn find_contradictions(kb_path: &Path) -> Result<Value> {
    let engine = create_engine(kb_path)?;
    let pairs = engine.find_contradictions()?;
    Ok(serde_json::to_value(pairs)?)
}

/// Parse page range string (e.g. "1-5,7,10-12") into sorted unique page numbers.
fn parse_page_range(range: &str, max_page: u32) -> Vec<u32> {
    let mut pages = Vec::new();
    for part in range.split(',') {
        let part = part.trim();
        if let Some(dash_pos) = part.find('-') {
            let start = part[..dash_pos].trim().parse::<u32>().unwrap_or(1);
            let end = part[dash_pos + 1..].trim().parse::<u32>().unwrap_or(max_page);
            for p in start..=end.min(max_page) {
                pages.push(p);
            }
        } else if let Ok(p) = part.parse::<u32>() && p <= max_page {
                pages.push(p);
        }
    }
    pages.sort();
    pages.dedup();
    pages
}
