//! Platform tool contracts (10 tools).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::common::{ExtractionEnvelope, KbPathInput};
use crate::registry::McpToolSpec;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListWorkspacesInput {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListWorkspacesOutput {
    pub workspaces: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SetActiveWorkspaceInput {
    pub kb_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SetActiveWorkspaceOutput {
    pub active_kb_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RegisterWorkspaceInput {
    pub kb_id: String,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RegisterWorkspaceOutput {
    pub kb_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListExtractionPluginsInput {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListExtractionPluginsOutput {
    pub backends: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProbeExtractionInput {
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProbeExtractionOutput {
    pub backend_id: String,
    pub extraction_method: String,
    pub quality_score: Option<f64>,
    pub needs_vlm: bool,
    pub extraction: ExtractionEnvelope,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SyncStatusInput {
    pub remote_url: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SyncStatusOutput {
    #[schemars(with = "serde_json::Value")]
    pub status: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SyncPushInput {
    pub remote_url: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SyncPushOutput {
    #[schemars(with = "serde_json::Value")]
    pub report: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SyncPullInput {
    pub remote_url: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
    #[serde(default)]
    pub rebuild_index: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SyncPullOutput {
    #[schemars(with = "serde_json::Value")]
    pub report: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SubmitPatchProposalInput {
    pub entry_path: String,
    pub operations: serde_json::Value,
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SubmitPatchProposalOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApplyPatchProposalInput {
    pub proposal_id: String,
    #[serde(flatten)]
    pub kb: KbPathInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApplyPatchProposalOutput {
    #[schemars(with = "serde_json::Value")]
    pub result: serde_json::Value,
}

pub fn tool_specs() -> Vec<McpToolSpec> {
    vec![
        McpToolSpec::new::<ListWorkspacesInput, ListWorkspacesOutput>(
            "list_workspaces",
            "List registered knowledge base workspaces",
        ),
        McpToolSpec::new::<SetActiveWorkspaceInput, SetActiveWorkspaceOutput>(
            "set_active_workspace",
            "Set the active workspace by kb_id",
        ),
        McpToolSpec::new::<RegisterWorkspaceInput, RegisterWorkspaceOutput>(
            "register_workspace",
            "Register or update a knowledge base workspace",
        ),
        McpToolSpec::new::<ListExtractionPluginsInput, ListExtractionPluginsOutput>(
            "list_extraction_plugins",
            "List extraction backends in the current router",
        ),
        McpToolSpec::new::<ProbeExtractionInput, ProbeExtractionOutput>(
            "probe_extraction",
            "Probe which extraction backend would be selected for a PDF",
        ),
        McpToolSpec::new::<SyncStatusInput, SyncStatusOutput>(
            "sync_status",
            "Compare local KB manifest with remote sync store",
        ),
        McpToolSpec::new::<SyncPushInput, SyncPushOutput>(
            "sync_push",
            "Push local KB objects to remote sync store",
        ),
        McpToolSpec::new::<SyncPullInput, SyncPullOutput>(
            "sync_pull",
            "Pull KB objects from remote sync store",
        ),
        McpToolSpec::new::<SubmitPatchProposalInput, SubmitPatchProposalOutput>(
            "submit_patch_proposal",
            "Submit wiki patch proposal without applying",
        ),
        McpToolSpec::new::<ApplyPatchProposalInput, ApplyPatchProposalOutput>(
            "apply_patch_proposal",
            "Apply a pending patch proposal by id",
        ),
    ]
}
