//! Common Data Transfer Objects
//!
//! Unified DTOs consolidating definitions from pdf-core and pdf-etl.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool execution context (unified version).
///
/// Combines the definitions from `pdf-core::dto::ToolContext` and
/// `pdf-core::plugin::tool_handler::ToolContext`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContext {
    /// Execution ID
    pub execution_id: String,
    /// Organization ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    /// Workflow ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    /// User ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl ToolContext {
    /// Create a new tool context
    pub fn new(execution_id: impl Into<String>) -> Self {
        Self {
            execution_id: execution_id.into(),
            org_id: None,
            workflow_id: None,
            user_id: None,
            request_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Builder: set organization ID
    pub fn with_org_id(mut self, org_id: impl Into<String>) -> Self {
        self.org_id = Some(org_id.into());
        self
    }

    /// Builder: set workflow ID
    pub fn with_workflow_id(mut self, workflow_id: impl Into<String>) -> Self {
        self.workflow_id = Some(workflow_id.into());
        self
    }

    /// Builder: set user ID
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Builder: set request ID
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Builder: add metadata entry
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Tool execution options (unified version).
///
/// Combines definitions from `pdf-core::dto::ToolExecutionOptions` and
/// `pdf-core::plugin::tool_handler::ToolExecutionOptions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionOptions {
    /// Whether to enable streaming output
    #[serde(default)]
    pub enable_streaming: bool,
    /// Timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    /// Whether to enable caching
    #[serde(default = "default_true")]
    pub enable_cache: bool,
    /// Whether to enable metrics
    #[serde(default = "default_true")]
    pub enable_metrics: bool,
    /// Additional options
    #[serde(default)]
    pub additional: HashMap<String, serde_json::Value>,
}

fn default_true() -> bool {
    true
}

impl Default for ToolExecutionOptions {
    fn default() -> Self {
        Self {
            enable_streaming: false,
            timeout: None,
            enable_cache: true,
            enable_metrics: true,
            additional: HashMap::new(),
        }
    }
}

impl ToolExecutionOptions {
    /// Create new default options
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder: enable streaming
    pub fn with_streaming(mut self) -> Self {
        self.enable_streaming = true;
        self
    }

    /// Builder: set timeout in milliseconds
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout = Some(timeout_ms);
        self
    }

    /// Builder: disable caching
    pub fn without_cache(mut self) -> Self {
        self.enable_cache = false;
        self
    }

    /// Builder: disable metrics
    pub fn without_metrics(mut self) -> Self {
        self.enable_metrics = false;
        self
    }

    /// Builder: add additional option
    pub fn with_option(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.additional.insert(key.into(), value);
        self
    }
}

/// Text extraction metadata
#[derive(Debug, Clone, Serialize, Deserialize, pdf_macros::Builder)]
pub struct TextExtractionMetadata {
    /// Content hash
    pub whisper_hash: String,
    /// Line-level metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_metadata: Option<serde_json::Value>,
}

/// Line-level information with bounding box
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineInfo {
    /// Bounding box coordinates
    pub bbox: Vec<f64>,
    /// Text content
    pub text: String,
}

/// Page metadata with text and layout information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageMetadata {
    /// Page number (1-indexed)
    pub page_number: u32,
    /// Text content
    pub text: String,
    /// Bounding box (x0, y0, x1, y1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<(f64, f64, f64, f64)>,
    /// Line-level information
    pub lines: Vec<LineInfo>,
}

/// Text extraction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextExtractionResult {
    /// Extracted text content
    pub extracted_text: String,
    /// Extraction metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_metadata: Option<TextExtractionMetadata>,
}

/// File information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// File path
    pub file_path: String,
    /// File size in bytes
    pub file_size: u64,
    /// File size in megabytes
    pub file_size_mb: f64,
}

impl FileInfo {
    /// Create FileInfo from a file path by reading filesystem metadata
    pub fn from_path(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len();
        let file_size_mb = (file_size as f64 / 1024.0 / 1024.0 * 100.0).round() / 100.0;
        Ok(Self {
            file_path: path.to_string_lossy().to_string(),
            file_size,
            file_size_mb,
        })
    }
}

