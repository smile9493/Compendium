//! Compile job tracking with staged pipeline observability.
//!
//! Persists full jobs under `.rsut_index/compile_jobs/{job_id}.json` and keeps
//! a compatibility summary in `compile_status.json`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::error::{PdfModuleError, PdfResult};
use crate::management::compile_status::CompileStatusStore;
use crate::management::types::{CompileHistoryEntry, CompileStatusRecord};

const MAX_HISTORY: usize = 10;

/// What initiated the compile job.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CompileTrigger {
    SinglePdf,
    Incremental,
    Recompile,
    PlanTask,
}

/// End-to-end pipeline status (distinct from per-entry [`crate::knowledge::entry::CompileStatus`]).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PipelineStatus {
    #[default]
    Running,
    AwaitingAgent,
    Completed,
    Failed,
    Partial,
}

/// Pipeline stages in execution order.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CompileStage {
    Extract,
    PromptGen,
    AgentWiki,
    IndexRebuild,
    QualityGate,
}

impl CompileStage {
    pub const ALL: [CompileStage; 5] = [
        CompileStage::Extract,
        CompileStage::PromptGen,
        CompileStage::AgentWiki,
        CompileStage::IndexRebuild,
        CompileStage::QualityGate,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Extract => "extract",
            Self::PromptGen => "prompt_gen",
            Self::AgentWiki => "agent_wiki",
            Self::IndexRebuild => "index_rebuild",
            Self::QualityGate => "quality_gate",
        }
    }
}

/// Status of a single pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum StageStatus {
    #[default]
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
}

/// One stage record within a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileStageRecord {
    pub stage: CompileStage,
    pub status: StageStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub retryable: bool,
}

impl CompileStageRecord {
    fn new(stage: CompileStage, retryable: bool) -> Self {
        Self {
            stage,
            status: StageStatus::Pending,
            started_at: None,
            finished_at: None,
            duration_ms: None,
            error: None,
            retryable,
        }
    }
}

fn default_stages() -> Vec<CompileStageRecord> {
    CompileStage::ALL
        .into_iter()
        .map(|s| {
            let retryable = matches!(s, CompileStage::IndexRebuild | CompileStage::QualityGate);
            CompileStageRecord::new(s, retryable)
        })
        .collect()
}

/// Paths and references produced during compilation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompileArtifacts {
    #[serde(default)]
    pub raw_paths: Vec<String>,
    #[serde(default)]
    pub prompt_paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_path: Option<String>,
}

/// Counters updated as the job progresses.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompileJobStats {
    pub entries_expected: usize,
    pub entries_saved: usize,
    pub entries_blocked: usize,
}

/// Full compile job persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileJob {
    pub job_id: String,
    pub trigger: CompileTrigger,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pipeline_status: PipelineStatus,
    pub stages: Vec<CompileStageRecord>,
    pub artifacts: CompileArtifacts,
    pub stats: CompileJobStats,
    #[serde(default)]
    pub saved_entry_paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl CompileJob {
    fn new(trigger: CompileTrigger) -> Self {
        let now = Utc::now();
        Self {
            job_id: Uuid::new_v4().to_string(),
            trigger,
            created_at: now,
            updated_at: now,
            pipeline_status: PipelineStatus::Running,
            stages: default_stages(),
            artifacts: CompileArtifacts::default(),
            stats: CompileJobStats::default(),
            saved_entry_paths: Vec::new(),
            message: None,
        }
    }

    fn stage_mut(&mut self, stage: CompileStage) -> PdfResult<&mut CompileStageRecord> {
        self.stages
            .iter_mut()
            .find(|s| s.stage == stage)
            .ok_or_else(|| PdfModuleError::Storage(format!("Unknown stage: {:?}", stage)))
    }

    pub fn current_stage(&self) -> Option<CompileStage> {
        self.stages.iter().find_map(|s| {
            if matches!(s.status, StageStatus::Running) {
                Some(s.stage)
            } else {
                None
            }
        })
    }

    pub fn stage_by_kind(&self, stage: CompileStage) -> Option<&CompileStageRecord> {
        self.stages.iter().find(|s| s.stage == stage)
    }
}

