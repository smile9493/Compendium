use crate::config::ServerConfig;
use crate::dto::{ExtractOptions, StructuredExtractionResult, TextExtractionResult};
use crate::engine::PdfiumEngine;
use crate::error::PdfResult;
use crate::mmap_loader::MmapPdfLoader;
use crate::quality_probe::{ExtractionMethod, QualityProbe, QualityReport};
use crate::validator::FileValidator;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};
use vlm_visual_gateway::{MetricsCollector, VlmConfig, VlmGateway};

/// MCP PDF processing pipeline with optional VLM enhancement.
///
/// This is the main entry point for PDF extraction in the MCP server.
/// It combines local Pdfium-based extraction with optional VLM (Vision Language Model)
/// enhancement for complex layouts.
///
/// # Architecture
///
/// ```text
/// ┌─────────────────┐
/// │ McpPdfPipeline  │
/// ├─────────────────┤
/// │ - FileValidator │ → PDF validation and type detection
/// │ - VlmGateway    │ → Optional VLM enhancement
/// │ - MetricsCollector │ → Prometheus metrics
/// └─────────────────┘
///          │
///          ▼
/// ┌─────────────────┐
/// │ PdfiumEngine    │
/// ├─────────────────┤
/// │ - mmap loading  │ → Zero-copy file access
/// │ - text extract  │ → Page-by-page extraction
/// │ - structured    │ → Bounding box extraction
/// └─────────────────┘
/// ```
pub struct McpPdfPipeline {
    validator: FileValidator,
    vlm_gateway: Option<VlmGateway>,
}

/// Context for a single PDF extraction operation.
pub struct ExtractionContext {
    pub quality_report: QualityReport,
    pub loader: MmapPdfLoader,
}

impl McpPdfPipeline {
    pub fn new(config: &ServerConfig) -> PdfResult<Self> {
        let metrics = Arc::new(MetricsCollector::with_default_registry());

        let vlm_gateway = match VlmConfig::from_env() {
            Ok(vlm_config) => match VlmGateway::new(vlm_config, metrics) {
                Ok(gateway) => {
                    info!("VLM gateway initialized successfully");
                    Some(gateway)
                }
                Err(e) => {
                    warn!(
                        "Failed to initialize VLM gateway: {} - operating in local-only mode",
                        e
                    );
                    None
                }
            },
            Err(_) => {
                info!("VLM not configured - operating in local-only mode");
                None
            }
        };

        Ok(Self {
            validator: FileValidator::new(config.security.max_file_size_mb as u32),
            vlm_gateway,
        })
    }

    pub fn with_vlm(config: &ServerConfig, vlm_config: VlmConfig) -> PdfResult<Self> {
        let metrics = Arc::new(MetricsCollector::with_default_registry());
        let vlm_gateway = VlmGateway::new(vlm_config, metrics)
            .map_err(|e| crate::error::PdfModuleError::Config(format!("VLM gateway: {}", e)))?;

        Ok(Self {
            validator: FileValidator::new(config.security.max_file_size_mb as u32),
            vlm_gateway: Some(vlm_gateway),
        })
    }

