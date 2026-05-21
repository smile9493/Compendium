//! Knowledge decay and staleness detection (Karpathy maintenance loop).

use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};

use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::entry::KnowledgeEntry;

/// Default age threshold when callers omit `max_age_days`.
pub const DEFAULT_STALE_DAYS: u32 = 90;

/// Half-life for exponential time decay (days).
pub const DECAY_HALF_LIFE_DAYS: i64 = 180;

/// One wiki entry flagged as potentially stale.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StaleEntry {
    pub path: String,
    pub title: String,
    pub days_since_update: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_since_validated: Option<i64>,
    pub quality_score: f32,
    pub decay_score: f32,
}

/// Exponential time decay: `0.5^(days / half_life)`.
pub fn time_decay_factor(days: i64, half_life_days: i64) -> f32 {
    if days <= 0 {
        return 1.0;
    }
    let half = half_life_days.max(1) as f64;
    (0.5f64).powf(days as f64 / half) as f32
}

/// Combined quality × time decay score.
pub fn decay_score(quality_score: f32, days_stale: i64) -> f32 {
    (quality_score.clamp(0.0, 1.0) * time_decay_factor(days_stale, DECAY_HALF_LIFE_DAYS))
        .clamp(0.0, 1.0)
}

fn days_since(dt: DateTime<Utc>) -> i64 {
    let now = Utc::now();
    (now - dt).num_days()
}

/// Scan `wiki/` for entries whose `updated` or `last_validated` exceeds `max_age_days`.
pub fn detect_stale_entries(
    knowledge_base: &Path,
    max_age_days: u32,
) -> PdfResult<Vec<StaleEntry>> {
    let wiki_dir = knowledge_base.join("wiki");
    if !wiki_dir.exists() {
        return Ok(Vec::new());
    }

    let threshold = i64::from(max_age_days);
    let mut stale = Vec::new();
    scan_stale(&wiki_dir, &wiki_dir, threshold, &mut stale)?;
    stale.sort_by(|a, b| {
        b.days_since_update.cmp(&a.days_since_update).then_with(|| a.path.cmp(&b.path))
    });
    Ok(stale)
}

fn scan_stale(
    base: &Path,
    dir: &Path,
    threshold_days: i64,
    out: &mut Vec<StaleEntry>,
) -> PdfResult<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)
        .map_err(|e| PdfModuleError::Storage(format!("read dir: {}", e)))?
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "index.md" || name == "log.md" {
            continue;
        }
        if path.is_dir() {
            if name != ".versions" {
                scan_stale(base, &path, threshold_days, out)?;
            }
            continue;
        }
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let rel = path.strip_prefix(base).unwrap_or(&path).to_string_lossy().to_string();
        let content = fs::read_to_string(&path)
            .map_err(|e| PdfModuleError::Storage(format!("read {}: {}", rel, e)))?;
        let Some(meta) = KnowledgeEntry::from_markdown(&content) else {
            continue;
        };

        let days_updated = days_since(meta.updated);
        let days_validated = meta.last_validated.map(days_since);
        let reference_days = days_validated.unwrap_or(days_updated);
        if reference_days < threshold_days {
            continue;
        }

        out.push(StaleEntry {
            path: rel,
            title: meta.title,
            days_since_update: days_updated,
            days_since_validated: days_validated,
            quality_score: meta.quality_score,
            decay_score: decay_score(meta.quality_score, reference_days),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_decay_at_half_life_is_half() {
        let f = time_decay_factor(DECAY_HALF_LIFE_DAYS, DECAY_HALF_LIFE_DAYS);
        assert!((f - 0.5).abs() < 0.01);
    }

    #[test]
    fn decay_score_blends_quality_and_time() {
        let s = decay_score(1.0, 0);
        assert!((s - 1.0).abs() < 0.001);
        let s2 = decay_score(0.8, DECAY_HALF_LIFE_DAYS);
        assert!((s2 - 0.4).abs() < 0.05);
    }
}
