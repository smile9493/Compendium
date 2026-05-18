//! VLM vision backend wrapping `vlm-visual-gateway`.

#[cfg(feature = "vlm")]
use async_trait::async_trait;
#[cfg(feature = "vlm")]
use std::sync::Arc;

#[cfg(feature = "vlm")]
use super::{ExtractionBackend, ExtractionCapabilities};
#[cfg(feature = "vlm")]
use crate::dto::TextExtractionResult;
#[cfg(feature = "vlm")]
use crate::engine::PdfiumEngine;
#[cfg(feature = "vlm")]
use crate::error::{PdfModuleError, PdfResult};
#[cfg(feature = "vlm")]
use crate::extractor::ExtractionContext;
#[cfg(feature = "vlm")]
use tracing::warn;
#[cfg(feature = "vlm")]
use vlm_visual_gateway::VlmGateway;

/// VLM layout/OCR backend.
#[cfg(feature = "vlm")]
pub struct VlmExtractionBackend {
    gateway: Arc<VlmGateway>,
}

#[cfg(feature = "vlm")]
impl VlmExtractionBackend {
    pub fn new(gateway: Arc<VlmGateway>) -> Self {
        Self { gateway }
    }
}

#[cfg(feature = "vlm")]
#[async_trait]
impl ExtractionBackend for VlmExtractionBackend {
    fn id(&self) -> &str {
        "vlm"
    }

    fn capabilities(&self) -> ExtractionCapabilities {
        ExtractionCapabilities::VLM
    }

    async fn extract_text(&self, ctx: &ExtractionContext) -> PdfResult<TextExtractionResult> {
        let pdf_data = ctx.loader.as_bytes();
        let page_count = PdfiumEngine::get_page_count_from_mmap(&ctx.loader)?;

        let mut all_text = String::new();
        let mut pages_processed = 0u32;

        for page_idx in 0..page_count {
            match extract_page(&self.gateway, pdf_data, page_idx).await {
                Ok(page_text) if !page_text.trim().is_empty() => {
                    all_text.push_str(&page_text);
                    all_text.push_str("\n\n");
                    pages_processed += 1;
                }
                Ok(_) => {}
                Err(e) => {
                    warn!(page = page_idx, error = %e, "VLM page extraction failed");
                }
            }
        }

        if all_text.trim().is_empty() {
            return PdfiumBackendFallback.extract_text(ctx).await;
        }

        Ok(TextExtractionResult {
            extracted_text: all_text,
            extraction_metadata: None,
            metadata: Some(serde_json::json!({
                "method": "vlm",
                "pages_processed": pages_processed,
                "total_pages": page_count
            })),
        })
    }
}

#[cfg(feature = "vlm")]
struct PdfiumBackendFallback;

#[cfg(feature = "vlm")]
#[async_trait]
impl ExtractionBackend for PdfiumBackendFallback {
    fn id(&self) -> &str {
        "pdfium"
    }

    fn capabilities(&self) -> ExtractionCapabilities {
        ExtractionCapabilities::PDFIUM
    }

    async fn extract_text(&self, ctx: &ExtractionContext) -> PdfResult<TextExtractionResult> {
        let text = PdfiumEngine::extract_text_from_mmap(&ctx.loader)?;
        Ok(TextExtractionResult {
            extracted_text: text,
            extraction_metadata: None,
            metadata: Some(serde_json::json!({"method": "pdfium_fallback"})),
        })
    }
}

#[cfg(feature = "vlm")]
async fn extract_page(
    gateway: &VlmGateway,
    pdf_data: &[u8],
    page_idx: u32,
) -> PdfResult<String> {
    let (rgba, width, height) =
        vlm_visual_gateway::render_page_pixels(pdf_data, page_idx, 150.0).map_err(|e| {
            PdfModuleError::Extraction(format!("Render page {page_idx}: {e}"))
        })?;

    let metadata = vlm_visual_gateway::types::PayloadMetadata {
        page_width: width as f32,
        page_height: height as f32,
        page_number: page_idx + 1,
    };

    let layout = gateway
        .perceive_layout(&rgba, None, &metadata)
        .await
        .map_err(|e| PdfModuleError::Extraction(format!("VLM perceive: {e}")))?;

    let mut page_text = String::new();
    for region in &layout.regions {
        if !region.content.is_empty() {
            page_text.push_str(&region.content);
            page_text.push('\n');
        }
    }
    Ok(page_text)
}
