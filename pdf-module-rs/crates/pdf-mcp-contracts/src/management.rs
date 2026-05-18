//! Management tool contracts (9 tools).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::common::{ExtractionHealth, KbPathInput};
use crate::registry::McpToolSpec;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetConfigInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetConfigOutput {
    #[schemars(with = "serde_json::Value")]
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SetConfigInput {
    pub key: String,
    pub value: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SetConfigOutput {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetHealthReportInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetHealthReportOutput {
    pub total_entries: usize,
    pub orphan_count: usize,
    pub contradiction_count: usize,
    pub broken_link_count: usize,
    pub index_size_mb: u64,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub avg_quality_score: String,
    pub domains: Vec<String>,
    pub last_compile: Option<String>,
    pub generated_at: String,
    pub report_text: String,
    pub quality_snapshot: serde_json::Value,
    pub extraction: ExtractionHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TriggerIncrementalCompileInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TriggerIncrementalCompileOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetCompileStatusInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetCompileStatusOutput {
    #[schemars(with = "serde_json::Value")]
    pub status: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListQualityIssuesInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListQualityIssuesOutput {
    pub issues: serde_json::Value,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FixSuggestInput {
    pub issue_id: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FixSuggestOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApplyQualityGateInput {
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub job_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApplyQualityGateOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ShowWikiBrowserInput {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ShowWikiBrowserOutput {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub uri: String,
}

pub fn tool_specs() -> Vec<McpToolSpec> {
    vec![
        McpToolSpec::new::<GetConfigInput, GetConfigOutput>(
            "get_config",
            "Get runtime configuration for a knowledge base",
        ),
        McpToolSpec::new::<SetConfigInput, SetConfigOutput>(
            "set_config",
            "Set a runtime configuration value (atomic write)",
        ),
        McpToolSpec::new::<GetHealthReportInput, GetHealthReportOutput>(
            "get_health_report",
            "Comprehensive KB health: entries, graph, index, quality, extraction stack",
        ),
        McpToolSpec::new::<TriggerIncrementalCompileInput, TriggerIncrementalCompileOutput>(
            "trigger_incremental_compile",
            "Manually trigger incremental compilation",
        ),
        McpToolSpec::new::<GetCompileStatusInput, GetCompileStatusOutput>(
            "get_compile_status",
            "Compile status with stages and quality snapshot",
        ),
        McpToolSpec::new::<ListQualityIssuesInput, ListQualityIssuesOutput>(
            "list_quality_issues",
            "List quality issues with stable issue_id",
        ),
        McpToolSpec::new::<FixSuggestInput, FixSuggestOutput>(
            "fix_suggest",
            "Suggest MCP actions to fix a quality issue",
        ),
        McpToolSpec::new::<ApplyQualityGateInput, ApplyQualityGateOutput>(
            "apply_quality_gate",
            "Run publish quality gate on all wiki entries",
        ),
        McpToolSpec::new::<ShowWikiBrowserInput, ShowWikiBrowserOutput>(
            "show_wiki_browser",
            "Open interactive wiki browser MCP App resource",
        ),
    ]
}
