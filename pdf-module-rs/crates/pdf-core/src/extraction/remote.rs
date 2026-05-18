//! HTTP remote extraction plugin backend.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};

use super::{ExtractionBackend, ExtractionCapabilities};
use crate::dto::TextExtractionResult;
use crate::error::{PdfModuleError, PdfResult};
use crate::extractor::ExtractionContext;

/// Remote OCR / extraction service configuration.
#[derive(Debug, Clone)]
pub struct RemoteExtractionConfig {
    pub id: String,
    pub endpoint: String,
    pub timeout: Duration,
    pub priority: i32,
}

#[derive(Serialize)]
struct RemoteExtractRequest {
    pdf_base64: String,
    mode: String,
}

#[derive(Deserialize)]
struct RemoteExtractResponse {
    extracted_text: String,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

/// HTTP JSON backend: `POST {endpoint}` with `{ pdf_base64, mode }`.
pub struct RemoteExtractionBackend {
    config: RemoteExtractionConfig,
    client: reqwest::Client,
}

impl RemoteExtractionBackend {
    pub fn new(config: RemoteExtractionConfig) -> PdfResult<Self> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| PdfModuleError::Extraction(format!("remote client: {e}")))?;
        Ok(Self { config, client })
    }

    pub fn from_configs(
        configs: Vec<RemoteExtractionConfig>,
    ) -> PdfResult<Vec<Arc<dyn ExtractionBackend>>> {
        let mut backends: Vec<Arc<dyn ExtractionBackend>> = Vec::new();
        let mut sorted = configs;
        sorted.sort_by_key(|b| std::cmp::Reverse(b.priority));
        for cfg in sorted {
            backends.push(Arc::new(Self::new(cfg)?));
        }
        Ok(backends)
    }
}

#[async_trait]
impl ExtractionBackend for RemoteExtractionBackend {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn capabilities(&self) -> ExtractionCapabilities {
        ExtractionCapabilities::REMOTE_OCR
    }

    async fn extract_text(&self, ctx: &ExtractionContext) -> PdfResult<TextExtractionResult> {
        let pdf_base64 = B64.encode(ctx.loader.as_bytes());
        let body = RemoteExtractRequest { pdf_base64, mode: "text".to_string() };
        let resp = self
            .client
            .post(&self.config.endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| PdfModuleError::Extraction(format!("remote POST: {e}")))?;

        if !resp.status().is_success() {
            return Err(PdfModuleError::Extraction(format!("remote returned {}", resp.status())));
        }

        let parsed: RemoteExtractResponse = resp
            .json()
            .await
            .map_err(|e| PdfModuleError::Extraction(format!("remote JSON: {e}")))?;

        Ok(TextExtractionResult {
            extracted_text: parsed.extracted_text,
            extraction_metadata: None,
            metadata: parsed.metadata.or_else(|| {
                Some(serde_json::json!({
                    "method": "remote",
                    "backend": self.config.id
                }))
            }),
        })
    }
}
