//! Quality analysis for knowledge entries.
//!
//! Detects common issues in the wiki: missing tags, orphan entries,
//! broken links, contradictions, and style drift.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::entry::KnowledgeEntry;
use crate::knowledge::index::vector::{cosine_similarity, EmbeddingModel, TfidfModel};

/// Severity level for quality issues.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARN"),
            Self::Error => write!(f, "ERROR"),
        }
    }
}

/// A single quality issue found during analysis.
#[derive(Debug, Clone)]
pub struct QualityIssue {
    pub severity: IssueSeverity,
    pub entry_path: String,
    pub message: String,
}

impl std::fmt::Display for QualityIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.severity, self.entry_path, self.message)
    }
}

/// A pair of entries that may have style or cognitive drift.
///
/// Detected when entries under the same tag have low cosine similarity
/// despite covering related topics, suggesting divergent writing styles
/// or conceptual evolution over time.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DriftPair {
    pub entry_a: String,
    pub entry_b: String,
    pub title_a: String,
    pub title_b: String,
    /// Cosine similarity of their TF-IDF vectors (lower = more drift).
    pub similarity: f32,
    /// The shared tag that connects them.
    pub shared_tag: String,
}

/// Comprehensive quality report for the wiki.
#[derive(Debug, Clone)]
pub struct QualityReport {
    /// Total entries scanned.
    pub total_entries: usize,
    /// All issues found.
    pub issues: Vec<QualityIssue>,
    /// Entry paths that have no incoming or outgoing links.
    pub orphan_entries: Vec<String>,
    /// Entry paths referenced by `related` or `contradictions` but not found on disk.
    pub broken_links: Vec<String>,
    /// Domains found in the wiki.
    pub domains: HashSet<String>,
    /// Average quality score across entries with score > 0.
    pub avg_quality_score: f32,
    /// Pairs of entries with potential style/cognitive drift.
    pub drift_pairs: Vec<DriftPair>,
}

impl QualityReport {
    /// Check if the report has any errors.
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == IssueSeverity::Error)
    }

    /// Check if the report has any warnings.
    pub fn has_warnings(&self) -> bool {
        self.issues.iter().any(|i| i.severity == IssueSeverity::Warning)
    }

    /// Format the report as Markdown.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Knowledge Quality Report\n\n");
        md.push_str(&format!("- **Total entries**: {}\n", self.total_entries));
        md.push_str(&format!("- **Domains**: {}\n", self.domains.len()));
        md.push_str(&format!(
            "- **Average quality score**: {:.1}%\n",
            self.avg_quality_score * 100.0
        ));
        md.push_str(&format!("- **Issues found**: {}\n", self.issues.len()));
        md.push_str(&format!("- **Orphan entries**: {}\n", self.orphan_entries.len()));
        md.push_str(&format!("- **Broken links**: {}\n\n", self.broken_links.len()));

        if !self.issues.is_empty() {
            md.push_str("## Issues\n\n");
            md.push_str("| Severity | Entry | Issue |\n");
            md.push_str("|----------|-------|-------|\n");
            for issue in &self.issues {
                md.push_str(&format!(
                    "| {} | {} | {} |\n",
                    issue.severity, issue.entry_path, issue.message
                ));
            }
            md.push('\n');
        }

        if !self.orphan_entries.is_empty() {
            md.push_str("## Orphan Entries\n\n");
            md.push_str("These entries have no incoming or outgoing links:\n\n");
            for path in &self.orphan_entries {
                md.push_str(&format!("- `{}`\n", path));
            }
            md.push('\n');
        }

        if !self.broken_links.is_empty() {
            md.push_str("## Broken Links\n\n");
            md.push_str("These paths are referenced but do not exist:\n\n");
            for path in &self.broken_links {
                md.push_str(&format!("- `{}`\n", path));
            }
            md.push('\n');
        }

        if !self.drift_pairs.is_empty() {
            md.push_str("## Style Drift Detection\n\n");
            md.push_str("These entry pairs share a tag but have low content similarity (possible style/cognitive drift):\n\n");
            md.push_str("| Entry A | Entry B | Shared Tag | Similarity |\n");
            md.push_str("|---------|---------|------------|------------|\n");
            for pair in &self.drift_pairs {
                md.push_str(&format!(
                    "| {} | {} | `{}` | {:.2} |\n",
                    pair.title_a, pair.title_b, pair.shared_tag, pair.similarity
                ));
            }
            md.push('\n');
        }

        md
    }
}

