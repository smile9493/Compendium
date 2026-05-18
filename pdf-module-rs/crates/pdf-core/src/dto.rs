//! Data Transfer Objects for PDF module
//! Corresponds to Python: dto.py

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Text extraction metadata
/// Corresponds to Python: TextExtractionMetadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextExtractionMetadata {
    pub whisper_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_metadata: Option<serde_json::Value>,
}

/// Line-level information with bounding box
/// Corresponds to Python: LineInfo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineInfo {
    pub bbox: Vec<f64>,
    pub text: String,
}

/// Bounding box for text region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Text region with type and bounding box
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRegion {
    pub region_type: String,
    pub bbox: BoundingBox,
    pub text: String,
}

/// Page data with text and optional regions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageData {
    pub page_number: u32,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regions: Option<Vec<TextRegion>>,
}

/// Page metadata with text and layout information
/// Corresponds to Python: PageMetadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageMetadata {
    pub page_number: u32,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<(f64, f64, f64, f64)>,
    pub lines: Vec<LineInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regions: Option<Vec<TextRegion>>,
}

/// Text extraction result
/// Corresponds to Python: TextExtractionResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextExtractionResult {
    pub extracted_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_metadata: Option<TextExtractionMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// File information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub file_path: String,
    pub file_size: u64,
    pub file_size_mb: f64,
}

impl FileInfo {
    /// Create FileInfo from a file path
    pub fn from_path(path: &Path) -> Result<Self, std::io::Error> {
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len();
        let file_size_mb = (file_size as f64 / 1024.0 / 1024.0 * 100.0).round() / 100.0;

        Ok(Self { file_path: path.to_string_lossy().to_string(), file_size, file_size_mb })
    }
}

/// Structured extraction result with page-level information
/// Corresponds to Python: StructuredExtractionResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredExtractionResult {
    pub extracted_text: String,
    pub page_count: u32,
    pub pages: Vec<PageMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_metadata: Option<TextExtractionMetadata>,
    pub file_info: FileInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Keyword match information
/// Corresponds to Python: KeywordMatch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordMatch {
    pub keyword: String,
    pub page_number: u32,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<(f64, f64, f64, f64)>,
    pub start_index: usize,
    pub end_index: usize,
    pub confidence: f64,
}

/// Keyword search result
/// Corresponds to Python: KeywordSearchResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordSearchResult {
    pub keywords: Vec<String>,
    pub matches: Vec<KeywordMatch>,
    pub total_matches: usize,
    pub pages_with_matches: Vec<u32>,
}

/// Extraction options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractOptions {
    #[serde(default)]
    pub enable_highlight: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
}

/// Adapter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub size: u64,
    pub max_size: u64,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

// === MCP Optimization DTOs ===

/// Parameter type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

/// Parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: ParameterType,
    pub description: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

/// Input type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputType {
    File,
    Database,
    Index,
    Text,
}

/// Output type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputType {
    File,
    Database,
    Index,
    Text,
    Json,
}

/// Resource requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirement {
    pub input: bool,
    pub output: bool,
}

/// Tool requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequirements {
    pub files: ResourceRequirement,
    pub databases: ResourceRequirement,
}

/// Extraction status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtractionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

/// Log level
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

/// Storage type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    Local,
    S3,
    Gcs,
    AzureBlob,
    MinIO,
    Http,
}

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub size: u64,
    pub modified: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

/// Environment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Development,
    Staging,
    Production,
}

/// Local storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStorageConfig {
    pub base_dir: String,
}

/// S3 storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3StorageConfig {
    pub bucket: String,
    pub region: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}

/// GCS storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GCSStorageConfig {
    pub bucket: String,
    pub credentials_path: String,
}

/// Azure storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureStorageConfig {
    pub account: String,
    pub key: String,
    pub container: String,
}

/// Property type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PropertyType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

/// Property definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    #[serde(rename = "type")]
    pub property_type: PropertyType,
    pub title: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

/// Variable property
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableProperty {
    #[serde(rename = "type")]
    pub property_type: PropertyType,
    pub title: String,
    pub description: String,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub workflow_id: String,
    pub elapsed_time: u64,
    pub output: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ExecutionMetadata>,
}

/// Execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    pub file_name: String,
    pub file_size: u64,
    pub processing_time: u64,
    pub cache_hit: bool,
    pub adapter_used: String,
}

/// Simple file info for storage operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageFileInfo {
    pub path: String,
    pub size: u64,
}

/// Log format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Json,
    Text,
}

/// Log output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LogOutput {
    Stdout,
    File { path: String },
    Syslog,
}

// === Plugin Architecture DTOs ===

/// Plugin type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    Local,
    Remote,
    Wasm,
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub multiplier: f64,
}

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_second: u32,
    pub burst_size: u32,
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub plugin_id: String,
    pub plugin_type: PluginType,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_policy: Option<RetryPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitConfig>,
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    30000
}

/// Execution status enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Success,
    Failed,
    Timeout,
    Cancelled,
}

/// Execution metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetric {
    pub tool_name: String,
    pub execution_id: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub status: ExecutionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

// Re-export unified ToolContext and ToolExecutionOptions from pdf-common (single source of truth).
pub use pdf_common::dto::{ToolContext, ToolExecutionOptions};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_extraction_result_serialization() {
        let result = TextExtractionResult {
            extracted_text: "Hello World".to_string(),
            extraction_metadata: None,
            metadata: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Hello World"));
        assert!(!json.contains("extraction_metadata"));
    }

    #[test]
    fn test_structured_extraction_result() {
        let result = StructuredExtractionResult {
            extracted_text: "Test".to_string(),
            page_count: 1,
            pages: vec![PageMetadata {
                page_number: 1,
                text: "Test".to_string(),
                bbox: Some((0.0, 0.0, 100.0, 100.0)),
                lines: vec![],
                regions: None,
            }],
            extraction_metadata: None,
            file_info: FileInfo {
                file_path: "/test.pdf".to_string(),
                file_size: 1024,
                file_size_mb: 0.0,
            },
            metadata: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: StructuredExtractionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.page_count, 1);
    }
}
