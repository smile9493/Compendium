use async_trait::async_trait;
use std::env;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::sync::Arc;
use std::sync::LazyLock;

use crate::dto::{
    ExtractOptions, FileInfo, PageMetadata, StructuredExtractionResult, TextExtractionResult,
};
use crate::engine::PdfEngine;
use crate::error::{PdfModuleError, PdfResult};
use crate::mmap_loader::MmapPdfLoader;

pub struct PdfiumEngine;

struct StructuredParts {
    extracted_text: String,
    page_count: u32,
    pages: Vec<PageMetadata>,
}

static PDFIUM: LazyLock<Option<Arc<pdfium_render::prelude::Pdfium>>> = LazyLock::new(|| {
    if let Ok(path) = env::var("PDFIUM_LIB_PATH") {
        if Path::new(&path).exists() {
            if let Ok(bindings) = pdfium_render::prelude::Pdfium::bind_to_library(&path) {
                return Some(Arc::new(pdfium_render::prelude::Pdfium::new(bindings)));
            }
        }
    }
    // Reload the pdfium binary using the system library path
    pdfium_render::prelude::Pdfium::bind_to_system_library()
        .ok()
        .map(|b| Arc::new(pdfium_render::prelude::Pdfium::new(b)))
});

impl PdfiumEngine {
    pub fn new() -> PdfResult<Self> {
        Ok(Self)
    }

    pub fn get_pdfium() -> PdfResult<Arc<pdfium_render::prelude::Pdfium>> {
        PDFIUM.clone().ok_or_else(|| {
            PdfModuleError::Extraction(
                "Failed to load pdfium. Set PDFIUM_LIB_PATH environment variable.".to_string(),
            )
        })
    }

    pub fn extract_text_from_mmap(loader: &MmapPdfLoader) -> PdfResult<String> {
        let data = loader.as_bytes();
        Self::safe_extract_text(data)
    }

    pub fn extract_structured_from_mmap(
        loader: &MmapPdfLoader,
        file_path: &Path,
    ) -> PdfResult<StructuredExtractionResult> {
        let data = loader.as_bytes();
        let parts = Self::safe_extract_structured_parts(data)?;
        let file_info = FileInfo::from_path(file_path)?;
        Ok(StructuredExtractionResult {
            extracted_text: parts.extracted_text,
            page_count: parts.page_count,
            pages: parts.pages,
            extraction_metadata: None,
            file_info,
            metadata: None,
        })
    }

    pub fn get_page_count_from_mmap(loader: &MmapPdfLoader) -> PdfResult<u32> {
        let data = loader.as_bytes();
        Self::safe_get_page_count(data)
    }

    pub fn safe_extract_text(data: &[u8]) -> PdfResult<String> {
        use pdfium_render::prelude::*;

        let pdfium = Self::get_pdfium()?;

        // SAFETY: AssertUnwindSafe is used here because Pdfium FFI calls may panic.
        // If Pdfium's internal C++ state is corrupted by a panic, subsequent calls
        // through the same Arc<Pdfium> would be unsound (use-after-free or double-free
        // on Pdfium's internal structures). This is an accepted trade-off:
        // - Pdfium panics are rare in practice (mostly OOM or invalid document data)
        // - The PdfiumEngine is stateless - no in-process state survives a panic
        // - On panic, the error propagates up and the calling task/cxn resets
        // - If Pdfium process-level state is corrupted, the container restart handles it
        // This matches the standard FFI panic containment pattern from ref/15-ffi-interop.
        catch_unwind(AssertUnwindSafe(|| {
            let document = pdfium.load_pdf_from_byte_slice(data, None)?;
            let mut all_text = String::new();
            let pages = document.pages();
            for i in 0..pages.len() {
                let page = pages.get(i)?;
                let text = page.text()?;
                all_text.push_str(&text.all());
                all_text.push('\n');
            }
            Ok::<String, PdfiumError>(all_text.trim().to_string())
        }))
        .map_err(|_| PdfModuleError::Extraction("Pdfium FFI panic".to_string()))?
        .map_err(|e| PdfModuleError::Extraction(format!("Pdfium error: {}", e)))
    }

