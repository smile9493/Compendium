//! PDF engine trait definition
//! Corresponds to Python: adapters/base.py

use async_trait::async_trait;
use std::path::Path;

use crate::dto::{ExtractOptions, StructuredExtractionResult, TextExtractionResult};
use crate::error::PdfResult;

/// PDF extraction engine trait
/// Corresponds to Python: X2TextAdapter (adapters/base.py)
#[async_trait]
pub trait PdfEngine: Send + Sync {
    /// Engine unique identifier
    fn id(&self) -> &str;

    /// Engine display name
    fn name(&self) -> &str;

    /// Engine description
    fn description(&self) -> &str;

    /// Extract plain text from PDF
    /// Corresponds to Python: X2TextAdapter.process()
    async fn extract_text(&self, file_path: &Path) -> PdfResult<TextExtractionResult>;

    /// Extract structured data with page info and positions
    /// Corresponds to Python: PyMuPDFAdapter.process_structured()
    async fn extract_structured(
        &self,
        file_path: &Path,
        options: &ExtractOptions,
    ) -> PdfResult<StructuredExtractionResult>;

    /// Get page count
    /// Corresponds to Python: PyMuPDFAdapter.get_page_count()
    async fn get_page_count(&self, file_path: &Path) -> PdfResult<u32>;
}
