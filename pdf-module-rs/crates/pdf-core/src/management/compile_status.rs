//! Atomic read/write for `.rsut_index/compile_status.json`.
//!
//! Single source of truth for compile lifecycle across MCP, HTTP, and health reporting.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::Utc;
use tracing::warn;

use crate::error::{PdfModuleError, PdfResult};
use crate::management::types::{CompileHistoryEntry, CompileStatusRecord};

const MAX_HISTORY: usize = 10;

/// Manages compile status persistence for a knowledge base.
#[derive(Clone)]
pub struct CompileStatusStore {
    path: PathBuf,
}

/// Outcome statistics for a finished compile run.
#[derive(Debug, Clone, Default)]
pub struct CompileFinishStats {
    pub entries_compiled: usize,
    pub entries_skipped: usize,
}

/// RAII guard for an in-flight compile; finish via [`CompileGuard::finish_success`] or [`CompileGuard::finish_error`].
pub struct CompileGuard {
    store: CompileStatusStore,
    started: Instant,
}

impl CompileStatusStore {
    pub fn new(knowledge_base: &Path) -> Self {
        Self { path: knowledge_base.join(".rsut_index").join("compile_status.json") }
    }

    /// Read current status, or default when missing or corrupt.
    pub fn read(&self) -> PdfResult<CompileStatusRecord> {
        if !self.path.exists() {
            return Ok(default_record());
        }

        let content = fs::read_to_string(&self.path)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to read compile status: {e}")))?;

        match serde_json::from_str::<CompileStatusRecord>(&content) {
            Ok(record) => Ok(record),
            Err(e) => {
                warn!(error = %e, "Invalid compile_status.json, returning default");
                Ok(default_record())
            }
        }
    }

    /// Mark a compile as started (`running: true`).
    pub fn begin_compile(&self) -> PdfResult<CompileGuard> {
        let mut record = self.read().unwrap_or_else(|_| default_record());
        record.running = true;
        record.last_started = Some(Utc::now());
        record.message = "Compile in progress.".to_string();
        self.write_atomic(&record)?;

        Ok(CompileGuard { store: self.clone(), started: Instant::now() })
    }

    /// Write the full status record (used by compile job store).
    pub fn write_record(&self, record: &CompileStatusRecord) -> PdfResult<()> {
        self.write_atomic(record)
    }

    fn write_atomic(&self, record: &CompileStatusRecord) -> PdfResult<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| PdfModuleError::Storage(format!("Failed to create index dir: {e}")))?;
        }

        let json = serde_json::to_string_pretty(record).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to serialize compile status: {e}"))
        })?;

        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, &json)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to write compile status: {e}")))?;
        fs::rename(&tmp, &self.path).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to commit compile status: {e}"))
        })?;

        Ok(())
    }
}

impl CompileGuard {
    pub fn finish_success(self, stats: CompileFinishStats) -> PdfResult<()> {
        self.finish_with_outcome("success", None, stats)
    }

    pub fn finish_error(self, message: impl Into<String>) -> PdfResult<()> {
        self.finish_with_outcome("error", Some(message.into()), CompileFinishStats::default())
    }

    fn finish_with_outcome(
        self,
        outcome: &str,
        error_message: Option<String>,
        stats: CompileFinishStats,
    ) -> PdfResult<()> {
        let duration_ms = self.started.elapsed().as_millis() as u64;
        let mut record = self.store.read().unwrap_or_else(|_| default_record());
        let finished = Utc::now();
        let started_at = record.last_started.unwrap_or(finished);

        record.running = false;
        record.last_finished = Some(finished);
        record.last_duration_ms = Some(duration_ms);
        record.last_outcome = Some(outcome.to_string());
        record.message = error_message.unwrap_or_else(|| {
            format!(
                "Compile finished: {} compiled, {} skipped.",
                stats.entries_compiled, stats.entries_skipped
            )
        });

        let history_entry = CompileHistoryEntry {
            started_at,
            finished_at: finished,
            duration_ms,
            outcome: outcome.to_string(),
            entries_compiled: stats.entries_compiled,
            entries_skipped: stats.entries_skipped,
            job_id: record.active_job_id.clone(),
        };
        record.history.insert(0, history_entry);
        record.history.truncate(MAX_HISTORY);

        self.store.write_atomic(&record)
    }
}

fn default_record() -> CompileStatusRecord {
    CompileStatusRecord {
        running: false,
        last_started: None,
        last_finished: None,
        last_duration_ms: None,
        last_outcome: None,
        message: "No compile has been performed yet.".to_string(),
        history: Vec::new(),
        active_job_id: None,
        pipeline_status: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_begin_and_finish_success() {
        let dir = tempfile::tempdir().unwrap();
        let store = CompileStatusStore::new(dir.path());

        let guard = store.begin_compile().unwrap();
        let reading = store.read().unwrap();
        assert!(reading.running);

        guard
            .finish_success(CompileFinishStats { entries_compiled: 3, entries_skipped: 1 })
            .unwrap();

        let done = store.read().unwrap();
        assert!(!done.running);
        assert_eq!(done.last_outcome.as_deref(), Some("success"));
        assert_eq!(done.history.len(), 1);
        assert_eq!(done.history[0].entries_compiled, 3);
    }
}
