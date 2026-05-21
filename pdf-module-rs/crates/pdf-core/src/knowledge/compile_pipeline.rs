//! End-to-end compile job completion: index rebuild + quality gate.

use std::path::Path;

use tracing::warn;

use crate::error::PdfResult;
use crate::knowledge::confidence_propagation::PropagationPolicy;
use crate::knowledge::engine::{CompileResult, IncrementalResult, KnowledgeEngine};
use crate::knowledge::index::rebuild_all_with_policy;
use crate::knowledge::quality::analyze_wiki;
use crate::management::compile_job::{
    CompileJobStore, CompileStage, CompileTrigger, PipelineStatus,
};
use crate::management::quality_snapshot::refresh_quality_snapshot;

use crate::knowledge::publish_gate::apply_publish_gate;
use crate::wiki::{NervousEvent, NervousEventKind, sync_nervous_system};

/// Run extract + prompt stages for a single PDF; leaves job awaiting Agent.
pub async fn run_single_pdf_extract(
    engine: &KnowledgeEngine,
    store: &CompileJobStore,
    pdf_path: &Path,
    domain: Option<&str>,
) -> PdfResult<(String, CompileResult)> {
    let job = store.begin_job(CompileTrigger::SinglePdf)?;
    let job_id = job.job_id.clone();

    store.start_stage(&job_id, CompileStage::Extract)?;
    let result = engine.compile_to_wiki(pdf_path, domain).await;
    match &result {
        Ok(r) => {
            store.push_artifact_raw(&job_id, r.raw_path.to_string_lossy().to_string())?;
            for e in &r.entries {
                store.push_artifact_prompt(&job_id, e.path.to_string_lossy().to_string())?;
            }
            store.succeed_stage(&job_id, CompileStage::Extract)?;
            store.start_stage(&job_id, CompileStage::PromptGen)?;
            store.succeed_stage(&job_id, CompileStage::PromptGen)?;
            store.set_awaiting_agent(&job_id)?;
        }
        Err(e) => {
            let _ = store.fail_stage(&job_id, CompileStage::Extract, e.to_string());
        }
    }
    result.map(|r| (job_id, r))
}

/// Incremental extract for all changed PDFs in raw/.
pub async fn run_incremental_extract(
    engine: &KnowledgeEngine,
    store: &CompileJobStore,
) -> PdfResult<(String, IncrementalResult)> {
    let job = store.begin_job(CompileTrigger::Incremental)?;
    let job_id = job.job_id.clone();
    let raw_dir = engine.raw_dir();

    store.start_stage(&job_id, CompileStage::Extract)?;
    let result = engine.incremental_compile(&raw_dir).await;
    match &result {
        Ok(r) => {
            for item in &r.results {
                store.push_artifact_raw(&job_id, item.raw_path.to_string_lossy().to_string())?;
                for e in &item.entries {
                    store.push_artifact_prompt(&job_id, e.path.to_string_lossy().to_string())?;
                }
            }
            let mut j = store.load_job(&job_id)?;
            j.stats.entries_expected = r.compiled.max(1);
            store.write_job(&j)?;
            store.succeed_stage(&job_id, CompileStage::Extract)?;
            store.start_stage(&job_id, CompileStage::PromptGen)?;
            store.succeed_stage(&job_id, CompileStage::PromptGen)?;
            store.set_awaiting_agent(&job_id)?;
        }
        Err(e) => {
            let _ = store.fail_stage(&job_id, CompileStage::Extract, e.to_string());
        }
    }
    result.map(|r| (job_id, r))
}

/// Result of finishing a compile job pipeline.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CompleteCompileJobResult {
    pub job_id: String,
    pub pipeline_status: String,
    pub index_rebuild: serde_json::Value,
    pub quality_gate: serde_json::Value,
    pub entries_saved: usize,
    pub entries_blocked: usize,
    pub human_review_summary: String,
}