    fn safe_extract_structured_parts(data: &[u8]) -> PdfResult<StructuredParts> {
        use pdfium_render::prelude::*;

        let pdfium = Self::get_pdfium()?;

        // SAFETY: See comment in safe_extract_text for rationale on AssertUnwindSafe.
        // Same trade-off applies: PDFium FFI panic is contained, error propagated.
        catch_unwind(AssertUnwindSafe(|| {
            let document = pdfium.load_pdf_from_byte_slice(data, None)?;
            let pages = document.pages();
            let page_count = pages.len() as u32;
            let mut all_text = String::new();
            let mut page_metas = Vec::with_capacity(page_count as usize);

            for i in 0..pages.len() {
                let page = pages.get(i)?;
                let text = page.text()?;
                let text_str = text.all();
                let width = page.width().value as f64;
                let height = page.height().value as f64;

                page_metas.push(PageMetadata {
                    page_number: (i + 1) as u32,
                    text: text_str.trim().to_string(),
                    bbox: Some((0.0, 0.0, width, height)),
                    lines: vec![],
                    regions: None,
                });
                all_text.push_str(&text_str);
                all_text.push('\n');
            }

            Ok::<StructuredParts, PdfiumError>(StructuredParts {
                extracted_text: all_text.trim().to_string(),
                page_count,
                pages: page_metas,
            })
        }))
        .map_err(|_| PdfModuleError::Extraction("Pdfium FFI panic".to_string()))?
        .map_err(|e| PdfModuleError::Extraction(format!("Pdfium error: {}", e)))
    }

    pub fn safe_get_page_count(data: &[u8]) -> PdfResult<u32> {
        use pdfium_render::prelude::*;

        let pdfium = Self::get_pdfium()?;

        // SAFETY: See comment in safe_extract_text for rationale on AssertUnwindSafe.
        // Same trade-off applies: PDFium FFI panic is contained, error propagated.
        catch_unwind(AssertUnwindSafe(|| {
            let document = pdfium.load_pdf_from_byte_slice(data, None)?;
            Ok::<u32, PdfiumError>(document.pages().len() as u32)
        }))
        .map_err(|_| PdfModuleError::Extraction("Pdfium FFI panic".to_string()))?
        .map_err(|e| PdfModuleError::Extraction(format!("Pdfium error: {}", e)))
    }
}

impl Default for PdfiumEngine {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl PdfEngine for PdfiumEngine {
    fn id(&self) -> &str {
        "pdfium"
    }

    fn name(&self) -> &str {
        "PdfiumEngine"
    }

    fn description(&self) -> &str {
        "Pdfium engine with catch_unwind FFI seawall"
    }

    async fn extract_text(&self, file_path: &Path) -> PdfResult<TextExtractionResult> {
        let data = std::fs::read(file_path)?;
        let text = Self::safe_extract_text(&data)?;
        Ok(TextExtractionResult {
            extracted_text: text,
            extraction_metadata: None,
            metadata: None,
        })
    }

    async fn extract_structured(
        &self,
        file_path: &Path,
        _options: &ExtractOptions,
    ) -> PdfResult<StructuredExtractionResult> {
        let data = std::fs::read(file_path)?;
        let parts = Self::safe_extract_structured_parts(&data)?;
        let file_info = FileInfo::from_path(file_path)?;
        Ok(StructuredExtractionResult {
            extracted_text: parts.extracted_text,
            page_count: parts.page_count,
            pages: parts.pages,
            extraction_metadata: None,
            file_info,
            metadata: None,
        })
    }

    async fn get_page_count(&self, file_path: &Path) -> PdfResult<u32> {
        let data = std::fs::read(file_path)?;
        Self::safe_get_page_count(&data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_metadata() {
        let engine = PdfiumEngine::new().unwrap();
        assert_eq!(engine.id(), "pdfium");
        assert!(engine.test_connection());
    }
}
