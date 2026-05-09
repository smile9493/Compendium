use thiserror::Error;

/// VLM Gateway error enumeration
#[derive(Debug, Error)]
pub enum VlmError {
    #[error("Request timeout after {0} seconds")]
    Timeout(u64),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Invalid image data: {0}")]
    InvalidImage(String),

    #[error("Response parse error: {0}")]
    ParseError(String),

    #[error("Service unavailable: {0}")]
    Unavailable(String),

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("Configuration error: {0}")]
    Config(String),
}

/// Pdfium FFI error enumeration
#[derive(Debug, Error)]
pub enum PdfiumGuardError {
    #[error("FFI panic occurred")]
    Panic,

    #[error("Lock poisoned")]
    LockPoisoned,

    #[error("Render failed: {0}")]
    RenderFailed(String),

    #[error("Join error: {0}")]
    JoinError(String),
}

/// Result type alias for VLM operations
pub type VlmResult<T> = Result<T, VlmError>;

/// Result type alias for PdfiumGuard operations
pub type PdfiumGuardResult<T> = Result<T, PdfiumGuardError>;
