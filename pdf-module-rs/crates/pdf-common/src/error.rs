//! Unified error type for the PDF module ecosystem.
//!
//! Consolidates error definitions from `pdf-core::PdfModuleError` and
//! `pdf-etl::EtlError` into a single type with consistent behavior.

use serde::{Serialize, Serializer};
use thiserror::Error;

/// Error classification for monitoring and alerting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// File system errors (4xx)
    FileSystem,
    /// Extraction errors (5xx)
    Extraction,
    /// Plugin / tool errors (5xx)
    Plugin,
    /// Configuration errors (5xx)
    Config,
    /// Validation errors (4xx)
    Validation,
    /// Network / HTTP errors (5xx)
    Network,
    /// Database errors (5xx)
    Database,
    /// LLM errors (5xx)
    LLM,
}

/// Unified error type for the entire PDF module workspace.
///
/// Replaces the previous `PdfModuleError` (pdf-core) and `EtlError` (pdf-etl).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PdfError {
    // === File System Errors ===
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid file type: {0}")]
    InvalidFileType(String),

    #[error("File too large: {0}")]
    FileTooLarge(String),

    #[error("Corrupted file: {0}")]
    CorruptedFile(String),

    // === Extraction Errors ===
    #[error("Extraction failed: {0}")]
    Extraction(String),

    #[error("Engine/adapter not found: {0}")]
    AdapterNotFound(String),

    // === Plugin / Tool Errors ===
    #[error("Tool registration failed: {0}")]
    ToolRegistration(String),

    #[error("Tool execution failed: {0}")]
    ToolExecution(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Tool already registered: {0}")]
    ToolAlreadyRegistered(String),

    #[error("Invalid tool definition: {0}")]
    InvalidToolDefinition(String),

    #[error("Plugin load error: {0}")]
    PluginLoad(String),

    #[error("Tool unavailable: {0}")]
    ToolUnavailable(String),

    #[error("Discovery failed: {0}")]
    Discovery(String),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    // === Validation Errors ===
    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Schema validation failed: {0}")]
    SchemaValidation(String),

    // === Configuration Errors ===
    #[error("Configuration error: {0}")]
    Config(String),

    // === Storage / Audit Errors ===
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Audit error: {0}")]
    Audit(String),

    // === Network Errors ===
    #[error("HTTP error: {0}")]
    Http(String),

    // === Database Errors ===
    #[error("Database error: {0}")]
    Database(String),

    // === LLM Errors ===
    #[error("LLM error: {0}")]
    LLM(String),

    // === Parameter Errors ===
    #[error("Missing parameter: {0}")]
    ParameterMissing(String),

    #[error("Invalid parameter type: {0}")]
    ParameterType(String),

    // === Catch-all ===
    #[error("Unknown error: {0}")]
    Unknown(String),

    // === External Error Conversions ===
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl PdfError {
    /// Map error to an HTTP status code.
    pub fn status_code(&self) -> u16 {
        match self {
            // 4xx Client Errors
            Self::FileNotFound(_) => 404,
            Self::InvalidFileType(_) => 400,
            Self::FileTooLarge(_) => 413,
            Self::CorruptedFile(_) => 422,
            Self::AdapterNotFound(_) => 400,
            Self::ToolNotFound(_) => 404,
            Self::ToolAlreadyRegistered(_) => 409,
            Self::Validation(_) => 400,
            Self::SchemaValidation(_) => 400,
            Self::InvalidToolDefinition(_) => 400,
            Self::ParameterMissing(_) => 400,
            Self::ParameterType(_) => 400,
            Self::Timeout(_) => 408,

            // 5xx Server Errors
            Self::Extraction(_) => 500,
            Self::ToolRegistration(_) => 500,
            Self::ToolExecution(_) => 500,
            Self::PluginLoad(_) => 500,
            Self::ToolUnavailable(_) => 503,
            Self::Discovery(_) => 500,
            Self::Config(_) => 500,
            Self::Storage(_) => 500,
            Self::Audit(_) => 500,
            Self::Http(_) => 500,
            Self::Database(_) => 500,
            Self::LLM(_) => 500,
            Self::Io(_) => 500,
            Self::Json(_) => 500,
            Self::Unknown(_) => 500,
        }
    }

    /// Classify the error into a high-level category.
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::FileNotFound(_)
            | Self::InvalidFileType(_)
            | Self::FileTooLarge(_)
            | Self::CorruptedFile(_)
            | Self::Io(_) => ErrorCategory::FileSystem,

            Self::Extraction(_) | Self::AdapterNotFound(_) => ErrorCategory::Extraction,

            Self::ToolRegistration(_)
            | Self::ToolExecution(_)
            | Self::ToolNotFound(_)
            | Self::ToolAlreadyRegistered(_)
            | Self::InvalidToolDefinition(_)
            | Self::PluginLoad(_)
            | Self::ToolUnavailable(_)
            | Self::Discovery(_) => ErrorCategory::Plugin,

            Self::Timeout(_) => ErrorCategory::Network,

            Self::Validation(_)
            | Self::SchemaValidation(_)
            | Self::ParameterMissing(_)
            | Self::ParameterType(_) => ErrorCategory::Validation,

            Self::Config(_) | Self::Storage(_) | Self::Audit(_) | Self::Json(_) => {
                ErrorCategory::Config
            }

            Self::Http(_) => ErrorCategory::Network,
            Self::Database(_) => ErrorCategory::Database,
            Self::LLM(_) => ErrorCategory::LLM,
            Self::Unknown(_) => ErrorCategory::Extraction,
        }
    }

    /// Machine-readable error type name.
    pub fn error_type(&self) -> &'static str {
        match self {
            Self::FileNotFound(_) => "FileNotFoundError",
            Self::InvalidFileType(_) => "InvalidFileTypeError",
            Self::FileTooLarge(_) => "FileTooLargeError",
            Self::CorruptedFile(_) => "CorruptedFileError",
            Self::Extraction(_) => "ExtractionError",
            Self::AdapterNotFound(_) => "AdapterNotFoundError",
            Self::ToolRegistration(_) => "ToolRegistrationError",
            Self::ToolExecution(_) => "ToolExecutionError",
            Self::ToolNotFound(_) => "ToolNotFoundError",
            Self::ToolAlreadyRegistered(_) => "ToolAlreadyRegisteredError",
            Self::InvalidToolDefinition(_) => "InvalidToolDefinitionError",
            Self::PluginLoad(_) => "PluginLoadError",
            Self::ToolUnavailable(_) => "ToolUnavailableError",
            Self::Discovery(_) => "DiscoveryError",
            Self::Timeout(_) => "TimeoutError",
            Self::Validation(_) => "ValidationError",
            Self::SchemaValidation(_) => "SchemaValidationError",
            Self::Config(_) => "ConfigError",
            Self::Storage(_) => "StorageError",
            Self::Audit(_) => "AuditError",
            Self::Http(_) => "HttpError",
            Self::Database(_) => "DatabaseError",
            Self::LLM(_) => "LLMError",
            Self::ParameterMissing(_) => "ParameterMissingError",
            Self::ParameterType(_) => "ParameterTypeError",
            Self::Io(_) => "IoError",
            Self::Json(_) => "JsonError",
            Self::Unknown(_) => "UnknownError",
        }
    }

    /// Serialize to a JSON value for API responses.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "error": self.error_type(),
            "message": self.to_string(),
            "category": self.category(),
            "status_code": self.status_code(),
        })
    }
}

