//! Extract tool contracts (6 tools).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::common::ExtractionEnvelope;
use crate::registry::McpToolSpec;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractTextInput {
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractTextOutput {
    pub text: String,
    pub extraction: ExtractionEnvelope,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractStructuredInput {
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractStructuredOutput {
    #[schemars(with = "serde_json::Value")]
    pub structured: serde_json::Value,
    pub extraction: ExtractionEnvelope,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetPageCountInput {
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetPageCountOutput {
    pub page_count: u32,
    pub extraction: ExtractionEnvelope,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchKeywordsInput {
    pub file_path: String,
    pub keywords: Vec<String>,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default = "default_context_length")]
    pub context_length: u64,
}

fn default_context_length() -> u64 {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchKeywordsOutput {
    pub total_matches: usize,
    pub pages_with_matches: usize,
    pub matches: Vec<KeywordMatch>,
    pub extraction: ExtractionEnvelope,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct KeywordMatch {
    pub keyword: String,
    pub page: u32,
    pub position: usize,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtrudeToServerWikiInput {
    pub file_path: String,
    #[serde(default)]
    pub wiki_base_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtrudeToServerWikiOutput {
    pub status: String,
    pub raw_path: String,
    pub index_path: String,
    pub log_path: String,
    pub page_count: u32,
    pub message: String,
    pub next_step: String,
    pub extraction: ExtractionEnvelope,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtrudeToAgentPayloadInput {
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtrudeToAgentPayloadOutput {
    pub payload: String,
    pub extraction: ExtractionEnvelope,
}

pub fn tool_specs() -> Vec<McpToolSpec> {
    vec![
        McpToolSpec::new::<ExtractTextInput, ExtractTextOutput>(
            "extract_text",
            "Extract plain text from a PDF file using pdfium engine",
        ),
        McpToolSpec::new::<ExtractStructuredInput, ExtractStructuredOutput>(
            "extract_structured",
            "Extract structured data (per-page text + bbox) from PDF",
        ),
        McpToolSpec::new::<GetPageCountInput, GetPageCountOutput>(
            "get_page_count",
            "Get the number of pages in a PDF file",
        ),
        McpToolSpec::new::<SearchKeywordsInput, SearchKeywordsOutput>(
            "search_keywords",
            "Search for keywords in a PDF file and return matches with page numbers and context",
        ),
        McpToolSpec::new::<ExtrudeToServerWikiInput, ExtrudeToServerWikiOutput>(
            "extrude_to_server_wiki",
            "Extract PDF to server-side wiki raw/ (Karpathy paradigm)",
        ),
        McpToolSpec::new::<ExtrudeToAgentPayloadInput, ExtrudeToAgentPayloadOutput>(
            "extrude_to_agent_payload",
            "Extract PDF and return markdown payload with compilation instructions for AI Agent",
        ),
    ]
}
