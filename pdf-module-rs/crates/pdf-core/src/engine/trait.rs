//! PDF engine trait definition
//! Corresponds to Python: adapters/base.py

use async_trait::async_trait;
use futures::Stream;
use std::path::Path;
use std::pin::Pin;

use crate::dto::{ExtractOptions, PageMetadata, StructuredExtractionResult, TextExtractionResult};
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

    /// Stream pages one by one (for large PDFs)
    /// Returns an async iterator for memory-efficient processing
    async fn extract_page_stream(
        &self,
        #[allow(unused_variables)] file_path: &Path,
        #[allow(unused_variables)] options: &ExtractOptions,
    ) -> PdfResult<Pin<Box<dyn Stream<Item = PdfResult<PageMetadata>> + Send>>> {
        // Default implementation returns error
        Err(crate::error::PdfModuleError::Extraction(
            "Stream extraction not implemented for this engine".to_string(),
        ))
    }

    /// Test engine availability
    /// Corresponds to Python: X2TextAdapter.test_connection()
    fn test_connection(&self) -> bool {
        false
    }
}
