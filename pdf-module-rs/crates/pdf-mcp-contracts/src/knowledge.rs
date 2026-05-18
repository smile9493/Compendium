//! Knowledge / compile tool contracts (12 tools).

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

pub fn tool_specs() -> Vec<McpToolSpec> {
    vec![
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
    ]
}
