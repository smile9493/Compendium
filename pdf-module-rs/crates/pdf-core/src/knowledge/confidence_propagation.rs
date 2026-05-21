//! Confidence propagation over the knowledge graph.
//!
//! ## Semantics
//!
//! - Each entry declares `confidence` (high / medium / low) for **claims on that page**.
//! - **Effective confidence** flows forward along `related` / `wikilink` edges and `aggregated_from`
//!   for up to `propagation_depth` hops (default 2: L0→L1→L2).
//! - Effective rank = min(declared, propagated from upstream).
//! - Outbound **contradiction** edges within depth mark targets `needs_recompile`.
//!
//! ## Policy
//!
//! - `auto_write` (default **true**): persist adjustments to wiki front matter after rebuild.
//! - `dry_run`: compute report only; no file writes or log append.
//! - Changes are summarized in `wiki/log.md` when writes occur.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use petgraph::Direction;
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};

use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::entry::{
    CompileStatus, EntryConfidence, KnowledgeEntry, extract_markdown_body,
};
use crate::knowledge::index::graph::{EdgeKind, GraphIndex};
use crate::knowledge::index::wiki_dir;
use crate::wiki::{NervousEvent, NervousEventKind, WikiStorage};

/// Controls persistence and depth of confidence propagation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PropagationPolicy {
    /// Persist wiki front matter when not `dry_run` (default true).
    #[serde(default = "default_auto_write")]
    pub auto_write: bool,
    /// Report only; no writes or log append.
    #[serde(default)]
    pub dry_run: bool,
    /// Max forward hops along reference edges (default 2).
    #[serde(default = "default_propagation_depth")]
    pub propagation_depth: u8,
}

fn default_auto_write() -> bool {
    true
}

fn default_propagation_depth() -> u8 {
    2
}

impl Default for PropagationPolicy {
    fn default() -> Self {
        Self { auto_write: true, dry_run: false, propagation_depth: default_propagation_depth() }
    }
}

impl PropagationPolicy {
    pub fn dry_run_only(depth: u8) -> Self {
        Self { auto_write: false, dry_run: true, propagation_depth: depth }
    }

    /// Build policy from MCP/CLI JSON args (`dry_run`, `auto_write`, `propagation_depth`).
    pub fn from_json_args(args: &serde_json::Value) -> Self {
        Self {
            auto_write: args.get("auto_write").and_then(|v| v.as_bool()).unwrap_or(true),
            dry_run: args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false),
            propagation_depth: args
                .get("propagation_depth")
                .and_then(|v| v.as_u64())
                .unwrap_or(2)
                .clamp(1, 8) as u8,
        }
    }
}

/// Per-entry propagation outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropagatedEntry {
    pub path: String,
    pub declared_confidence: Option<String>,
    pub effective_confidence: String,
    pub status_after: String,
    pub downgraded: bool,
    pub needs_review: bool,
    pub inbound_sources: Vec<String>,
}

/// Full propagation report for a knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidencePropagationReport {
    pub entries_scanned: usize,
    pub downgraded_count: usize,
    pub needs_review_count: usize,
    pub written_count: usize,
    pub dry_run: bool,
    /// True when `auto_write` was false (report only, no persistence attempted).
    #[serde(default)]
    pub skipped_write: bool,
    pub propagation_depth: u8,
    pub entries: Vec<PropagatedEntry>,
}

/// Ordinal strength for min-confidence merge.
fn confidence_rank(c: EntryConfidence) -> u8 {
    match c {
        EntryConfidence::High => 2,
        EntryConfidence::Medium => 1,
        EntryConfidence::Low => 0,
    }
}

fn rank_to_confidence(r: u8) -> EntryConfidence {
    match r {
        2 => EntryConfidence::High,
        1 => EntryConfidence::Medium,
        _ => EntryConfidence::Low,
    }
}