    fn probe_and_load(&self, file_path: &Path) -> PdfResult<ExtractionContext> {
        self.validator.validate(file_path)?;
        let loader = MmapPdfLoader::load(file_path)?;
        let quality_report = QualityProbe::probe_with_pdfium(loader.as_bytes())?;

        info!(
            file = ?file_path,
            quality = ?quality_report.quality,
            text_density = quality_report.text_density,
            needs_vlm = quality_report.needs_vlm,
            extraction_method = ?quality_report.extraction_method,
            "PDF quality analysis complete"
        );

        Ok(ExtractionContext {
            quality_report,
            loader,
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn extract_text(&self, file_path: &Path) -> PdfResult<TextExtractionResult> {
        let ctx = self.probe_and_load(file_path)?;

        match ctx.quality_report.extraction_method {
            ExtractionMethod::Pdfium => {
                let text = PdfiumEngine::extract_text_from_mmap(&ctx.loader)?;
                Ok(TextExtractionResult {
                    extracted_text: text,
                    extraction_metadata: None,
                    metadata: None,
                })
            }
            ExtractionMethod::Vlm => self.extract_text_via_vlm(&ctx).await,
            ExtractionMethod::Hybrid => self.extract_text_hybrid(&ctx).await,
        }
    }

    async fn extract_text_via_vlm(
        &self,
        ctx: &ExtractionContext,
    ) -> PdfResult<TextExtractionResult> {
        let Some(ref gateway) = self.vlm_gateway else {
            warn!("VLM extraction requested but no gateway available, falling back to Pdfium");
            let text = PdfiumEngine::extract_text_from_mmap(&ctx.loader)?;
            return Ok(TextExtractionResult {
                extracted_text: text,
                extraction_metadata: None,
                metadata: Some(serde_json::json!({
                    "method": "pdfium_fallback",
                    "reason": "no_vlm_gateway"
                })),
            });
        };

        let pdf_data = ctx.loader.as_bytes();
        let page_count = PdfiumEngine::get_page_count_from_mmap(&ctx.loader)?;

        let mut all_text = String::new();
        let mut pages_processed = 0u32;

        for page_idx in 0..page_count {
            match self
                .extract_page_text_via_vlm(gateway, pdf_data, page_idx)
                .await
            {
                Ok(page_text) => {
                    all_text.push_str(&page_text);
                    all_text.push_str("\n\n");
                    pages_processed += 1;
                }
                Err(e) => {
                    warn!(page = page_idx, error = %e, "VLM extraction failed for page, skipping");
                }
            }
        }

        if all_text.trim().is_empty() {
            warn!("VLM extraction produced no text, falling back to Pdfium");
            let text = PdfiumEngine::extract_text_from_mmap(&ctx.loader)?;
            return Ok(TextExtractionResult {
                extracted_text: text,
                extraction_metadata: None,
                metadata: Some(serde_json::json!({
                    "method": "pdfium_fallback",
                    "reason": "vlm_empty_result"
                })),
            });
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

    async fn extract_page_text_via_vlm(
        &self,
        gateway: &VlmGateway,
        pdf_data: &[u8],
        page_idx: u32,
    ) -> PdfResult<String> {
        let (rgba, width, height) =
            vlm_visual_gateway::render_page_pixels(pdf_data, page_idx, 150.0).map_err(|e| {
                crate::error::PdfModuleError::Extraction(format!("Render page {}: {}", page_idx, e))
            })?;

        let metadata = vlm_visual_gateway::types::PayloadMetadata {
            page_width: width as f32,
            page_height: height as f32,
            page_number: page_idx + 1,
        };

        let layout = gateway
            .perceive_layout(&rgba, None, &metadata)
            .await
            .map_err(|e| {
                crate::error::PdfModuleError::Extraction(format!("VLM perceive: {}", e))
            })?;

        let mut page_text = String::new();
        for region in &layout.regions {
            if !region.content.is_empty() {
                page_text.push_str(&region.content);
                page_text.push('\n');
            }
        }

        Ok(page_text)
    }

    async fn extract_text_hybrid(
        &self,
        ctx: &ExtractionContext,
    ) -> PdfResult<TextExtractionResult> {
        let pdfium_text = PdfiumEngine::extract_text_from_mmap(&ctx.loader)?;

        let Some(ref gateway) = self.vlm_gateway else {
            return Ok(TextExtractionResult {
                extracted_text: pdfium_text,
                extraction_metadata: None,
                metadata: Some(serde_json::json!({
                    "method": "pdfium",
                    "reason": "no_vlm_gateway"
                })),
            });
        };

        let pdfium_len = pdfium_text.chars().count();
        let pdf_data = ctx.loader.as_bytes();
        let page_count = PdfiumEngine::get_page_count_from_mmap(&ctx.loader)?;

        let mut vlm_text = String::new();
        let mut pages_enhanced = 0u32;

        for page_idx in 0..page_count {
            match self
                .extract_page_text_via_vlm(gateway, pdf_data, page_idx)
                .await
            {
                Ok(page_text) if !page_text.trim().is_empty() => {
                    vlm_text.push_str(&page_text);
                    vlm_text.push_str("\n\n");
                    pages_enhanced += 1;
                }
                Ok(_) => {}
                Err(e) => {
                    warn!(page = page_idx, error = %e, "Hybrid: VLM failed for page");
                }
            }
        }

        let vlm_len = vlm_text.chars().count();
        let (final_text, method) = if vlm_len > pdfium_len {
            (vlm_text, "hybrid_vlm_primary")
        } else {
            (pdfium_text, "hybrid_pdfium_primary")
        };

        Ok(TextExtractionResult {
            extracted_text: final_text,
            extraction_metadata: None,
            metadata: Some(serde_json::json!({
                "method": method,
                "pdfium_chars": pdfium_len,
                "vlm_chars": vlm_len,
                "pages_enhanced": pages_enhanced
            })),
        })
    }

    #[tracing::instrument(skip(self, _options))]
    pub async fn extract_structured(
        &self,
        file_path: &Path,
        _options: &ExtractOptions,
    ) -> PdfResult<StructuredExtractionResult> {
        let ctx = self.probe_and_load(file_path)?;

        match ctx.quality_report.extraction_method {
            ExtractionMethod::Pdfium => {
                PdfiumEngine::extract_structured_from_mmap(&ctx.loader, file_path)
            }
            ExtractionMethod::Vlm => self.extract_structured_via_vlm(&ctx, file_path).await,
            ExtractionMethod::Hybrid => self.extract_structured_hybrid(&ctx, file_path).await,
        }
    }

    async fn extract_structured_via_vlm(
        &self,
        ctx: &ExtractionContext,
        file_path: &Path,
    ) -> PdfResult<StructuredExtractionResult> {
        let Some(ref gateway) = self.vlm_gateway else {
            warn!("VLM structured extraction requested but no gateway available, falling back to Pdfium");
            return PdfiumEngine::extract_structured_from_mmap(&ctx.loader, file_path);
        };

        let pdf_data = ctx.loader.as_bytes();
        let page_count = PdfiumEngine::get_page_count_from_mmap(&ctx.loader)?;

        let mut all_pages = Vec::with_capacity(page_count as usize);
        let mut all_text = String::new();

        for page_idx in 0..page_count {
            match self
                .extract_page_structured_via_vlm(gateway, pdf_data, page_idx)
                .await
            {
                Ok((page_text, regions)) => {
                    all_text.push_str(&page_text);
                    all_text.push('\n');

                    all_pages.push(crate::dto::PageMetadata {
                        page_number: page_idx + 1,
                        text: page_text,
                        bbox: None,
                        lines: vec![],
                        regions: Some(regions),
                    });
                }
                Err(e) => {
                    warn!(page = page_idx, error = %e, "VLM structured extraction failed for page");
                    all_pages.push(crate::dto::PageMetadata {
                        page_number: page_idx + 1,
                        text: String::new(),
                        bbox: None,
                        lines: vec![],
                        regions: None,
                    });
                }
            }
        }

        let file_info =
            crate::dto::FileInfo::from_path(file_path).unwrap_or_else(|_| crate::dto::FileInfo {
                file_path: file_path.to_string_lossy().to_string(),
                file_size: 0,
                file_size_mb: 0.0,
            });

        Ok(StructuredExtractionResult {
            pages: all_pages,
            page_count,
            extracted_text: all_text,
            extraction_metadata: None,
            file_info,
            metadata: Some(serde_json::json!({
                "method": "vlm"
            })),
        })
    }

    async fn extract_page_structured_via_vlm(
        &self,
        gateway: &VlmGateway,
        pdf_data: &[u8],
        page_idx: u32,
    ) -> PdfResult<(String, Vec<crate::dto::TextRegion>)> {
        let (rgba, width, height) =
            vlm_visual_gateway::render_page_pixels(pdf_data, page_idx, 150.0).map_err(|e| {
                crate::error::PdfModuleError::Extraction(format!("Render page {}: {}", page_idx, e))
            })?;

        let metadata = vlm_visual_gateway::types::PayloadMetadata {
            page_width: width as f32,
            page_height: height as f32,
            page_number: page_idx + 1,
        };

        let layout = gateway
            .perceive_layout(&rgba, None, &metadata)
            .await
            .map_err(|e| {
                crate::error::PdfModuleError::Extraction(format!("VLM perceive: {}", e))
            })?;

        let mut page_text = String::new();
        let mut regions = Vec::new();

        for region in &layout.regions {
            if !region.content.is_empty() {
                page_text.push_str(&region.content);
                page_text.push('\n');
            }

            regions.push(crate::dto::TextRegion {
                region_type: match region.region_type {
                    vlm_visual_gateway::types::RegionType::Title => "title".to_string(),
                    vlm_visual_gateway::types::RegionType::Body => "body".to_string(),
                    vlm_visual_gateway::types::RegionType::Table => "table".to_string(),
                    vlm_visual_gateway::types::RegionType::Image => "image".to_string(),
                    vlm_visual_gateway::types::RegionType::Caption => "caption".to_string(),
                },
                bbox: crate::dto::BoundingBox {
                    x: region.bbox.x,
                    y: region.bbox.y,
                    width: region.bbox.width,
                    height: region.bbox.height,
                },
                text: region.content.clone(),
            });
        }

        Ok((page_text, regions))
    }

    async fn extract_structured_hybrid(
        &self,
        ctx: &ExtractionContext,
        file_path: &Path,
    ) -> PdfResult<StructuredExtractionResult> {
        let mut result = PdfiumEngine::extract_structured_from_mmap(&ctx.loader, file_path)?;

        let Some(ref gateway) = self.vlm_gateway else {
            return Ok(result);
        };

        let pdf_data = ctx.loader.as_bytes();
        let mut enhanced_pages = 0u32;

        for page in &mut result.pages {
            if page.text.chars().count() < 100 {
                if let Ok((vlm_text, vlm_regions)) = self
                    .extract_page_structured_via_vlm(gateway, pdf_data, page.page_number - 1)
                    .await
                {
                    if vlm_text.chars().count() > page.text.chars().count() {
                        page.text = vlm_text;
                        page.regions = Some(vlm_regions);
                        enhanced_pages += 1;
                    }
                }
            }
        }

        result.metadata = Some(serde_json::json!({
            "method": "hybrid",
            "enhanced_pages": enhanced_pages
        }));

        Ok(result)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_page_count(&self, file_path: &Path) -> PdfResult<u32> {
        let ctx = self.probe_and_load(file_path)?;
        PdfiumEngine::get_page_count_from_mmap(&ctx.loader)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        let config = ServerConfig::default();
        let pipeline = McpPdfPipeline::new(&config).unwrap();
        assert!(pipeline.vlm_gateway.is_none());
    }
}
