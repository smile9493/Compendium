//! Pluggable PDF extraction backends and priority routing.

pub mod remote;
#[cfg(feature = "vlm")]
pub mod vlm;

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;

use crate::dto::TextExtractionResult;
use crate::engine::PdfiumEngine;
use crate::error::PdfResult;
use crate::extractor::ExtractionContext;
use crate::quality_probe::{ExtractionMethod, QualityProbe};
use tracing::warn;

/// Capabilities advertised by an extraction backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtractionCapabilities {
    pub text: bool,
    pub ocr: bool,
    pub layout: bool,
    pub structured: bool,
}

impl ExtractionCapabilities {
    pub const PDFIUM: Self = Self { text: true, ocr: false, layout: false, structured: true };

    pub const VLM: Self = Self { text: true, ocr: true, layout: true, structured: false };

    pub const REMOTE_OCR: Self = Self { text: true, ocr: true, layout: false, structured: false };
}

/// Backend plugin for PDF text extraction.
#[async_trait]
pub trait ExtractionBackend: Send + Sync {
    fn id(&self) -> &str;
    fn capabilities(&self) -> ExtractionCapabilities;
    async fn extract_text(&self, ctx: &ExtractionContext) -> PdfResult<TextExtractionResult>;
}

/// Local Pdfium backend.
pub struct PdfiumBackend;

#[async_trait]
impl ExtractionBackend for PdfiumBackend {
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
            metadata: Some(serde_json::json!({"method": "pdfium"})),
        })
    }
}

/// Routes extraction to backends based on quality probe and priority chain.
pub struct ExtractionRouter {
    backends: Vec<Arc<dyn ExtractionBackend>>,
}

impl ExtractionRouter {
    pub fn new(backends: Vec<Arc<dyn ExtractionBackend>>) -> Self {
        Self { backends }
    }

    /// Default chain: pdfium only (VLM wired separately in pipeline for now).
    pub fn default_chain() -> Self {
        Self::new(vec![Arc::new(PdfiumBackend)])
    }

    pub fn backends(&self) -> &[Arc<dyn ExtractionBackend>] {
        &self.backends
    }

    pub fn backend_ids(&self) -> Vec<&str> {
        self.backends.iter().map(|b| b.id()).collect()
    }

    /// Select backend id for a file before extraction (quality-based).
    pub fn select_backend_id(&self, file_path: &Path) -> PdfResult<(String, ExtractionMethod)> {
        let data = std::fs::read(file_path).map_err(crate::error::PdfModuleError::Io)?;
        let report = QualityProbe::analyze(&data)?;
        let method = report.extraction_method;
        let id = match method {
            ExtractionMethod::Pdfium => "pdfium",
            ExtractionMethod::Vlm | ExtractionMethod::Hybrid => {
                if self.backends.iter().any(|b| b.id() == "vlm") {
                    "vlm"
                } else if self.backends.iter().any(|b| b.capabilities().ocr) {
                    self.backends
                        .iter()
                        .find(|b| b.capabilities().ocr)
                        .map(|b| b.id())
                        .unwrap_or("pdfium")
                } else {
                    "pdfium"
                }
            }
        };
        Ok((id.to_string(), method))
    }

    pub async fn extract_text(
        &self,
        ctx: &ExtractionContext,
        preferred_id: Option<&str>,
    ) -> PdfResult<TextExtractionResult> {
        let method = ctx.quality_report.extraction_method;
        let order = self.resolve_order(method, preferred_id);

        let mut last_err = None;
        for id in order {
            let Some(backend) = self.backends.iter().find(|b| b.id() == id) else {
                continue;
            };
            match backend.extract_text(ctx).await {
                Ok(result) if !result.extracted_text.trim().is_empty() => return Ok(result),
                Ok(_) => {
                    warn!(backend = %id, "empty extraction, trying next backend");
                }
                Err(e) => {
                    warn!(backend = %id, error = %e, "extraction failed, trying next backend");
                    last_err = Some(e);
                }
            }
        }

        if let Some(e) = last_err {
            return Err(e);
        }
        PdfiumBackend.extract_text(ctx).await
    }

    fn resolve_order(&self, method: ExtractionMethod, preferred: Option<&str>) -> Vec<String> {
        let mut order = Vec::new();
        if let Some(p) = preferred {
            order.push(p.to_string());
        }
        match method {
            ExtractionMethod::Pdfium => order.push("pdfium".into()),
            ExtractionMethod::Vlm => {
                order.push("vlm".into());
                order.push("pdfium".into());
            }
            ExtractionMethod::Hybrid => {
                order.push("vlm".into());
                order.push("pdfium".into());
            }
        }
        for b in &self.backends {
            if !order.iter().any(|id| id == b.id()) {
                order.push(b.id().to_string());
            }
        }
        order
    }
}

pub use remote::{RemoteExtractionBackend, RemoteExtractionConfig};
#[cfg(feature = "vlm")]
pub use vlm::VlmExtractionBackend;

#[cfg(test)]
mod tests {
    use super::{ExtractionRouter, PdfiumBackend};
    use std::sync::Arc;

    #[test]
    fn test_default_chain_has_pdfium() {
        let router = ExtractionRouter::default_chain();
        assert!(router.backend_ids().iter().any(|id| *id == "pdfium"));
    }

    #[test]
    fn test_router_new_with_backends() {
        let router = ExtractionRouter::new(vec![Arc::new(PdfiumBackend)]);
        assert_eq!(router.backends().len(), 1);
    }
}