/// Compute effective confidence with bounded-hop forward propagation.
pub fn compute_propagation(
    knowledge_base: &Path,
    graph: &GraphIndex,
    depth: u8,
) -> PdfResult<ConfidencePropagationReport> {
    let wd = wiki_dir(knowledge_base);
    let mut by_path: HashMap<String, (PathBuf, KnowledgeEntry)> = HashMap::new();
    scan_wiki(&wd, &wd, &mut by_path)?;

    let mut effective: HashMap<String, u8> = HashMap::new();
    for (rel, (_, entry)) in &by_path {
        let rank = confidence_rank(entry.confidence.clone().unwrap_or(EntryConfidence::Medium));
        effective.insert(rel.clone(), rank);
    }

    let mut needs_review: HashMap<String, bool> = HashMap::new();
    let mut inbound_sources: HashMap<String, Vec<String>> = HashMap::new();

    let hops = depth.max(1);
    for _ in 0..hops {
        let snapshot = effective.clone();
        let g = graph.graph();
        for (rel, (_, _entry)) in &by_path {
            let Some(&idx) = graph.path_to_node().get(rel) else {
                continue;
            };
            let src_rank = *snapshot.get(rel).unwrap_or(&1);
            for edge in g.edges_directed(idx, Direction::Outgoing) {
                let target = g[edge.target()].path.clone();
                match edge.weight() {
                    EdgeKind::Related | EdgeKind::Wikilink => {
                        effective
                            .entry(target.clone())
                            .and_modify(|r| *r = (*r).min(src_rank))
                            .or_insert(src_rank);
                        inbound_sources.entry(target).or_default().push(rel.clone());
                    }
                    EdgeKind::Contradiction => {
                        needs_review.insert(target.clone(), true);
                        inbound_sources.entry(target).or_default().push(rel.clone());
                    }
                    EdgeKind::TagCooccurrence => {}
                }
            }
        }
        for (rel, (_, entry)) in &by_path {
            for agg in &entry.aggregated_from {
                let key = normalize_path(agg);
                if let Some(&src_rank) = snapshot.get(&key) {
                    effective
                        .entry(rel.clone())
                        .and_modify(|r| *r = (*r).min(src_rank))
                        .or_insert(src_rank);
                    inbound_sources.entry(rel.clone()).or_default().push(key);
                }
            }
        }
    }

    build_report(&by_path, &effective, &needs_review, &inbound_sources, false, 0, depth, false)
}

/// Run propagation: compute, optionally write wiki + log.md.
pub fn run_propagation(
    knowledge_base: &Path,
    graph: &GraphIndex,
    policy: &PropagationPolicy,
) -> PdfResult<ConfidencePropagationReport> {
    let mut report = compute_propagation(knowledge_base, graph, policy.propagation_depth)?;
    report.dry_run = policy.dry_run;

    if policy.dry_run {
        return Ok(report);
    }
    if !policy.auto_write {
        report.skipped_write = true;
        return Ok(report);
    }

    let written = write_report_to_wiki(knowledge_base, &report)?;
    report.written_count = written;
    append_propagation_log(knowledge_base, &report)?;
    Ok(report)
}

/// Legacy entry: default policy (auto-write, depth 2).
pub fn apply_propagation(knowledge_base: &Path) -> PdfResult<ConfidencePropagationReport> {
    let mut graph = GraphIndex::new();
    graph.rebuild(&wiki_dir(knowledge_base))?;
    run_propagation(knowledge_base, &graph, &PropagationPolicy::default())
}

