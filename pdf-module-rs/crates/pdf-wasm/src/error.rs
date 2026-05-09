//! Error types for WASM PDF operations.

use thiserror::Error;

/// Errors that can occur during WASM PDF processing.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WasmError {
    #[error("PDFium initialization failed: {0}")]
    InitError(String),

    #[error("Failed to load PDF: {0}")]
    LoadError(String),

    #[error("Page access error: {0}")]
    PageError(String),

    #[error("Text extraction failed: {0}")]
    ExtractionError(String),

    #[error("Render failed: {0}")]
    RenderError(String),

    #[error("Arena allocation failed: {0}")]
    AllocationError(String),

    #[error("Invalid slice: {0}")]
    InvalidSlice(String),
}

#[cfg(feature = "wasm")]
impl From<WasmError> for wasm_bindgen::JsValue {
    fn from(err: WasmError) -> Self {
        js_sys::Error::new(&err.to_string()).into()
    }
}
