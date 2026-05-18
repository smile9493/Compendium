//! HTTP API response schemas for OpenAPI (utoipa).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ExtractionHealthHttp {
    pub backends: Vec<String>,
    pub vlm_configured: bool,
    pub default_method: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct HealthReportHttp {
    pub total_entries: usize,
    pub orphan_count: usize,
    pub contradiction_count: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub avg_quality_score: String,
    pub extraction: Option<ExtractionHealthHttp>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct IndexRebuildHttp {
    pub status: String,
    pub fulltext_entries_indexed: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub vector_entries_indexed: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorBody {
    pub error: String,
}