fn build_report(
    by_path: &HashMap<String, (PathBuf, KnowledgeEntry)>,
    effective: &HashMap<String, u8>,
    needs_review: &HashMap<String, bool>,
    inbound_sources: &HashMap<String, Vec<String>>,
    dry_run: bool,
    written_count: usize,
    depth: u8,
    skipped_write: bool,
) -> PdfResult<ConfidencePropagationReport> {
    let mut entries = Vec::new();
    let mut downgraded_count = 0usize;
    let mut needs_review_count = 0usize;

    for (rel, (_, entry)) in by_path {
        let declared_rank =
            confidence_rank(entry.confidence.clone().unwrap_or(EntryConfidence::Medium));
        let eff_rank = *effective.get(rel).unwrap_or(&declared_rank);
        let eff_conf = rank_to_confidence(eff_rank);
        let downgraded = eff_rank < declared_rank;
        let review = needs_review.get(rel).copied().unwrap_or(false);
        if downgraded {
            downgraded_count += 1;
        }
        if review {
            needs_review_count += 1;
        }
        let status_after =
            if review { CompileStatus::NeedsRecompile } else { entry.status.clone() };
        entries.push(PropagatedEntry {
            path: rel.clone(),
            declared_confidence: entry.confidence.as_ref().map(confidence_label),
            effective_confidence: confidence_label(&eff_conf).to_string(),
            status_after: status_label(&status_after).to_string(),
            downgraded,
            needs_review: review,
            inbound_sources: inbound_sources.get(rel).cloned().unwrap_or_default(),
        });
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(ConfidencePropagationReport {
        entries_scanned: entries.len(),
        downgraded_count,
        needs_review_count,
        written_count,
        dry_run,
        skipped_write,
        propagation_depth: depth,
        entries,
    })
}

fn write_report_to_wiki(
    knowledge_base: &Path,
    report: &ConfidencePropagationReport,
) -> PdfResult<usize> {
    let wd = wiki_dir(knowledge_base);
    let mut written = 0usize;
    for item in &report.entries {
        if !item.downgraded && !item.needs_review {
            continue;
        }
        let full = wd.join(&item.path);
        let Ok(content) = fs::read_to_string(&full) else {
            continue;
        };
        let Some(mut entry) = KnowledgeEntry::from_markdown(&content) else {
            continue;
        };
        let eff = match item.effective_confidence.as_str() {
            "high" => EntryConfidence::High,
            "low" => EntryConfidence::Low,
            _ => EntryConfidence::Medium,
        };
        entry.confidence = Some(eff);
        if item.needs_review {
            entry.status = CompileStatus::NeedsRecompile;
        }
        let body = extract_markdown_body(&content).unwrap_or("");
        let new_content = entry
            .to_markdown(body)
            .map_err(|e| PdfModuleError::Storage(format!("propagation write yaml: {e}")))?;
        fs::write(&full, new_content)
            .map_err(|e| PdfModuleError::Storage(format!("write: {e}")))?;
        written += 1;
    }
    Ok(written)
}

fn append_propagation_log(
    knowledge_base: &Path,
    report: &ConfidencePropagationReport,
) -> PdfResult<()> {
    let changed: Vec<&str> = report
        .entries
        .iter()
        .filter(|e| e.downgraded || e.needs_review)
        .map(|e| e.path.as_str())
        .take(12)
        .collect();
    let detail = if changed.is_empty() {
        format!(
            "confidence propagation: scanned={}, depth={}, no adjustments",
            report.entries_scanned, report.propagation_depth
        )
    } else {
        format!(
            "confidence propagation: written={}, downgraded={}, needs_review={}, depth={}, paths=[{}]",
            report.written_count,
            report.downgraded_count,
            report.needs_review_count,
            report.propagation_depth,
            changed.join(", ")
        )
    };
    let wiki = WikiStorage::new(knowledge_base)?;
    wiki.append_log(&NervousEvent::new(NervousEventKind::Propagation, detail))
}

fn confidence_label(c: &EntryConfidence) -> String {
    match c {
        EntryConfidence::High => "high",
        EntryConfidence::Medium => "medium",
        EntryConfidence::Low => "low",
    }
    .to_string()
}

fn status_label(s: &CompileStatus) -> &'static str {
    match s {
        CompileStatus::Pending => "pending",
        CompileStatus::Compiling => "compiling",
        CompileStatus::Compiled => "compiled",
        CompileStatus::NeedsRecompile => "needs_recompile",
        CompileStatus::Failed => "failed",
    }
}

fn normalize_path(p: &str) -> String {
    p.trim_start_matches("wiki/").trim_start_matches('/').replace('\\', "/")
}

