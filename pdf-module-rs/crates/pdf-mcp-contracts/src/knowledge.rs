//! Knowledge / compile tool contracts.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::common::KbPathInput;
use crate::registry::McpToolSpec;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompileToWikiInput {
    pub pdf_path: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompileToWikiOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IncrementalCompileInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IncrementalCompileOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MicroCompileInput {
    pub pdf_path: String,
    #[serde(default)]
    pub page_range: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MicroCompileOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AggregateEntriesInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AggregateEntriesOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HypothesisTestInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HypothesisTestOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecompileEntryInput {
    pub entry_path: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecompileEntryOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SaveWikiEntryInput {
    pub entry_path: String,
    pub content: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub plan_task_id: Option<String>,
    #[serde(default = "default_true")]
    pub mark_compiled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SaveWikiEntryOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompleteCompileJobInput {
    pub job_id: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub force: bool,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default = "default_true")]
    pub auto_write: bool,
    #[serde(default)]
    pub propagation_depth: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompileImageInput {
    pub image_path: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompileImageOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompleteCompileJobOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerateCompilePlanInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerateCompilePlanOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetCompilePlanInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetCompilePlanOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MarkPlanTaskDoneInput {
    pub task_id: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MarkPlanTaskDoneOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompileUploadedPdfInput {
    pub file_id: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompileUploadedPdfOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InitKnowledgeBaseInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InitKnowledgeBaseOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LintWikiInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LintWikiOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArchiveAnswerInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
    pub title: String,
    pub body_markdown: String,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub entry_path: Option<String>,
    #[serde(default)]
    pub related: Option<Vec<String>>,
    #[serde(default)]
    pub entry_type: Option<String>,
    #[serde(default)]
    pub confidence: Option<String>,
    #[serde(default)]
    pub sources: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArchiveAnswerOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DetectStaleEntriesInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub max_age_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DetectStaleEntriesOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MetaIngestInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub pdf_path: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MetaIngestOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MetaQueryInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub entry_path: Option<String>,
    #[serde(default)]
    pub limit: Option<u64>,
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MetaQueryOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MetaLintInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub max_age_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MetaLintOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoadToolsInput {
    /// `core` | `deferred` | `code_only` | `all` (index only)
    #[serde(default = "default_load_tier")]
    pub tier: String,
    #[serde(default = "default_load_limit")]
    pub limit: u32,
}

fn default_load_tier() -> String {
    "deferred".to_string()
}

fn default_load_limit() -> u32 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoadToolsOutput {
    pub tier: String,
    pub tools: Vec<serde_json::Value>,
    pub progressive_index: serde_json::Value,
}

pub fn tool_specs() -> Vec<McpToolSpec> {
    vec![
        McpToolSpec::new::<InitKnowledgeBaseInput, InitKnowledgeBaseOutput>(
            "init_knowledge_base",
            "Initialize a new knowledge base from Karpathy-style templates (schema, index, log)",
        ),
        McpToolSpec::new::<LintWikiInput, LintWikiOutput>(
            "lint_wiki",
            "Karpathy lint: orphans, broken links, contradictions, drift, missing concepts, stale entries",
        ),
        McpToolSpec::new::<DetectStaleEntriesInput, DetectStaleEntriesOutput>(
            "detect_stale_entries",
            "List wiki entries not updated or validated within max_age_days",
        ),
        McpToolSpec::new::<MetaIngestInput, MetaIngestOutput>(
            "ingest",
            "Karpathy ingest: compile_to_wiki or incremental_compile plus index context",
        ),
        McpToolSpec::new::<MetaQueryInput, MetaQueryOutput>(
            "query",
            "Karpathy query: search_knowledge plus optional entry context and index excerpt",
        ),
        McpToolSpec::new::<MetaLintInput, MetaLintOutput>(
            "lint",
            "Karpathy lint: lint_wiki, check_quality, find_orphans, detect_stale_entries, load-bearing",
        ),
        McpToolSpec::new::<LoadToolsInput, LoadToolsOutput>(
            "load_tools",
            "Progressive tool discovery: list deferred/code_only tools by tier (hybrid manifest)",
        ),
        McpToolSpec::new::<ArchiveAnswerInput, ArchiveAnswerOutput>(
            "archive_answer",
            "Write a query answer back to the wiki as an overview page",
        ),
        McpToolSpec::new::<CompileToWikiInput, CompileToWikiOutput>(
            "compile_to_wiki",
            "Compile a PDF into the knowledge base (Karpathy compiler pattern)",
        ),
        McpToolSpec::new::<IncrementalCompileInput, IncrementalCompileOutput>(
            "incremental_compile",
            "Scan raw/ for changed PDFs and compile only those that need it",
        ),
        McpToolSpec::new::<MicroCompileInput, MicroCompileOutput>(
            "micro_compile",
            "On-demand PDF extraction for conversation context (not saved to wiki)",
        ),
        McpToolSpec::new::<AggregateEntriesInput, AggregateEntriesOutput>(
            "aggregate_entries",
            "Identify clusters of related L1 entries for L2 synthesis",
        ),
        McpToolSpec::new::<HypothesisTestInput, HypothesisTestOutput>(
            "hypothesis_test",
            "Find contradicting entry pairs and generate debate framework",
        ),
        McpToolSpec::new::<RecompileEntryInput, RecompileEntryOutput>(
            "recompile_entry",
            "Recompile a single wiki entry with version bump",
        ),
        McpToolSpec::new::<SaveWikiEntryInput, SaveWikiEntryOutput>(
            "save_wiki_entry",
            "Create or update a wiki entry (YAML front matter required)",
        ),
        McpToolSpec::new::<CompleteCompileJobInput, CompleteCompileJobOutput>(
            "complete_compile_job",
            "Finish a compile job: rebuild indexes and run quality gate",
        ),
        McpToolSpec::new::<GenerateCompilePlanInput, GenerateCompilePlanOutput>(
            "generate_compile_plan",
            "Generate compile_plan.json with L1/L2/L3 tasks",
        ),
        McpToolSpec::new::<GetCompilePlanInput, GetCompilePlanOutput>(
            "get_compile_plan",
            "Read the current compile plan and task statuses",
        ),
        McpToolSpec::new::<MarkPlanTaskDoneInput, MarkPlanTaskDoneOutput>(
            "mark_plan_task_done",
            "Mark a compile plan task as done",
        ),
        McpToolSpec::new::<CompileUploadedPdfInput, CompileUploadedPdfOutput>(
            "compile_uploaded_pdf",
            "Compile an uploaded PDF by file_id from POST /api/upload",
        ),
        McpToolSpec::new::<CompileImageInput, CompileImageOutput>(
            "compile_image",
            "Compile a standalone image via built-in VLM (PNG/JPEG/WebP) into raw/ compile prompts",
        ),
    ]
}
