//! VLM-enhanced PDF extraction pipeline.
//!
//! Integrates `vlm-visual-gateway` with the existing Pdfium-based pipeline,
//! implementing the multi-model smart routing strategy:
//!
//! ```text
//! PDF Page → Complexity Assessment → Smart Model Selection
//!     ├─ Simple (text > 500 chars)    → GLM-4.6V-Flash (free)
//!     ├─ Moderate (50-500 chars)      → GLM-4.6V-FlashX (lightweight)
//!     └─ Complex (<50 chars, scanned) → GLM-4.6V (high performance)
//! ```

use std::path::Path;
use std::sync::Arc;

use tracing::{info, warn};

use vlm_visual_gateway::{
    DetectionResult, EscalationDetector, LayoutResult, MetricsCollector, PageComplexity,
    PayloadMetadata, VlmConfig, VlmGateway, VlmModel,
};

use crate::config::ServerConfig;
use crate::dto::{ExtractOptions, PageMetadata, StructuredExtractionResult, TextExtractionResult};
use crate::engine::{PdfEngine, PdfiumEngine};
use crate::error::{PdfModuleError, PdfResult};
use crate::validator::FileValidator;

/// Configuration for the VLM-enhanced pipeline.
#[derive(Debug, Clone)]
pub struct VlmPipelineConfig {
    pub vlm_enabled: bool,
    pub render_dpi: f32,
}

impl Default for VlmPipelineConfig {
    fn default() -> Self {
        Self {
            vlm_enabled: false,
            render_dpi: 150.0,
        }
    }
}

/// Enhanced extraction result that may include VLM layout understanding.
#[derive(Debug, Clone)]
pub struct VlmEnhancedResult {
    pub base_result: StructuredExtractionResult,
    pub layout_results: Vec<Option<LayoutResult>>,
    pub vlm_triggered: bool,
    pub model_usage: Vec<(PageComplexity, VlmModel)>,
}

/// VLM-enhanced PDF extraction pipeline.
///
/// Combines local Pdfium extraction with Vision Language Model (VLM)
/// enhancement for complex layouts that require visual understanding.
///
/// # Smart Model Selection
///
/// The pipeline automatically selects the appropriate VLM model based on
/// page complexity:
///
/// - **Simple** (text > 500 chars) → GLM-4.6V-Flash (free)
/// - **Moderate** (50-500 chars) → GLM-4.6V-FlashX (lightweight)
/// - **Complex** (<50 chars, scanned) → GLM-4.6V (high performance)
///
/// # Features
///
/// - **Automatic escalation**: Detect when VLM is needed
/// - **Multi-model routing**: Cost-optimal model selection
/// - **Graceful degradation**: Falls back to local extraction on VLM failure
/// - **Metrics collection**: Track VLM usage and performance
///
/// # Example
///
/// ```ignore
/// use pdf_core::{VlmEnhancedPipeline, ServerConfig};
/// use pdf_core::dto::ExtractOptions;
/// use vlm_visual_gateway::VlmConfig;
/// use std::path::Path;
///
/// let config = ServerConfig::from_env()?;
/// let vlm_config = VlmConfig::from_env().ok();
/// let pipeline = VlmEnhancedPipeline::new(&config, vlm_config)?;
///
/// // Extract with VLM enhancement
/// let path = Path::new("document.pdf");
/// let options = ExtractOptions::default();
/// let result = pipeline.extract_structured(path, &options).await?;
/// if result.vlm_triggered {
///     println!("VLM enhanced {} pages", result.layout_results.len());
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct VlmEnhancedPipeline {
    engine: PdfiumEngine,
    validator: FileValidator,
    detector: EscalationDetector,
    gateway: Option<VlmGateway>,
    metrics: Arc<MetricsCollector>,
    config: VlmPipelineConfig,
}

impl VlmEnhancedPipeline {
    pub fn new(server_config: &ServerConfig, vlm_config: Option<VlmConfig>) -> PdfResult<Self> {
        let detector = EscalationDetector::default();
        let metrics = Arc::new(MetricsCollector::with_default_registry());

        let gateway = match vlm_config {
            Some(config) => Some(
                VlmGateway::new(config, Arc::clone(&metrics))
                    .map_err(|e| PdfModuleError::Config(format!("VLM gateway: {e}")))?,
            ),
            None => None,
        };

        let vlm_enabled = gateway.is_some();

        Ok(Self {
            engine: PdfiumEngine::new()?,
            validator: FileValidator::new(server_config.security.max_file_size_mb as u32),
            detector,
            gateway,
            metrics,
            config: VlmPipelineConfig {
                vlm_enabled,
                ..Default::default()
            },
        })
    }