impl Serialize for PdfError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_json().serialize(serializer)
    }
}

/// Convenience type alias.
pub type Result<T> = std::result::Result<T, PdfError>;

impl From<PdfError> for std::io::Error {
    fn from(err: PdfError) -> Self {
        std::io::Error::other(err.to_string())
    }
}

impl From<serde_yaml::Error> for PdfError {
    fn from(err: serde_yaml::Error) -> Self {
        Self::Storage(format!("YAML error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(PdfError::FileNotFound("x".into()).status_code(), 404);
        assert_eq!(PdfError::InvalidFileType("x".into()).status_code(), 400);
        assert_eq!(PdfError::FileTooLarge("x".into()).status_code(), 413);
        assert_eq!(PdfError::CorruptedFile("x".into()).status_code(), 422);
        assert_eq!(PdfError::Extraction("x".into()).status_code(), 500);
        assert_eq!(PdfError::AdapterNotFound("x".into()).status_code(), 400);
        assert_eq!(PdfError::Timeout(5000).status_code(), 408);
        assert_eq!(PdfError::ToolNotFound("x".into()).status_code(), 404);
        assert_eq!(
            PdfError::ToolAlreadyRegistered("x".into()).status_code(),
            409
        );
    }

    #[test]
    fn test_error_category() {
        assert_eq!(
            PdfError::FileNotFound("x".into()).category(),
            ErrorCategory::FileSystem
        );
        assert_eq!(
            PdfError::Extraction("x".into()).category(),
            ErrorCategory::Extraction
        );
        assert_eq!(
            PdfError::ToolExecution("x".into()).category(),
            ErrorCategory::Plugin
        );
        assert_eq!(
            PdfError::Database("x".into()).category(),
            ErrorCategory::Database
        );
        assert_eq!(PdfError::LLM("x".into()).category(), ErrorCategory::LLM);
        assert_eq!(PdfError::Timeout(1000).category(), ErrorCategory::Network);
    }

    #[test]
    fn test_error_to_json() {
        let err = PdfError::FileNotFound("/path/to/file.pdf".into());
        let json = err.to_json();
        assert_eq!(json["error"], "FileNotFoundError");
        assert_eq!(json["status_code"], 404);
        assert_eq!(json["category"], "file_system");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let pdf_err: PdfError = io_err.into();
        assert!(matches!(pdf_err, PdfError::Io(_)));
    }

    #[test]
    fn test_json_error_conversion() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let pdf_err: PdfError = json_err.into();
        assert!(matches!(pdf_err, PdfError::Json(_)));
    }
}