/// Unified API response for MCP, HTTP, and Web UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileJobView {
    pub running: bool,
    pub pipeline_status: Option<String>,
    pub active_job_id: Option<String>,
    pub job: Option<CompileJob>,
    pub last_started: Option<DateTime<Utc>>,
    pub last_finished: Option<DateTime<Utc>>,
    pub last_duration_ms: Option<u64>,
    pub last_outcome: Option<String>,
    pub message: String,
    pub history: Vec<CompileHistoryEntry>,
}

/// Manages compile job persistence for a knowledge base.
#[derive(Clone)]
pub struct CompileJobStore {
    knowledge_base: PathBuf,
    jobs_dir: PathBuf,
    status_store: CompileStatusStore,
}

impl CompileJobStore {
    pub fn new(knowledge_base: &Path) -> Self {
        let kb = knowledge_base.to_path_buf();
        Self {
            jobs_dir: kb.join(".rsut_index").join("compile_jobs"),
            status_store: CompileStatusStore::new(knowledge_base),
            knowledge_base: kb,
        }
    }

    pub fn knowledge_base(&self) -> &Path {
        &self.knowledge_base
    }

    pub fn begin_job(&self, trigger: CompileTrigger) -> PdfResult<CompileJob> {
        let mut job = CompileJob::new(trigger);
        job.stats.entries_expected = 1;
        self.write_job(&job)?;
        self.sync_status_summary(&job, true)?;
        Ok(job)
    }

