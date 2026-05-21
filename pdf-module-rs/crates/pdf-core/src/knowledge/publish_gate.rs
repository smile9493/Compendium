//! Publish gate: quality thresholds and publish_status on wiki entries.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::entry::{
    CompileStatus, KnowledgeEntry, PublishStatus, extract_markdown_body,
};
use crate::knowledge::quality::{IssueSeverity, QualityReport, analyze_wiki};
use crate::management::config_manager::ConfigManager;

pub const KEY_QUALITY_MIN_SCORE: &str = "quality_min_score";
pub const KEY_GATE_BLOCK_ON_ERRORS: &str = "gate_block_on_error_issues";
pub const KEY_AUTO_PUBLISH: &str = "auto_publish_on_gate_pass";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateConfig {
    pub quality_min_score: f32,
    pub gate_block_on_error_issues: bool,
    pub auto_publish_on_gate_pass: bool,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            quality_min_score: 0.7,
            gate_block_on_error_issues: true,
            auto_publish_on_gate_pass: false,
        }
    }
}

impl GateConfig {
    pub fn load(knowledge_base: &Path) -> PdfResult<Self> {
        let mut cm = ConfigManager::new(knowledge_base);
        cm.load()?;
        Ok(Self {
            quality_min_score: cm
                .get(KEY_QUALITY_MIN_SCORE)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.7),
            gate_block_on_error_issues: cm
                .get(KEY_GATE_BLOCK_ON_ERRORS)
                .map(|s| s != "false" && s != "0")
                .unwrap_or(true),
            auto_publish_on_gate_pass: cm
                .get(KEY_AUTO_PUBLISH)
                .map(|s| s == "true" || s == "1")
                .unwrap_or(false),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GateResult {
    pub published_count: usize,
    pub blocked_count: usize,
    pub draft_count: usize,
    pub entries_scanned: usize,
}

/// Whether an entry should appear in searchable indexes.
pub fn is_searchable(entry: &KnowledgeEntry, min_score: f32) -> bool {
    entry.publish_status == PublishStatus::Published && entry.quality_score >= min_score
}

/// Run quality analysis and update `publish_status` on each wiki entry.
pub fn apply_publish_gate(knowledge_base: &Path) -> PdfResult<GateResult> {
    let config = GateConfig::load(knowledge_base)?;
    let wiki_dir = knowledge_base.join("wiki");
    if !wiki_dir.exists() {
        return Ok(GateResult::default());
    }

    let report = analyze_wiki(&wiki_dir)?;
    let error_paths: std::collections::HashSet<String> = report
        .issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .map(|i| i.entry_path.clone())
        .collect();

    let mut result = GateResult { entries_scanned: report.total_entries, ..Default::default() };

    scan_and_apply(&wiki_dir, &wiki_dir, &config, &report, &error_paths, &mut result)?;
    Ok(result)
}

#[allow(clippy::only_used_in_recursion)]
fn scan_and_apply(
    base: &Path,
    dir: &Path,
    config: &GateConfig,
    report: &QualityReport,
    error_paths: &std::collections::HashSet<String>,
    result: &mut GateResult,
) -> PdfResult<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| PdfModuleError::Storage(e.to_string()))? {
        let entry = entry.map_err(|e| PdfModuleError::Storage(e.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            scan_and_apply(base, &path, config, report, error_paths, result)?;
        } else if path.extension().is_some_and(|e| e == "md") {
            let rel = path.strip_prefix(base).unwrap_or(&path).to_string_lossy().replace('\\', "/");
            let content =
                fs::read_to_string(&path).map_err(|e| PdfModuleError::Storage(e.to_string()))?;
            let Some(mut meta) = KnowledgeEntry::from_markdown(&content) else {
                continue;
            };

            let has_error = config.gate_block_on_error_issues && error_paths.contains(&rel);
            let low_score =
                meta.quality_score > 0.0 && meta.quality_score < config.quality_min_score;
            let zero_score_uncompiled =
                meta.quality_score == 0.0 && meta.status != CompileStatus::Compiled;

            let new_status = if has_error || low_score || zero_score_uncompiled {
                PublishStatus::Blocked
            } else if config.auto_publish_on_gate_pass
                || (meta.status == CompileStatus::Compiled
                    && meta.quality_score >= config.quality_min_score)
            {
                PublishStatus::Published
            } else {
                PublishStatus::Draft
            };

            if new_status != meta.publish_status {
                meta.publish_status = new_status;
                meta.touch();
                let body = extract_markdown_body(&content).unwrap_or("").trim_start();
                let new_content = meta.to_markdown(body)?;
                fs::write(&path, new_content)
                    .map_err(|e| PdfModuleError::Storage(e.to_string()))?;
                debug!(path = %rel, status = ?new_status, "Updated publish_status");
            }

            match meta.publish_status {
                PublishStatus::Published => result.published_count += 1,
                PublishStatus::Blocked => result.blocked_count += 1,
                PublishStatus::Draft => result.draft_count += 1,
            }
        }
    }
    Ok(())
}