/// Scan all wiki entries under `wiki/` and produce a quality report.
pub fn analyze_wiki(wiki_dir: &Path) -> PdfResult<QualityReport> {
    let mut issues = Vec::new();
    let mut entries: Vec<(PathBuf, KnowledgeEntry)> = Vec::new();
    let mut all_paths: HashSet<String> = HashSet::new();
    let mut domains = HashSet::new();
    let mut referenced_paths: HashSet<String> = HashSet::new();

    // Walk wiki/ recursively and parse front matter
    scan_entries_recursive(wiki_dir, wiki_dir, &mut entries, &mut all_paths)?;

    let total_entries = entries.len();

    for (path, entry) in &entries {
        let rel = path.strip_prefix(wiki_dir).unwrap_or(path).to_string_lossy().to_string();

        // Check minimal quality
        if entry.title.is_empty() {
            issues.push(QualityIssue {
                severity: IssueSeverity::Error,
                entry_path: rel.clone(),
                message: "Missing title".into(),
            });
        }
        if entry.domain.is_empty() {
            issues.push(QualityIssue {
                severity: IssueSeverity::Error,
                entry_path: rel.clone(),
                message: "Missing domain".into(),
            });
        }
        if entry.tags.is_empty() {
            issues.push(QualityIssue {
                severity: IssueSeverity::Warning,
                entry_path: rel.clone(),
                message: "No tags assigned".into(),
            });
        }
        if entry.quality_score == 0.0 {
            issues.push(QualityIssue {
                severity: IssueSeverity::Info,
                entry_path: rel.clone(),
                message: "Quality score is 0 (not yet assessed)".into(),
            });
        }

        domains.insert(entry.domain.clone());

        // Collect referenced paths for broken link detection
        for r in &entry.related {
            referenced_paths.insert(r.clone());
        }
        for c in &entry.contradictions {
            referenced_paths.insert(c.clone());
        }
    }

    // Detect broken links
    let mut broken_links = Vec::new();
    for referenced in &referenced_paths {
        // Try to resolve relative to wiki_dir
        let resolved = wiki_dir.join(referenced);
        if !resolved.exists() {
            // Also check if it's a direct path
            if !all_paths.contains(referenced) {
                broken_links.push(referenced.clone());
            }
        }
    }

    // Detect orphan entries (no related links in or out)
    let mut linked_entries: HashSet<String> = HashSet::new();
    for (_, entry) in &entries {
        let rel = entry.relative_path().to_string_lossy().to_string();
        if !entry.related.is_empty() || !entry.contradictions.is_empty() {
            linked_entries.insert(rel.clone());
        }
        for r in &entry.related {
            linked_entries.insert(r.clone());
        }
    }
    let orphan_entries: Vec<String> = entries
        .iter()
        .filter(|(_, entry)| {
            let rel = entry.relative_path().to_string_lossy().to_string();
            !linked_entries.contains(&rel) && entry.level != crate::knowledge::entry::EntryLevel::L0
        })
        .map(|(path, _)| path.strip_prefix(wiki_dir).unwrap_or(path).to_string_lossy().to_string())
        .collect();

    // Average quality score
    let scored: Vec<f32> =
        entries.iter().map(|(_, e)| e.quality_score).filter(|&s| s > 0.0).collect();
    let avg_quality_score =
        if scored.is_empty() { 0.0 } else { scored.iter().sum::<f32>() / scored.len() as f32 };

    Ok(QualityReport {
        total_entries,
        issues,
        orphan_entries,
        broken_links,
        domains,
        avg_quality_score,
        drift_pairs: detect_drift(wiki_dir, &entries),
    })
}

fn scan_entries_recursive(
    base: &Path,
    dir: &Path,
    entries: &mut Vec<(PathBuf, KnowledgeEntry)>,
    all_paths: &mut HashSet<String>,
) -> PdfResult<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)
        .map_err(|e| PdfModuleError::Storage(format!("Failed to read dir: {}", e)))?
    {
        let entry =
            entry.map_err(|e| PdfModuleError::Storage(format!("Failed to read entry: {}", e)))?;
        let path = entry.path();
        if path.is_dir() {
            scan_entries_recursive(base, &path, entries, all_paths)?;
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            let rel = path.strip_prefix(base).unwrap_or(&path).to_string_lossy().to_string();
            all_paths.insert(rel);
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some(entry) = KnowledgeEntry::from_markdown(&content) {
                    entries.push((path, entry));
                }
            }
        }
    }
    Ok(())
}

/// Detect style/cognitive drift between entries that share tags.
///
/// For each tag with >= 2 entries, computes pairwise TF-IDF cosine similarity.
/// Pairs with similarity below `threshold` are flagged as potential drift.
///
/// Uses a simplified TF-IDF model with 256-dimensional feature hashing,
/// trained on the corpus of all entry titles and bodies.
const DRIFT_THRESHOLD: f32 = 0.6;