/// Structured extraction result with page-level information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredExtractionResult {
    /// Full extracted text
    pub extracted_text: String,
    /// Total page count
    pub page_count: u32,
    /// Per-page metadata
    pub pages: Vec<PageMetadata>,
    /// Extraction metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_metadata: Option<TextExtractionMetadata>,
    /// Source file information
    pub file_info: FileInfo,
}

/// Keyword match information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordMatch {
    /// Matched keyword
    pub keyword: String,
    /// Page number where match was found
    pub page_number: u32,
    /// Surrounding text context
    pub text: String,
    /// Bounding box of match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<(f64, f64, f64, f64)>,
    /// Start index in page text
    pub start_index: usize,
    /// End index in page text
    pub end_index: usize,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
}

/// Keyword search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordSearchResult {
    /// Searched keywords
    pub keywords: Vec<String>,
    /// All matches found
    pub matches: Vec<KeywordMatch>,
    /// Total match count
    pub total_matches: usize,
    /// Pages containing at least one match
    pub pages_with_matches: Vec<u32>,
}

/// Extraction options
#[derive(Debug, Clone, Default, Serialize, Deserialize, pdf_macros::Builder)]
pub struct ExtractOptions {
    /// Whether to enable highlight information
    #[serde(default)]
    pub enable_highlight: bool,
    /// Specific adapter/engine to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
}

/// Adapter/engine information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterInfo {
    /// Engine identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Engine description
    pub description: String,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Current cache size
    pub size: u64,
    /// Maximum cache size
    pub max_size: u64,
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Hit rate (0.0 - 1.0)
    pub hit_rate: f64,
}

/// Execution status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Success,
    Failed,
    Timeout,
    Cancelled,
}

/// Execution metric
#[derive(Debug, Clone, Serialize, Deserialize, pdf_macros::Builder)]
pub struct ExecutionMetric {
    pub tool_name: String,
    pub execution_id: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub status: ExecutionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_context_builder() {
        let ctx = ToolContext::new("exec-123")
            .with_org_id("org-456")
            .with_user_id("user-789")
            .with_metadata("key", "value");

        assert_eq!(ctx.execution_id, "exec-123");
        assert_eq!(ctx.org_id, Some("org-456".to_string()));
        assert_eq!(ctx.user_id, Some("user-789".to_string()));
        assert_eq!(ctx.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_tool_context_serialization() {
        let ctx = ToolContext::new("exec-123").with_org_id("org-456");
        let json = serde_json::to_string(&ctx).unwrap();
        let parsed: ToolContext = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.execution_id, "exec-123");
        assert_eq!(parsed.org_id, Some("org-456".to_string()));
    }

    #[test]
    fn test_tool_execution_options_default() {
        let opts = ToolExecutionOptions::default();
        assert!(!opts.enable_streaming);
        assert!(opts.timeout.is_none());
        assert!(opts.enable_cache);
        assert!(opts.enable_metrics);
        assert!(opts.additional.is_empty());
    }

    #[test]
    fn test_tool_execution_options_builder() {
        let opts = ToolExecutionOptions::new()
            .with_streaming()
            .with_timeout(60000)
            .without_cache()
            .without_metrics()
            .with_option("custom_key", serde_json::json!("custom_value"));

        assert!(opts.enable_streaming);
        assert_eq!(opts.timeout, Some(60000));
        assert!(!opts.enable_cache);
        assert!(!opts.enable_metrics);
        assert_eq!(
            opts.additional.get("custom_key"),
            Some(&serde_json::json!("custom_value"))
        );
    }

    #[test]
    fn test_text_extraction_result_serialization() {
        let result = TextExtractionResult {
            extracted_text: "Hello World".to_string(),
            extraction_metadata: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Hello World"));
        assert!(!json.contains("extraction_metadata"));
    }

    #[test]
    fn test_structured_extraction_result_roundtrip() {
        let result = StructuredExtractionResult {
            extracted_text: "Test".to_string(),
            page_count: 1,
            pages: vec![PageMetadata {
                page_number: 1,
                text: "Test".to_string(),
                bbox: Some((0.0, 0.0, 100.0, 100.0)),
                lines: vec![],
            }],
            extraction_metadata: None,
            file_info: FileInfo {
                file_path: "/test.pdf".to_string(),
                file_size: 1024,
                file_size_mb: 0.0,
            },
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: StructuredExtractionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.page_count, 1);
    }
}
