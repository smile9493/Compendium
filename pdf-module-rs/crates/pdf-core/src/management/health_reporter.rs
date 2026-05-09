//! Health reporter — aggregates data from quality analysis, fulltext index,
//! and knowledge graph into a single `HealthReport`.

use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::error::PdfResult;
use crate::knowledge::quality;
use crate::management::types::HealthReport;

/// Generates a `HealthReport` for a knowledge base directory.
pub struct HealthReporter {
    kb_path: PathBuf,
}

impl HealthReporter {
    /// Create a reporter for the given knowledge base root.
    pub fn new(kb_path: &Path) -> Self {
        Self {
            kb_path: kb_path.to_path_buf(),
        }
    }

    /// Generate the health report.
    ///
    /// Scans the wiki directory for quality analysis, opens the fulltext index
    /// for size statistics, and reads the graph index for topology info.
    /// Individual subsystem failures are tolerated — partial data is still returned.
    pub fn report(&self) -> PdfResult<HealthReport> {
        let wiki_dir = self.kb_path.join("wiki");

        // Quality analysis (tolerate missing wiki dir)
        let quality_report = if wiki_dir.exists() {
            quality::analyze_wiki(&wiki_dir).unwrap_or_else(|_| quality::QualityReport {
                total_entries: 0,
                issues: vec![],
                orphan_entries: vec![],
                broken_links: vec![],
                domains: Default::default(),
                avg_quality_score: 0.0,
                drift_pairs: vec![],
            })
        } else {
            quality::QualityReport {
                total_entries: 0,
                issues: vec![],
                orphan_entries: vec![],
                broken_links: vec![],
                domains: Default::default(),
                avg_quality_score: 0.0,
                drift_pairs: vec![],
            }
        };

        // Contradiction count from quality issues
        let contradiction_count = quality_report
            .issues
            .iter()
            .filter(|i| i.message.to_lowercase().contains("contradiction"))
            .count();

        // Fulltext index size
        let index_size_bytes = self.measure_index_size();

        // Graph statistics
        let (graph_node_count, graph_edge_count) = self.graph_stats(&wiki_dir);

        // Last compile time from the compile status file
        let last_compile = self.read_last_compile_time();

        Ok(HealthReport {
            total_entries: quality_report.total_entries,
            orphan_count: quality_report.orphan_entries.len(),
            contradiction_count,
            broken_link_count: quality_report.broken_links.len(),
            index_size_bytes,
            graph_node_count,
            graph_edge_count,
            avg_quality_score: quality_report.avg_quality_score,
            domains: quality_report.domains.into_iter().collect(),
            last_compile,
            generated_at: Utc::now(),
        })
    }

    /// Measure the total size of the `.rsut_index/` directory in bytes.
    fn measure_index_size(&self) -> u64 {
        let index_dir = self.kb_path.join(".rsut_index");
        if !index_dir.exists() {
            return 0;
        }
        dir_size(&index_dir)
    }

    /// Read graph statistics by rebuilding in-memory (lightweight for small graphs).
    fn graph_stats(&self, wiki_dir: &Path) -> (usize, usize) {
        if !wiki_dir.exists() {
            return (0, 0);
        }
        let mut graph = crate::knowledge::GraphIndex::new();
        match graph.rebuild(wiki_dir) {
            Ok(node_count) => (node_count, graph.edge_count()),
            Err(_) => (0, 0),
        }
    }

    /// Read the last compile timestamp from the status file.
    fn read_last_compile_time(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        let status_path = self.kb_path.join(".rsut_index").join("compile_status.json");
        if !status_path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&status_path).ok()?;
        let record: serde_json::Value = serde_json::from_str(&content).ok()?;
        record
            .get("last_finished")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
    }
}

/// Recursively sum the size of all files in a directory.
fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else if let Ok(meta) = p.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_report_empty_kb() {
        let dir = TempDir::new().expect("tmpdir");
        let reporter = HealthReporter::new(dir.path());
        let report = reporter.report().expect("report");
        assert_eq!(report.total_entries, 0);
        assert_eq!(report.orphan_count, 0);
        assert_eq!(report.index_size_bytes, 0);
    }
}
