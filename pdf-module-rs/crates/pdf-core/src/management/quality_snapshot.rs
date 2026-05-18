//! Persisted quality scan snapshot at `.rsut_index/quality_snapshot.json`.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::publish_gate::{apply_publish_gate, GateResult};
use crate::knowledge::quality::{analyze_wiki, QualityReport};

/// Summary issue for agents and UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIssueBrief {
    pub severity: String,
    pub entry_path: String,
    pub message: String,
}

/// Quality scan persisted after compile or manual refresh.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QualitySnapshot {
    pub scanned_at: Option<String>,
    pub issues_count: usize,
    pub orphan_count: usize,
    pub contradiction_pairs: usize,
    pub drift_pairs: usize,
    pub broken_links_count: usize,
    pub published_count: usize,
    pub blocked_count: usize,
    pub draft_count: usize,
    pub top_issues: Vec<QualityIssueBrief>,
}

#[derive(Clone)]
pub struct QualitySnapshotStore {
    path: PathBuf,
}

impl QualitySnapshotStore {
    pub fn new(knowledge_base: &Path) -> Self {
        Self { path: knowledge_base.join(".rsut_index").join("quality_snapshot.json") }
    }

    pub fn read(&self) -> PdfResult<QualitySnapshot> {
        if !self.path.exists() {
            return Ok(QualitySnapshot::default());
        }
        let content = fs::read_to_string(&self.path).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to read quality snapshot: {e}"))
        })?;
        match serde_json::from_str(&content) {
            Ok(s) => Ok(s),
            Err(e) => {
                warn!(error = %e, "Invalid quality_snapshot.json");
                Ok(QualitySnapshot::default())
            }
        }
    }

    pub fn write(&self, snapshot: &QualitySnapshot) -> PdfResult<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                PdfModuleError::Storage(format!("Failed to create snapshot dir: {e}"))
            })?;
        }
        let json = serde_json::to_string_pretty(snapshot).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to serialize quality snapshot: {e}"))
        })?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to write quality snapshot: {e}"))
        })?;
        fs::rename(&tmp, &self.path).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to commit quality snapshot: {e}"))
        })?;
        Ok(())
    }
}

/// Run a full wiki quality scan and persist the snapshot.
pub fn refresh_quality_snapshot(knowledge_base: &Path) -> PdfResult<QualitySnapshot> {
    let wiki_dir = knowledge_base.join("wiki");
    if !wiki_dir.exists() {
        let empty = QualitySnapshot::default();
        QualitySnapshotStore::new(knowledge_base).write(&empty)?;
        return Ok(empty);
    }

    let report = analyze_wiki(&wiki_dir)?;
    let gate = apply_publish_gate(knowledge_base).unwrap_or_default();
    let snapshot = snapshot_from_report(&report, &wiki_dir, &gate);
    QualitySnapshotStore::new(knowledge_base).write(&snapshot)?;
    Ok(snapshot)
}

pub fn snapshot_from_report(
    report: &QualityReport,
    wiki_dir: &Path,
    gate: &GateResult,
) -> QualitySnapshot {
    let mut top_issues: Vec<QualityIssueBrief> = report
        .issues
        .iter()
        .take(20)
        .map(|i| QualityIssueBrief {
            severity: i.severity.to_string(),
            entry_path: i.entry_path.clone(),
            message: i.message.clone(),
        })
        .collect();

    top_issues.sort_by(|a, b| a.severity.cmp(&b.severity));

    QualitySnapshot {
        scanned_at: Some(Utc::now().to_rfc3339()),
        issues_count: report.issues.len(),
        orphan_count: report.orphan_entries.len(),
        contradiction_pairs: count_contradiction_pairs(wiki_dir),
        drift_pairs: report.drift_pairs.len(),
        broken_links_count: report.broken_links.len(),
        published_count: gate.published_count,
        blocked_count: gate.blocked_count,
        draft_count: gate.draft_count,
        top_issues,
    }
}

/// Count unique explicit contradiction links in front matter.
pub fn count_contradiction_pairs(wiki_dir: &Path) -> usize {
    let mut seen = HashSet::new();
    let mut count = 0usize;
    scan_contradictions(wiki_dir, wiki_dir, &mut seen, &mut count);
    count
}

#[allow(clippy::only_used_in_recursion)]
fn scan_contradictions(base: &Path, dir: &Path, seen: &mut HashSet<String>, count: &mut usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_contradictions(base, &path, seen, count);
        } else if path.extension().is_none_or(|e| e == "md") {
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            if let Some(entry) = crate::knowledge::entry::KnowledgeEntry::from_markdown(&content) {
                let rel = path.strip_prefix(base).unwrap_or(&path).to_string_lossy().to_string();
                for contra in &entry.contradictions {
                    let mut pair_key = [rel.clone(), contra.clone()];
                    pair_key.sort();
                    let key = pair_key.join("↔");
                    if seen.insert(key) {
                        *count += 1;
                    }
                }
            }
        }
    }
}