fn detect_drift(wiki_dir: &Path, entries: &[(PathBuf, KnowledgeEntry)]) -> Vec<DriftPair> {
    if entries.len() < 2 {
        return Vec::new();
    }

    // Build corpus for TF-IDF training
    let corpus: Vec<String> = entries
        .iter()
        .map(|(path, entry)| {
            let body = fs::read_to_string(path).unwrap_or_default();
            let body_text = body.split("---").nth(2).unwrap_or("").to_string();
            format!("{} {}", entry.title, body_text)
        })
        .collect();

    let mut model = TfidfModel::new(256);
    model.train(&corpus);

    // Embed each entry
    let embeddings: Vec<Vec<f32>> = corpus.iter().map(|doc| model.embed(doc)).collect();

    // Build tag → entry indices mapping
    let mut tag_entries: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, (_, entry)) in entries.iter().enumerate() {
        for tag in &entry.tags {
            tag_entries.entry(tag.clone()).or_default().push(idx);
        }
    }

    let mut drift_pairs = Vec::new();
    let mut seen_pairs = HashSet::new();

    for (tag, indices) in &tag_entries {
        if indices.len() < 2 {
            continue;
        }
        // Only check entries with large time spans (> 90 days)
        for i in 0..indices.len() {
            for j in (i + 1)..indices.len() {
                let idx_a = indices[i];
                let idx_b = indices[j];
                let (_, entry_a) = &entries[idx_a];
                let (_, entry_b) = &entries[idx_b];

                let time_span = (entry_b.created - entry_a.created).num_days().unsigned_abs();
                if time_span < 90 {
                    continue;
                }

                let pair_key = {
                    let a = entries[idx_a].0.to_string_lossy().to_string();
                    let b = entries[idx_b].0.to_string_lossy().to_string();
                    if a <= b {
                        format!("{}↔{}", a, b)
                    } else {
                        format!("{}↔{}", b, a)
                    }
                };
                if !seen_pairs.insert(pair_key) {
                    continue;
                }

                let sim = cosine_similarity(&embeddings[idx_a], &embeddings[idx_b]);
                if sim < DRIFT_THRESHOLD {
                    let rel_a = entries[idx_a]
                        .0
                        .strip_prefix(wiki_dir)
                        .unwrap_or(&entries[idx_a].0)
                        .to_string_lossy()
                        .to_string();
                    let rel_b = entries[idx_b]
                        .0
                        .strip_prefix(wiki_dir)
                        .unwrap_or(&entries[idx_b].0)
                        .to_string_lossy()
                        .to_string();

                    drift_pairs.push(DriftPair {
                        entry_a: rel_a,
                        entry_b: rel_b,
                        title_a: entry_a.title.clone(),
                        title_b: entry_b.title.clone(),
                        similarity: sim,
                        shared_tag: tag.clone(),
                    });
                }
            }
        }
    }

    // Sort by similarity ascending (worst drift first)
    drift_pairs.sort_by(|a, b| {
        a.similarity.partial_cmp(&b.similarity).unwrap_or(std::cmp::Ordering::Equal)
    });

    drift_pairs
}

/// Suggested MCP tool calls for an agent to resolve quality findings.
pub fn build_next_actions(report: &QualityReport, knowledge_base: &str) -> Vec<serde_json::Value> {
    let mut actions = Vec::new();
    let kb = knowledge_base;

    for orphan in report.orphan_entries.iter().take(5) {
        actions.push(serde_json::json!({
            "tool": "suggest_links",
            "args": { "knowledge_base": kb, "entry_path": orphan, "top_k": 5 },
            "reason": "Orphan entry — discover links to integrate into the graph"
        }));
    }

    if !report.broken_links.is_empty() {
        actions.push(serde_json::json!({
            "tool": "patch_wiki_entry",
            "args": {
                "knowledge_base": kb,
                "entry_path": "<entry_with_broken_related>",
                "operations": [{ "type": "replace_front_matter", "related": [] }]
            },
            "reason": "Fix broken related/contradiction paths in front matter"
        }));
    }

    actions.push(serde_json::json!({
        "tool": "hypothesis_test",
        "args": { "knowledge_base": kb },
        "reason": "Review explicit contradiction pairs and resolve or document open questions"
    }));

    if report.drift_pairs.len() > 2 {
        actions.push(serde_json::json!({
            "tool": "recompile_entry",
            "args": { "knowledge_base": kb, "entry_path": report.drift_pairs[0].entry_a },
            "reason": "Style/cognitive drift detected — recompile or harmonize wording"
        }));
    }

    actions.push(serde_json::json!({
        "tool": "rebuild_index",
        "args": { "knowledge_base": kb },
        "reason": "Refresh Tantivy, vector, and graph indexes after quality fixes"
    }));

    actions
}
