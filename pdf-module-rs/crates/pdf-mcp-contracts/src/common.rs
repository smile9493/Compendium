//! Shared contract types across MCP tools.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Extraction routing metadata returned by extract and probe tools.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ExtractionEnvelope {
    pub backend_id: String,
    pub method: String,
    pub fallback_used: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_vlm: Option<bool>,
}

/// Optional knowledge-base path resolution (`kb_id` preferred over `knowledge_base`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct KbPathInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kb_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knowledge_base: Option<String>,
}

/// Extraction stack section in health reports.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ExtractionHealth {
    pub backends: Vec<String>,
    pub vlm_configured: bool,
    pub default_method: String,
}