fn scan_wiki(
    base: &Path,
    dir: &Path,
    out: &mut HashMap<String, (PathBuf, KnowledgeEntry)>,
) -> PdfResult<()> {
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
                out.insert(rel, (path, meta));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_low_source_downgrades_derived() {
        let kb = std::env::temp_dir().join(format!("conf_prop_{}", std::process::id()));
        let wiki = kb.join("wiki/it");
        fs::create_dir_all(&wiki).unwrap();
        let source = r#"---
title: Source
domain: IT
tags: [a]
level: l0
confidence: low
status: compiled
quality_score: 0.5
created: 2026-01-01T00:00:00Z
updated: 2026-01-01T00:00:00Z
---
"#;
        let derived = r#"---
title: Derived
domain: IT
tags: [a]
level: l1
confidence: high
aggregated_from: [it/source.md]
status: compiled
quality_score: 0.8
created: 2026-01-01T00:00:00Z
updated: 2026-01-01T00:00:00Z
---
"#;
        fs::write(wiki.join("source.md"), source).unwrap();
        fs::write(wiki.join("derived.md"), derived).unwrap();

        let mut graph = GraphIndex::new();
        graph.rebuild(&kb.join("wiki")).unwrap();
        let report = compute_propagation(&kb, &graph, 2).unwrap();
        let derived_row = report.entries.iter().find(|e| e.path.ends_with("derived.md")).unwrap();
        assert!(derived_row.downgraded);
        assert_eq!(derived_row.effective_confidence, "low");
        let _ = fs::remove_dir_all(&kb);
    }

    #[test]
    fn test_auto_write_persists_downgraded_confidence() {
        let kb = std::env::temp_dir().join(format!("conf_write_{}", std::process::id()));
        let wiki = kb.join("wiki/it");
        fs::create_dir_all(&wiki).unwrap();
        fs::write(wiki.join("log.md"), "# Build Log\n\n").unwrap();
        let source = r#"---
title: Source
domain: IT
tags: [a]
level: l0
confidence: low
status: compiled
quality_score: 0.5
created: 2026-01-01T00:00:00Z
updated: 2026-01-01T00:00:00Z
---
"#;
        let derived = r#"---
title: Derived
domain: IT
tags: [a]
level: l1
confidence: high
aggregated_from: [it/source.md]
status: compiled
quality_score: 0.8
created: 2026-01-01T00:00:00Z
updated: 2026-01-01T00:00:00Z
---

# Body

---

hr in body
"#;
        fs::write(wiki.join("source.md"), source).unwrap();
        fs::write(wiki.join("derived.md"), derived).unwrap();
        let mut graph = GraphIndex::new();
        graph.rebuild(&kb.join("wiki")).unwrap();
        let report = run_propagation(&kb, &graph, &PropagationPolicy::default()).unwrap();
        assert!(!report.dry_run);
        assert!(report.written_count >= 1);
        let written = fs::read_to_string(wiki.join("derived.md")).unwrap();
        assert!(written.contains("confidence: low"));
        assert!(written.contains("hr in body"));
        let _ = fs::remove_dir_all(&kb);
    }

    #[test]
    fn test_dry_run_writes_nothing() {
        let kb = std::env::temp_dir().join(format!("conf_dry_{}", std::process::id()));
        let wiki = kb.join("wiki");
        fs::create_dir_all(&wiki).unwrap();
        fs::write(wiki.join("log.md"), "# Build Log\n\n").unwrap();
        let md = r#"---
title: T
domain: IT
tags: [a]
level: l1
confidence: high
status: compiled
quality_score: 0.5
created: 2026-01-01T00:00:00Z
updated: 2026-01-01T00:00:00Z
---
body
"#;
        fs::write(wiki.join("t.md"), md).unwrap();
        let mut graph = GraphIndex::new();
        graph.rebuild(&wiki).unwrap();
        let policy = PropagationPolicy::dry_run_only(2);
        let report = run_propagation(&kb, &graph, &policy).unwrap();
        assert!(report.dry_run);
        assert_eq!(report.written_count, 0);
        let content = fs::read_to_string(wiki.join("t.md")).unwrap();
        assert!(content.contains("confidence: high"));
        let _ = fs::remove_dir_all(&kb);
    }
}