    pub fn with_config(
        server_config: &ServerConfig,
        vlm_config: Option<VlmConfig>,
        pipeline_config: VlmPipelineConfig,
    ) -> PdfResult<Self> {
        let mut pipeline = Self::new(server_config, vlm_config)?;
        pipeline.config = pipeline_config;
        Ok(pipeline)
    }

    pub fn local_only(server_config: &ServerConfig) -> PdfResult<Self> {
        Self::new(server_config, None)
    }

    #[tracing::instrument(skip(self))]
    pub async fn extract_text(&self, file_path: &Path) -> PdfResult<TextExtractionResult> {
        self.validator.validate(file_path)?;
        self.engine.extract_text(file_path).await
    }

    #[tracing::instrument(skip(self, options))]
    pub async fn extract_structured(
        &self,
        file_path: &Path,
        options: &ExtractOptions,
    ) -> PdfResult<VlmEnhancedResult> {
        self.validator.validate(file_path)?;

        let base_result = self.engine.extract_structured(file_path, options).await?;

        if !self.config.vlm_enabled || self.gateway.is_none() {
            let page_count = base_result.pages.len();
            return Ok(VlmEnhancedResult {
                base_result,
                layout_results: vec![None; page_count],
                vlm_triggered: false,
                model_usage: vec![],
            });
        }

        let mut layout_results: Vec<Option<LayoutResult>> = Vec::new();
        let mut vlm_triggered = false;
        let mut model_usage: Vec<(PageComplexity, VlmModel)> = Vec::new();

        for page in &base_result.pages {
            let text_len = page.text.len() as u32;
            let layout_confidence = self.estimate_layout_confidence(page);
            let complexity = Self::assess_page_complexity(text_len, layout_confidence);

            let model = if self
                .gateway
                .as_ref()
                .expect("gateway should be initialized")
                .config()
                .enable_multi_model_routing
            {
                VlmModel::select_for_complexity(complexity)
            } else {
                self.gateway
                    .as_ref()
                    .expect("gateway should be initialized")
                    .config()
                    .model
            };

            info!(
                page_number = page.page_number,
                complexity = ?complexity,
                model = model.model_id(),
                text_len = text_len,
                layout_confidence = layout_confidence,
                "Smart routing selected"
            );

            model_usage.push((complexity, model));

            let extraction = vlm_visual_gateway::PdfiumExtraction {
                character_count: text_len,
                layout_confidence,
                text: page.text.clone(),
                page_width: page.bbox.map(|b| b.2 as f32).unwrap_or(612.0),
                page_height: page.bbox.map(|b| b.3 as f32).unwrap_or(792.0),
            };

            let detection = self.detector.detect(&extraction);

            match detection {
                DetectionResult::Normal => {
                    layout_results.push(None);
                }
                DetectionResult::ZeroText | DetectionResult::LayoutChaos => {
                    vlm_triggered = true;
                    info!(
                        page_number = page.page_number,
                        detection = ?detection,
                        model = model.model_id(),
                        "VLM escalation triggered"
                    );
                    layout_results.push(None);
                }
            }
        }

        Ok(VlmEnhancedResult {
            base_result,
            layout_results,
            vlm_triggered,
            model_usage,
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_page_count(&self, file_path: &Path) -> PdfResult<u32> {
        self.validator.validate(file_path)?;
        self.engine.get_page_count(file_path).await
    }

    pub fn metrics(&self) -> &Arc<MetricsCollector> {
        &self.metrics
    }

    pub fn gateway(&self) -> Option<&VlmGateway> {
        self.gateway.as_ref()
    }

    /// Render a page and send to VLM for layout understanding.
    ///
    /// Data flow:
    ///   Pdfium render → raw RGBA Vec<u8> → perceive_layout (internal Base64) → VLM API
    #[tracing::instrument(skip(self))]
    pub async fn perceive_page(
        &self,
        file_path: &Path,
        page_index: u32,
    ) -> PdfResult<Option<LayoutResult>> {
        let gateway = self
            .gateway
            .as_ref()
            .ok_or_else(|| PdfModuleError::Config("VLM gateway not configured".into()))?;

        let data = std::fs::read(file_path)?;

        // render_page_pixels returns raw RGBA bytes — no encoding yet
        let (pixels, width, height) =
            vlm_visual_gateway::render_page_pixels(&data, page_index, self.config.render_dpi)
                .map_err(|e| PdfModuleError::Extraction(format!("render page: {e}")))?;

        let metadata = PayloadMetadata {
            page_width: width as f32,
            page_height: height as f32,
            page_number: page_index + 1,
        };

        // perceive_layout accepts raw RGBA bytes and does Base64 internally
        match gateway.perceive_layout(&pixels, None, &metadata).await {
            Ok(layout) => Ok(Some(layout)),
            Err(e) => {
                warn!("VLM perceive failed: {e} - degrading to local");
                Ok(None)
            }
        }
    }

    /// Assess page complexity and return the appropriate level.
    ///
    /// # Complexity Levels
    /// - **Simple**: >500 chars, good layout confidence (>0.7)
    /// - **Moderate**: 50-500 chars, moderate confidence (0.3-0.7)
    /// - **Complex**: <50 chars, low confidence (<0.3) — likely scanned/image
    fn assess_page_complexity(text_len: u32, layout_confidence: f32) -> PageComplexity {
        if text_len > 500 && layout_confidence > 0.7 {
            PageComplexity::Simple
        } else if text_len >= 50 && layout_confidence >= 0.3 {
            PageComplexity::Moderate
        } else {
            PageComplexity::Complex
        }
    }

    fn estimate_layout_confidence(&self, page: &PageMetadata) -> f32 {
        let line_count = page.lines.len() as f32;
        let text_len = page.text.len() as f32;

        if text_len < 10.0 {
            return 0.0;
        }

        let line_score = (line_count / 10.0).min(1.0);
        let text_score = (text_len / 500.0).min(1.0);

        (line_score * 0.5 + text_score * 0.5).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_local_only() {
        let config = ServerConfig::default();
        let pipeline = VlmEnhancedPipeline::local_only(&config).unwrap();
        assert!(!pipeline.config.vlm_enabled);
        assert!(pipeline.gateway.is_none());
    }

    #[test]
    fn test_layout_confidence_estimation() {
        let config = ServerConfig::default();
        let pipeline = VlmEnhancedPipeline::local_only(&config).unwrap();

        let page = PageMetadata {
            page_number: 1,
            text: String::new(),
            bbox: Some((0.0, 0.0, 612.0, 792.0)),
            lines: vec![],
            regions: None,
        };
        assert_eq!(pipeline.estimate_layout_confidence(&page), 0.0);

        let page = PageMetadata {
            page_number: 1,
            text: "A".repeat(1000),
            bbox: Some((0.0, 0.0, 612.0, 792.0)),
            lines: (0..20)
                .map(|i| crate::dto::LineInfo {
                    bbox: vec![0.0, i as f64 * 20.0, 500.0, 15.0],
                    text: format!("line {i}"),
                })
                .collect(),
            regions: None,
        };
        assert!(pipeline.estimate_layout_confidence(&page) > 0.8);
    }

    #[test]
    fn test_page_complexity_assessment() {
        // Simple page: lots of text, high confidence
        assert_eq!(
            VlmEnhancedPipeline::assess_page_complexity(1000, 0.9),
            PageComplexity::Simple
        );
        assert_eq!(
            VlmModel::select_for_complexity(PageComplexity::Simple),
            VlmModel::Glm46vFlash
        );

        // Moderate page: some text, moderate confidence
        assert_eq!(
            VlmEnhancedPipeline::assess_page_complexity(200, 0.5),
            PageComplexity::Moderate
        );
        assert_eq!(
            VlmModel::select_for_complexity(PageComplexity::Moderate),
            VlmModel::Glm46vFlashX
        );

        // Complex page: little/no text, low confidence (scanned)
        assert_eq!(
            VlmEnhancedPipeline::assess_page_complexity(10, 0.1),
            PageComplexity::Complex
        );
        assert_eq!(
            VlmModel::select_for_complexity(PageComplexity::Complex),
            VlmModel::Glm46v
        );
    }
}
