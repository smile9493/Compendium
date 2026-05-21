//! Index / wiki tool contracts (11 tools including get_compilation_context).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::common::KbPathInput;
use crate::registry::McpToolSpec;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchKnowledgeInput {
    pub query: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub limit: Option<u64>,
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchKnowledgeOutput {
    pub hits: Vec<SearchHitOut>,
    pub meta: SearchMetaOut,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchHitOut {
    pub path: String,
    pub title: String,
    pub score: f32,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchMetaOut {
    pub index_empty: bool,
    pub used_fallback: bool,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RebuildIndexInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default = "default_auto_write")]
    pub auto_write: bool,
    #[serde(default)]
    pub propagation_depth: Option<u8>,
}

fn default_auto_write() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RebuildIndexOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetEntryContextInput {
    pub entry_path: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub hops: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetEntryContextOutput {
    #[schemars(with = "serde_json::Value")]
    pub neighbors: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetAgentContextInput {
    pub entry_path: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub hops: Option<u64>,
    #[serde(default)]
    pub max_body_chars: Option<u64>,
    #[serde(default)]
    pub related_limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetAgentContextOutput {
    pub entry_path: String,
    pub center: AgentCenterOut,
    pub neighbors: serde_json::Value,
    pub related_snippets: Vec<RelatedSnippetOut>,
    pub token_estimate: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentCenterOut {
    pub title: String,
    pub domain: String,
    pub tags: Vec<String>,
    pub front_matter: serde_json::Value,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RelatedSnippetOut {
    pub path: String,
    pub title: String,
    pub score: f32,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PreviewWikiPatchInput {
    pub entry_path: String,
    pub operations: serde_json::Value,
    #[serde(flatten)]
    pub kb: KbPathInput,
}

pub type ApplyWikiPatchInput = PatchWikiEntryInput;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PatchWikiEntryInput {
    pub entry_path: String,
    pub operations: serde_json::Value,
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PatchWikiEntryOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

pub type ApplyWikiPatchOutput = PatchWikiEntryOutput;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FindOrphansInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FindOrphansOutput {
    pub orphans: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SuggestLinksInput {
    pub entry_path: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub top_k: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SuggestLinksOutput {
    #[schemars(with = "serde_json::Value")]
    pub suggestions: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExportConceptMapInput {
    pub entry_path: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub depth: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExportConceptMapOutput {
    pub mermaid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CheckQualityInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CheckQualityOutput {
    #[schemars(with = "serde_json::Value")]
    pub report: serde_json::Value,
    pub report_markdown: String,
    pub has_errors: bool,
    pub has_warnings: bool,
    pub next_actions: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetCompilationContextInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub include_prompt_excerpts: bool,
    #[serde(default = "default_max_chars")]
    pub max_chars: u64,
}

fn default_max_chars() -> u64 {
    8000
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetCompilationContextOutput {
    pub active_job_id: Option<String>,
    pub pipeline_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job: Option<serde_json::Value>,
    pub stages: serde_json::Value,
    pub artifacts: serde_json::Value,
    pub stats: serde_json::Value,
    pub quality_snapshot: serde_json::Value,
    pub suggested_next_tools: Vec<String>,
    #[serde(default)]
    pub prompt_excerpts: Vec<PromptExcerptOut>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromptExcerptOut {
    pub path: String,
    pub excerpt: String,
}

pub fn tool_specs() -> Vec<McpToolSpec> {
    vec![
        McpToolSpec::new::<SearchKnowledgeInput, SearchKnowledgeOutput>(
            "search_knowledge",
            "Search wiki entries (hybrid Tantivy + TF-IDF RRF)",
        ),
        McpToolSpec::new::<RebuildIndexInput, RebuildIndexOutput>(
            "rebuild_index",
            "Rebuild Tantivy, petgraph, and TF-IDF indexes from wiki Markdown",
        ),
        McpToolSpec::new::<GetEntryContextInput, GetEntryContextOutput>(
            "get_entry_context",
            "Get N-hop neighbors of a knowledge entry",
        ),
        McpToolSpec::new::<GetAgentContextInput, GetAgentContextOutput>(
            "get_agent_context",
            "Token-efficient context bundle: center body, neighbors, related snippets",
        ),
        McpToolSpec::new::<PreviewWikiPatchInput, PatchWikiEntryOutput>(
            "preview_wiki_patch",
            "Preview a structured patch (unified diff, no write)",
        ),
        McpToolSpec::new::<PatchWikiEntryInput, PatchWikiEntryOutput>(
            "patch_wiki_entry",
            "Apply structured patch and reindex entry",
        ),
        McpToolSpec::new::<ApplyWikiPatchInput, ApplyWikiPatchOutput>(
            "apply_wiki_patch",
            "Alias for patch_wiki_entry — apply structured patch and reindex",
        ),
        McpToolSpec::new::<FindOrphansInput, FindOrphansOutput>(
            "find_orphans",
            "Find entries with no related/contradiction links",
        ),
        McpToolSpec::new::<SuggestLinksInput, SuggestLinksOutput>(
            "suggest_links",
            "Suggest links based on tag similarity (Jaccard)",
        ),
        McpToolSpec::new::<ExportConceptMapInput, ExportConceptMapOutput>(
            "export_concept_map",
            "Export local concept map as Mermaid.js text",
        ),
        McpToolSpec::new::<CheckQualityInput, CheckQualityOutput>(
            "check_quality",
            "Analyze wiki quality and return report with next actions",
        ),
        McpToolSpec::new::<GetCompilationContextInput, GetCompilationContextOutput>(
            "get_compilation_context",
            "Compile-job context for awaiting_agent: stages, artifacts, prompts",
        ),
    ]
}
