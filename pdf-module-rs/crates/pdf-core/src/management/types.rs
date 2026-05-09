//! Shared types for the management layer.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Health report for a knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Total wiki entries.
    pub total_entries: usize,
    /// Entries with no incoming or outgoing links.
    pub orphan_count: usize,
    /// Entries that explicitly contradict each other.
    pub contradiction_count: usize,
    /// Broken cross-references.
    pub broken_link_count: usize,
    /// Total size of the Tantivy fulltext index in bytes.
    pub index_size_bytes: u64,
    /// Number of nodes in the knowledge graph.
    pub graph_node_count: usize,
    /// Number of edges in the knowledge graph.
    pub graph_edge_count: usize,
    /// Average quality score (0.0–1.0).
    pub avg_quality_score: f32,
    /// Domains present in the wiki.
    pub domains: Vec<String>,
    /// Timestamp of the last successful compilation, if any.
    pub last_compile: Option<DateTime<Utc>>,
    /// Timestamp of this report generation.
    pub generated_at: DateTime<Utc>,
}

impl fmt::Display for HealthReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Knowledge Base Health Report")?;
        writeln!(f, "───────────────────────────")?;
        writeln!(f, "Total entries:        {}", self.total_entries)?;
        writeln!(f, "Orphan entries:       {}", self.orphan_count)?;
        writeln!(f, "Contradictions:       {}", self.contradiction_count)?;
        writeln!(f, "Broken links:         {}", self.broken_link_count)?;
        writeln!(
            f,
            "Index size:           {} MB",
            self.index_size_bytes / 1024 / 1024
        )?;
        writeln!(
            f,
            "Graph:                {} nodes, {} edges",
            self.graph_node_count, self.graph_edge_count
        )?;
        writeln!(
            f,
            "Avg quality score:    {:.1}%",
            self.avg_quality_score * 100.0
        )?;
        writeln!(f, "Domains:              {}", self.domains.join(", "))?;
        writeln!(
            f,
            "Last compile:         {}",
            self.last_compile.map_or("never".to_string(), |t| t
                .format("%Y-%m-%d %H:%M")
                .to_string())
        )?;
        writeln!(
            f,
            "Generated at:         {}",
            self.generated_at.format("%Y-%m-%d %H:%M:%S")
        )?;
        Ok(())
    }
}

/// Record of a compilation event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileStatusRecord {
    /// Whether a compile is currently running.
    pub running: bool,
    /// Timestamp of the last compile start.
    pub last_started: Option<DateTime<Utc>>,
    /// Timestamp of the last compile completion.
    pub last_finished: Option<DateTime<Utc>>,
    /// Duration of the last compile in milliseconds.
    pub last_duration_ms: Option<u64>,
    /// Outcome: "success", "partial", or "error".
    pub last_outcome: Option<String>,
    /// Human-readable status message.
    pub message: String,
    /// Recent compile history (most recent first, max 10).
    pub history: Vec<CompileHistoryEntry>,
}

/// A single entry in compile history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileHistoryEntry {
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub outcome: String,
    pub entries_compiled: usize,
    pub entries_skipped: usize,
}

impl fmt::Display for CompileStatusRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Compile Status")?;
        writeln!(f, "──────────────")?;
        writeln!(
            f,
            "Running:      {}",
            if self.running { "yes" } else { "no" }
        )?;
        writeln!(
            f,
            "Last started: {}",
            self.last_started.map_or("never".to_string(), |t| t
                .format("%Y-%m-%d %H:%M")
                .to_string())
        )?;
        writeln!(
            f,
            "Last finished:{}",
            self.last_finished.map_or(" never".to_string(), |t| format!(
                " {}",
                t.format("%Y-%m-%d %H:%M")
            ))
        )?;
        if let Some(ms) = self.last_duration_ms {
            writeln!(f, "Duration:     {} ms", ms)?;
        }
        writeln!(
            f,
            "Outcome:      {}",
            self.last_outcome.as_deref().unwrap_or("n/a")
        )?;
        writeln!(f, "Message:      {}", self.message)?;
        if !self.history.is_empty() {
            writeln!(f, "\nRecent history:")?;
            for entry in &self.history {
                writeln!(
                    f,
                    "  {} → {} ({} ms) [{}] compiled={}, skipped={}",
                    entry.started_at.format("%m-%d %H:%M"),
                    entry.finished_at.format("%H:%M"),
                    entry.duration_ms,
                    entry.outcome,
                    entry.entries_compiled,
                    entry.entries_skipped,
                )?;
            }
        }
        Ok(())
    }
}