/// Run index rebuild and quality gate stages, then mark the job complete.
pub fn complete_compile_job(
    knowledge_base: &Path,
    job_id: &str,
    force: bool,
    propagation_policy: Option<PropagationPolicy>,
) -> PdfResult<CompleteCompileJobResult> {
    let propagation_policy = propagation_policy.unwrap_or_default();
    let store = CompileJobStore::new(knowledge_base);
    let job = store.load_job(job_id)?;

    if !force && matches!(job.pipeline_status, PipelineStatus::Completed | PipelineStatus::Failed) {
        return Ok(CompleteCompileJobResult {
            job_id: job_id.to_string(),
            pipeline_status: serde_json::to_value(&job.pipeline_status)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| "completed".to_string()),
            index_rebuild: serde_json::json!({"skipped": true}),
            quality_gate: serde_json::json!({"skipped": true}),
            entries_saved: job.stats.entries_saved,
            entries_blocked: job.stats.entries_blocked,
            human_review_summary: "skipped: job already completed".to_string(),
        });
    }

    store.start_stage(job_id, CompileStage::IndexRebuild)?;
    let index_stats = match rebuild_all_with_policy(knowledge_base, &propagation_policy) {
        Ok((stats, propagation)) => {
            store.succeed_stage(job_id, CompileStage::IndexRebuild)?;
            serde_json::json!({
                "fulltext_entries_indexed": stats.fulltext_entries_indexed,
                "graph_nodes": stats.graph_nodes,
                "graph_edges": stats.graph_edges,
                "vector_entries_indexed": stats.vector_entries_indexed,
                "confidence_propagation": propagation,
            })
        }
        Err(e) => {
            let _ = store.fail_stage(job_id, CompileStage::IndexRebuild, e.to_string());
            return Err(e);
        }
    };

    store.start_stage(job_id, CompileStage::QualityGate)?;
    let wiki_dir = knowledge_base.join("wiki");
    let gate_result = if wiki_dir.exists() {
        match apply_publish_gate(knowledge_base) {
            Ok(g) => {
                let mut job = store.load_job(job_id)?;
                job.stats.entries_blocked = g.blocked_count;
                store.write_job(&job)?;
                let _ = refresh_quality_snapshot(knowledge_base);
                serde_json::to_value(&g).unwrap_or_default()
            }
            Err(e) => {
                warn!(error = %e, "Publish gate failed, falling back to snapshot only");
                let _ = analyze_wiki(&wiki_dir).ok();
                let _ = refresh_quality_snapshot(knowledge_base);
                serde_json::json!({"fallback": true, "error": e.to_string()})
            }
        }
    } else {
        let _ = refresh_quality_snapshot(knowledge_base);
        serde_json::json!({"skipped": true, "reason": "no wiki directory"})
    };
    store.succeed_stage(job_id, CompileStage::QualityGate)?;

    let job = store.load_job(job_id)?;
    let blocked = job.stats.entries_blocked;
    let (status, outcome) = if blocked > 0 {
        (PipelineStatus::Partial, "partial")
    } else {
        (PipelineStatus::Completed, "success")
    };

    if job.stage_by_kind(CompileStage::AgentWiki).is_some_and(|s| {
        matches!(
            s.status,
            crate::management::compile_job::StageStatus::Running
                | crate::management::compile_job::StageStatus::Pending
        )
    }) {
        let _ = store.succeed_stage(job_id, CompileStage::AgentWiki);
    }

    store.complete_job(
        job_id,
        status,
        Some(format!("Pipeline complete ({outcome}): index rebuilt, quality gate applied.")),
    )?;

    let final_job = store.load_job(job_id)?;
    let human_review_summary = format!(
        "job={job_id} saved={} blocked={} outcome={outcome}",
        final_job.stats.entries_saved, final_job.stats.entries_blocked
    );
    let _ = sync_nervous_system(
        knowledge_base,
        NervousEvent::new(NervousEventKind::CompileComplete, human_review_summary.clone()),
    );
    Ok(CompleteCompileJobResult {
        job_id: job_id.to_string(),
        pipeline_status: outcome.to_string(),
        index_rebuild: index_stats,
        quality_gate: gate_result,
        entries_saved: final_job.stats.entries_saved,
        entries_blocked: final_job.stats.entries_blocked,
        human_review_summary,
    })
}