    pub fn load_job(&self, job_id: &str) -> PdfResult<CompileJob> {
        let path = self.job_path(job_id);
        if !path.exists() {
            return Err(PdfModuleError::FileNotFound(format!("Compile job not found: {job_id}")));
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to read compile job: {e}")))?;
        serde_json::from_str(&content)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to parse compile job: {e}")))
    }

    pub fn active_job_id(&self) -> PdfResult<Option<String>> {
        Ok(self.status_store.read()?.active_job_id)
    }

    pub fn active_job(&self) -> PdfResult<Option<CompileJob>> {
        let Some(id) = self.active_job_id()? else {
            return Ok(None);
        };
        match self.load_job(&id) {
            Ok(j) => Ok(Some(j)),
            Err(PdfModuleError::FileNotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn start_stage(&self, job_id: &str, stage: CompileStage) -> PdfResult<CompileJob> {
        let mut job = self.load_job(job_id)?;
        let record = job.stage_mut(stage)?;
        record.status = StageStatus::Running;
        record.started_at = Some(Utc::now());
        record.error = None;
        job.pipeline_status = PipelineStatus::Running;
        job.updated_at = Utc::now();
        self.write_job(&job)?;
        self.sync_status_summary(&job, true)?;
        Ok(job)
    }

    pub fn succeed_stage(&self, job_id: &str, stage: CompileStage) -> PdfResult<CompileJob> {
        let mut job = self.load_job(job_id)?;
        let now = Utc::now();
        let record = job.stage_mut(stage)?;
        if let Some(started) = record.started_at {
            record.duration_ms = Some((now - started).num_milliseconds().max(0) as u64);
        }
        record.status = StageStatus::Succeeded;
        record.finished_at = Some(now);
        job.updated_at = now;
        self.write_job(&job)?;
        self.sync_status_summary(&job, true)?;
        Ok(job)
    }

    pub fn fail_stage(
        &self,
        job_id: &str,
        stage: CompileStage,
        error: impl Into<String>,
    ) -> PdfResult<CompileJob> {
        let mut job = self.load_job(job_id)?;
        let now = Utc::now();
        let msg = error.into();
        let record = job.stage_mut(stage)?;
        if let Some(started) = record.started_at {
            record.duration_ms = Some((now - started).num_milliseconds().max(0) as u64);
        }
        record.status = StageStatus::Failed;
        record.finished_at = Some(now);
        record.error = Some(msg.clone());
        job.pipeline_status = PipelineStatus::Failed;
        job.message = Some(msg);
        job.updated_at = now;
        self.write_job(&job)?;
        self.sync_status_summary(&job, false)?;
        self.finish_summary(&job, "error")?;
        Ok(job)
    }

    pub fn set_awaiting_agent(&self, job_id: &str) -> PdfResult<CompileJob> {
        let mut job = self.load_job(job_id)?;
        job.pipeline_status = PipelineStatus::AwaitingAgent;
        job.message =
            Some("Extraction complete. Waiting for Agent to write wiki entries.".to_string());
        job.updated_at = Utc::now();
        self.write_job(&job)?;
        self.sync_status_summary(&job, false)?;
        Ok(job)
    }

    pub fn push_artifact_raw(&self, job_id: &str, path: impl Into<String>) -> PdfResult<()> {
        let mut job = self.load_job(job_id)?;
        job.artifacts.raw_paths.push(path.into());
        job.updated_at = Utc::now();
        self.write_job(&job)
    }

    pub fn push_artifact_prompt(&self, job_id: &str, path: impl Into<String>) -> PdfResult<()> {
        let mut job = self.load_job(job_id)?;
        job.artifacts.prompt_paths.push(path.into());
        job.updated_at = Utc::now();
        self.write_job(&job)
    }

    pub fn record_entry_saved(
        &self,
        job_id: &str,
        entry_path: impl Into<String>,
    ) -> PdfResult<CompileJob> {
        let path = entry_path.into();
        let mut job = self.load_job(job_id)?;
        if !job.saved_entry_paths.contains(&path) {
            job.saved_entry_paths.push(path);
            job.stats.entries_saved = job.saved_entry_paths.len();
        }
        let agent = job.stage_mut(CompileStage::AgentWiki)?;
        if agent.status == StageStatus::Pending {
            agent.status = StageStatus::Running;
            agent.started_at = Some(Utc::now());
        }
        job.updated_at = Utc::now();
        self.write_job(&job)?;
        self.sync_status_summary(&job, true)?;
        Ok(job)
    }

    pub fn complete_job(
        &self,
        job_id: &str,
        pipeline_status: PipelineStatus,
        message: Option<String>,
    ) -> PdfResult<CompileJob> {
        let mut job = self.load_job(job_id)?;
        let outcome = match pipeline_status {
            PipelineStatus::Completed => "success",
            PipelineStatus::Partial => "partial",
            PipelineStatus::Failed => "error",
            PipelineStatus::AwaitingAgent => "awaiting_agent",
            PipelineStatus::Running => "running",
        };
        job.pipeline_status = pipeline_status;
        job.message = message;
        job.updated_at = Utc::now();
        self.write_job(&job)?;
        self.sync_status_summary(&job, false)?;
        if outcome != "running" && outcome != "awaiting_agent" {
            self.finish_summary(&job, outcome)?;
        }
        Ok(job)
    }

    pub fn build_view(&self) -> PdfResult<CompileJobView> {
        let record = self.status_store.read()?;
        let job = match &record.active_job_id {
            Some(id) => self.load_job(id).ok(),
            None => None,
        };
        let running = record.running
            || job.as_ref().is_some_and(|j| {
                matches!(j.pipeline_status, PipelineStatus::Running | PipelineStatus::AwaitingAgent)
            });
        let pipeline_status = job
            .as_ref()
            .map(|j| serde_json::to_value(&j.pipeline_status).ok())
            .flatten()
            .and_then(|v| v.as_str().map(String::from));
        Ok(CompileJobView {
            running,
            pipeline_status,
            active_job_id: record.active_job_id,
            job,
            last_started: record.last_started,
            last_finished: record.last_finished,
            last_duration_ms: record.last_duration_ms,
            last_outcome: record.last_outcome,
            message: record.message,
            history: record.history,
        })
    }

    fn job_path(&self, job_id: &str) -> PathBuf {
        self.jobs_dir.join(format!("{job_id}.json"))
    }

    pub fn write_job(&self, job: &CompileJob) -> PdfResult<()> {
        fs::create_dir_all(&self.jobs_dir).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to create compile_jobs dir: {e}"))
        })?;
        let json = serde_json::to_string_pretty(job).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to serialize compile job: {e}"))
        })?;
        let path = self.job_path(&job.job_id);
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, &json)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to write compile job: {e}")))?;
        fs::rename(&tmp, &path)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to commit compile job: {e}")))?;
        Ok(())
    }

    fn sync_status_summary(&self, job: &CompileJob, running: bool) -> PdfResult<()> {
        let mut record = self.status_store.read().unwrap_or_else(|_| default_record());
        record.running = running;
        record.active_job_id = Some(job.job_id.clone());
        record.pipeline_status = Some(
            serde_json::to_value(&job.pipeline_status)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| "running".to_string()),
        );
        if running && record.last_started.is_none() {
            record.last_started = Some(job.created_at);
        }
        // In-flight jobs must not inherit a prior terminal `last_outcome` (H1).
        if running
            || matches!(
                job.pipeline_status,
                PipelineStatus::Running | PipelineStatus::AwaitingAgent
            )
        {
            record.last_outcome = None;
        }
        record.message = job
            .message
            .clone()
            .unwrap_or_else(|| format!("Compile job {} in progress.", job.job_id));
        self.status_store.write_record(&record)
    }

    fn finish_summary(&self, job: &CompileJob, outcome: &str) -> PdfResult<()> {
        let mut record = self.status_store.read().unwrap_or_else(|_| default_record());
        let finished = Utc::now();
        let started_at = record.last_started.unwrap_or(job.created_at);
        let duration_ms = (finished - started_at).num_milliseconds().max(0) as u64;

        record.running = false;
        record.last_finished = Some(finished);
        record.last_duration_ms = Some(duration_ms);
        record.last_outcome = Some(outcome.to_string());
        record.message = job.message.clone().unwrap_or_else(|| {
            format!(
                "Job {} finished: {} saved, {} blocked.",
                job.job_id, job.stats.entries_saved, job.stats.entries_blocked
            )
        });
        record.active_job_id = None;
        record.pipeline_status = Some(
            serde_json::to_value(&job.pipeline_status)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| outcome.to_string()),
        );

        let history_entry = CompileHistoryEntry {
            started_at,
            finished_at: finished,
            duration_ms,
            outcome: outcome.to_string(),
            entries_compiled: job.stats.entries_saved,
            entries_skipped: 0,
            job_id: Some(job.job_id.clone()),
        };
        record.history.insert(0, history_entry);
        record.history.truncate(MAX_HISTORY);

        self.status_store.write_record(&record)
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
    fn test_job_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let store = CompileJobStore::new(dir.path());

        let job = store.begin_job(CompileTrigger::SinglePdf).unwrap();
        let id = job.job_id.clone();

        store.start_stage(&id, CompileStage::Extract).unwrap();
        store.succeed_stage(&id, CompileStage::Extract).unwrap();
        store.start_stage(&id, CompileStage::PromptGen).unwrap();
        store.succeed_stage(&id, CompileStage::PromptGen).unwrap();
        store.set_awaiting_agent(&id).unwrap();

        let loaded = store.load_job(&id).unwrap();
        assert_eq!(loaded.pipeline_status, PipelineStatus::AwaitingAgent);

        store.record_entry_saved(&id, "wiki/it/foo.md").unwrap();
        store.complete_job(&id, PipelineStatus::Completed, Some("done".to_string())).unwrap();

        let view = store.build_view().unwrap();
        assert!(!view.running);
        assert_eq!(view.last_outcome.as_deref(), Some("success"));
    }

    #[test]
    fn test_awaiting_agent_clears_stale_last_outcome() {
        let dir = tempfile::tempdir().unwrap();
        let store = CompileJobStore::new(dir.path());

        let first = store.begin_job(CompileTrigger::SinglePdf).unwrap();
        store
            .complete_job(&first.job_id, PipelineStatus::Completed, Some("first".into()))
            .unwrap();

        let second = store.begin_job(CompileTrigger::SinglePdf).unwrap();
        store.set_awaiting_agent(&second.job_id).unwrap();

        let view = store.build_view().unwrap();
        assert_eq!(view.pipeline_status.as_deref(), Some("awaiting_agent"));
        assert_eq!(view.last_outcome, None);
    }
}

/// Build unified compile status JSON for MCP/HTTP (includes optional quality snapshot).
#[cfg(feature = "knowledge")]
pub fn build_compile_status_json(knowledge_base: &Path) -> PdfResult<serde_json::Value> {
    use crate::management::quality_snapshot::QualitySnapshotStore;

    let store = CompileJobStore::new(knowledge_base);
    let view = store.build_view()?;
    let mut value = serde_json::to_value(&view)
        .map_err(|e| PdfModuleError::Storage(format!("Failed to serialize compile view: {e}")))?;
    let snapshot = QualitySnapshotStore::new(knowledge_base).read().unwrap_or_default();
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "quality_snapshot".to_string(),
            serde_json::to_value(&snapshot).unwrap_or(serde_json::json!({})),
        );
    }
    Ok(value)
}
