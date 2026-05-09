use std::sync::Arc;
use std::time::Duration;

use regex::Regex;
use reqwest::Client;
use std::sync::LazyLock;
use tokio::sync::{broadcast, Semaphore};
use tokio::time::timeout;
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::{VlmError, VlmResult};
use crate::metrics::MetricsCollector;
use crate::types::{
    ChatCompletionResponse, GlmOcrResponse, LayoutResult, PayloadMetadata, Region, RegionType,
    VlmConfig, VlmPayload,
};

static COORD_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[\s*(\d+(?:\.\d+)?)\s*,\s*(\d+(?:\.\d+)?)\s*,\s*(\d+(?:\.\d+)?)\s*,\s*(\d+(?:\.\d+)?)\s*\]")
        .expect("Invalid coordinate regex pattern")
});

static REGION_TYPE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(title|body|table|image|caption|paragraph|heading)")
        .expect("Invalid region type regex pattern")
});

/// VLM Gateway — the core component that dispatches visual perception
/// requests to a remote VLM API and returns layout understanding results.
///
/// **Design**: stateless, clone-friendly (all fields behind `Arc`), non-blocking.
pub struct VlmGateway {
    client: Client,
    config: Arc<VlmConfig>,
    metrics: Arc<MetricsCollector>,
    semaphore: Arc<Semaphore>,
    shutdown_tx: broadcast::Sender<()>,
}

impl Clone for VlmGateway {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            config: Arc::clone(&self.config),
            metrics: Arc::clone(&self.metrics),
            semaphore: Arc::clone(&self.semaphore),
            shutdown_tx: self.shutdown_tx.clone(),
        }
    }
}

impl VlmGateway {
    /// Create a new gateway from config + shared metrics collector.
    pub fn new(config: VlmConfig, metrics: Arc<MetricsCollector>) -> VlmResult<Self> {
        let client = Client::builder()
            .timeout(config.timeout + Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| VlmError::Config(format!("reqwest client build: {e}")))?;

        let semaphore = Arc::new(Semaphore::new(config.max_concurrency));
        let (shutdown_tx, _) = broadcast::channel(1);

        Ok(Self {
            client,
            config: Arc::new(config),
            metrics,
            semaphore,
            shutdown_tx,
        })
    }

    /// Create from environment variables.
    pub fn from_env(metrics: Arc<MetricsCollector>) -> VlmResult<Self> {
        let config = VlmConfig::from_env()?;
        Self::new(config, metrics)
    }

    // ─── Core public method ───────────────────────────

    /// Perceive layout understanding for a single page.
    ///
    /// `image_data` — raw RGBA pixel bytes (will be Base64-encoded internally).
    /// `hint_text`  — optional Pdfium-extracted text fragments.
    /// `metadata`   — page dimensions and page number.
    #[tracing::instrument(skip(self, image_data), fields(page = metadata.page_number))]
    pub async fn perceive_layout(
        &self,
        image_data: &[u8],
        hint_text: Option<&str>,
        metadata: &PayloadMetadata,
    ) -> VlmResult<LayoutResult> {
        let trace_id = Uuid::new_v4().to_string();
        let provider = self.config.model.model_id();

        // 1. Validate input
        if image_data.is_empty() {
            return Err(VlmError::InvalidImage("image_data is empty".into()));
        }

        // 2. Encode raw RGBA pixels to Base64
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, image_data);

        // 3. Truncate hint
        let hint = hint_text.map(|h| {
            if h.len() > 10_000 {
                warn!(trace_id = %trace_id, "hint_text truncated from {} to 10000 chars", h.len());
                h[..10_000].to_string()
            } else {
                h.to_string()
            }
        });

        // 4. Build payload
        let payload = VlmPayload {
            image: b64,
            hint,
            metadata: PayloadMetadata {
                page_width: metadata.page_width,
                page_height: metadata.page_height,
                page_number: metadata.page_number,
            },
            model: provider.to_string(),
        };

        // 5. Check shutdown signal
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        tokio::select! {
            _ = shutdown_rx.recv() => {
                warn!(trace_id = %trace_id, "VLM request cancelled: shutdown signal");
                Err(VlmError::Unavailable("shutdown".into()))
            }
            result = self.dispatch_request(&payload, &trace_id) => {
                result
            }
        }
    }

    /// Non-blocking fire-and-forget dispatch.
    ///
    /// The main pipeline returns immediately with the local Pdfium result.
    /// The VLM result is sent through `result_tx` when ready.
    pub fn spawn_perceive_layout(
        &self,
        image_data: Vec<u8>,
        hint_text: Option<String>,
        metadata: PayloadMetadata,
        result_tx: tokio::sync::oneshot::Sender<VlmResult<LayoutResult>>,
    ) {
        let gateway = self.clone();

        tokio::spawn(async move {
            let result = gateway
                .perceive_layout(&image_data, hint_text.as_deref(), &metadata)
                .await;
            let _ = result_tx.send(result);
        });
    }

    /// Health check — lightweight GET to the endpoint.
    pub async fn health_check(&self) -> bool {
        self.client
            .get(&self.config.endpoint)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map(|r| r.status().is_success() || r.status().as_u16() == 404)
            .unwrap_or(false)
    }

    /// Create a handle for graceful shutdown.
    pub fn handle(&self) -> VlmGatewayHandle {
        VlmGatewayHandle {
            shutdown_tx: self.shutdown_tx.clone(),
        }
    }

    /// Access the gateway configuration for multi-model routing.
    pub fn config(&self) -> &VlmConfig {
        &self.config
    }

    // ─── Private helpers ──────────────────────────────

    #[tracing::instrument(skip(self, payload), fields(trace_id = %trace_id))]
    async fn dispatch_request(
        &self,
        payload: &VlmPayload,
        trace_id: &str,
    ) -> VlmResult<LayoutResult> {
        // Rate-limit via semaphore
        let permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| VlmError::Unavailable("semaphore closed".into()))?;

        let timer = self.metrics.start_request_timer();
        let provider = self.config.model.model_id().to_string();

        let max_retries = self.config.max_retries;
        let mut attempt = 0;
        let mut last_error: Option<VlmError> = None;

        while attempt <= max_retries {
            if attempt > 0 {
                let delay = self.calculate_backoff(attempt);
                info!(
                    trace_id = %trace_id,
                    attempt = attempt,
                    delay_ms = delay.as_millis(),
                    "Retrying VLM request"
                );
                tokio::time::sleep(delay).await;
            }

            let result = timeout(self.config.timeout, self.send_request(payload)).await;

            match result {
                Ok(Ok(response)) => {
                    let layout = match self.parse_response(&response) {
                        Ok(layout) => layout,
                        Err(e) => {
                            warn!(trace_id = %trace_id, error = %e, attempt = attempt, "Failed to parse VLM response");
                            last_error = Some(e);
                            attempt += 1;
                            continue;
                        }
                    };
                    drop(permit);
                    timer.observe_success(&provider);
                    info!(trace_id = %trace_id, "VLM perceive_layout succeeded");
                    return Ok(layout);
                }
                Ok(Err(e)) => {
                    let is_retryable = Self::is_retryable_error(&e);
                    let reason = match &e {
                        VlmError::RateLimit => "rate_limit",
                        VlmError::Unavailable(_) => "unavailable",
                        _ => "network_error",
                    };

                    warn!(trace_id = %trace_id, error = %e, attempt = attempt, retryable = is_retryable, "VLM request failed");

                    if !is_retryable || attempt >= max_retries {
                        drop(permit);
                        timer.observe_error(&provider);
                        self.metrics.record_degradation(reason);
                        warn!(trace_id = %trace_id, error = %e, "VLM request failed - degrading");
                        return Err(e);
                    }

                    last_error = Some(e);
                    attempt += 1;
                }
                Err(_) => {
                    let is_retryable = attempt < max_retries;
                    warn!(trace_id = %trace_id, attempt = attempt, retryable = is_retryable, "VLM request timed out");

                    if !is_retryable {
                        drop(permit);
                        timer.observe_timeout(&provider);
                        self.metrics.record_degradation("timeout");
                        warn!(trace_id = %trace_id, "VLM request timed out - degrading");
                        return Err(VlmError::Timeout(self.config.timeout.as_secs()));
                    }

                    last_error = Some(VlmError::Timeout(self.config.timeout.as_secs()));
                    attempt += 1;
                }
            }
        }

        drop(permit);
        timer.observe_error(&provider);
        self.metrics.record_degradation("max_retries_exceeded");
        warn!(trace_id = %trace_id, "All retry attempts exhausted - degrading");
        Err(last_error.unwrap_or_else(|| VlmError::Unavailable("max retries exceeded".into())))
    }

    fn is_retryable_error(error: &VlmError) -> bool {
        match error {
            VlmError::RateLimit
            | VlmError::Unavailable(_)
            | VlmError::Network(_)
            | VlmError::Timeout(_) => true,
            VlmError::ParseError(_) | VlmError::InvalidImage(_) | VlmError::Config(_) => false,
        }
    }

    fn calculate_backoff(&self, attempt: u32) -> Duration {
        let base_delay = self.config.retry_delay_base.as_millis() as u64;
        let delay_ms = base_delay * 2u64.pow(attempt);
        let max_delay = self.config.retry_delay_max.as_millis() as u64;
        let delay = delay_ms.min(max_delay);
        let jitter = delay * 10 / 100;
        Duration::from_millis(delay + jitter)
    }

    #[tracing::instrument(skip(self, payload))]
    async fn send_request(&self, payload: &VlmPayload) -> VlmResult<String> {
        let is_ocr = self.config.model.uses_layout_parsing_endpoint();

        if is_ocr {
            self.send_ocr_request(payload).await
        } else {
            self.send_chat_request(payload).await
        }
    }

    async fn send_chat_request(&self, payload: &VlmPayload) -> VlmResult<String> {
        let enable_thinking = self.config.enable_thinking && self.config.model.supports_thinking();
        let enable_function_call =
            self.config.enable_function_call && self.config.model.supports_function_call();

        let chat_request = payload.to_chat_request(
            "Analyze this PDF page and identify all layout regions. \
             For each region, provide: type (title/body/table/image/caption), \
             coordinates as [xmin,ymin,xmax,ymax], and a brief content description. \
             Format each region on a separate line.",
            enable_thinking,
            enable_function_call,
        );

        let resp = self
            .client
            .post(&self.config.endpoint)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&chat_request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    VlmError::Timeout(self.config.timeout.as_secs())
                } else if e.is_connect() {
                    VlmError::Unavailable(e.to_string())
                } else {
                    VlmError::Network(e.to_string())
                }
            })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return if status.as_u16() == 429 {
                Err(VlmError::RateLimit)
            } else if status.as_u16() >= 500 {
                Err(VlmError::Unavailable(format!("HTTP {status}: {body}")))
            } else {
                Err(VlmError::Network(format!("HTTP {status}: {body}")))
            };
        }

        resp.text()
            .await
            .map_err(|e| VlmError::Network(e.to_string()))
    }

    async fn send_ocr_request(&self, payload: &VlmPayload) -> VlmResult<String> {
        let ocr_request = crate::types::GlmOcrRequest {
            model: self.config.model.model_id().to_string(),
            file: format!("data:application/pdf;base64,{}", payload.image),
        };

        let endpoint_base = self
            .config
            .endpoint
            .trim_end_matches("/api/paas/v4/chat/completions")
            .trim_end_matches("/api/paas/v4/layout_parsing");
        let ocr_endpoint = format!("{}{}", endpoint_base, self.config.model.api_path());

        let resp = self
            .client
            .post(&ocr_endpoint)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&ocr_request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    VlmError::Timeout(self.config.timeout.as_secs())
                } else if e.is_connect() {
                    VlmError::Unavailable(e.to_string())
                } else {
                    VlmError::Network(e.to_string())
                }
            })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return if status.as_u16() == 429 {
                Err(VlmError::RateLimit)
            } else if status.as_u16() >= 500 {
                Err(VlmError::Unavailable(format!("HTTP {status}: {body}")))
            } else {
                Err(VlmError::Network(format!("HTTP {status}: {body}")))
            };
        }

        resp.text()
            .await
            .map_err(|e| VlmError::Network(e.to_string()))
    }

    fn parse_response(&self, body: &str) -> VlmResult<LayoutResult> {
        if self.config.model.uses_layout_parsing_endpoint() {
            self.parse_ocr_response(body)
        } else {
            self.parse_chat_response(body)
        }
    }

    fn parse_chat_response(&self, body: &str) -> VlmResult<LayoutResult> {
        let chat_response: ChatCompletionResponse =
            serde_json::from_str(body).map_err(|e| VlmError::ParseError(e.to_string()))?;

        let message_content = chat_response
            .choices
            .first()
            .and_then(|c| c.message.content.as_ref())
            .ok_or_else(|| VlmError::ParseError("empty response from VLM".into()))?;

        let regions = self.parse_layout_from_text(message_content);

        Ok(LayoutResult {
            regions,
            reading_order: Vec::new(),
            confidence: 1.0,
        })
    }

    fn parse_ocr_response(&self, body: &str) -> VlmResult<LayoutResult> {
        let ocr_response: GlmOcrResponse =
            serde_json::from_str(body).map_err(|e| VlmError::ParseError(e.to_string()))?;

        let mut regions = Vec::new();
        let mut total_text = String::new();

        for page_items in &ocr_response.layout_details {
            for item in page_items {
                if item.label == "text"
                    || item.label == "title"
                    || item.label == "caption"
                    || item.label == "table"
                    || item.label == "list"
                {
                    if let Some(ref content) = item.content {
                        total_text.push_str(content);
                        total_text.push('\n');

                        if let Some(bbox) = &item.bbox_2d {
                            let region_type = match item.label.as_str() {
                                "title" => RegionType::Title,
                                "table" => RegionType::Table,
                                "caption" => RegionType::Caption,
                                "image" => RegionType::Image,
                                _ => RegionType::Body,
                            };

                            regions.push(Region {
                                region_type,
                                bbox: crate::types::BoundingBox {
                                    x: bbox[0] as f32,
                                    y: bbox[1] as f32,
                                    width: (bbox[2] - bbox[0]) as f32,
                                    height: (bbox[3] - bbox[1]) as f32,
                                },
                                content: content.clone(),
                            });
                        }
                    }
                } else if item.label == "image" {
                    regions.push(Region {
                        region_type: RegionType::Image,
                        bbox: item.bbox_2d.map_or_else(
                            || crate::types::BoundingBox {
                                x: 0.0,
                                y: 0.0,
                                width: 0.0,
                                height: 0.0,
                            },
                            |bbox| crate::types::BoundingBox {
                                x: bbox[0] as f32,
                                y: bbox[1] as f32,
                                width: (bbox[2] - bbox[0]) as f32,
                                height: (bbox[3] - bbox[1]) as f32,
                            },
                        ),
                        content: "[image]".to_string(),
                    });
                }
            }
        }

        let _text = ocr_response.text.unwrap_or(total_text);
        let confidence = if regions.is_empty() { 0.0 } else { 1.0 };

        Ok(LayoutResult {
            regions,
            reading_order: Vec::new(),
            confidence,
        })
    }

    fn parse_layout_from_text(&self, text: &str) -> Vec<Region> {
        let mut regions = Vec::new();

        for cap in COORD_REGEX.captures_iter(text) {
            let x = cap[1].parse::<f32>().unwrap_or(0.0);
            let y = cap[2].parse::<f32>().unwrap_or(0.0);
            let w = cap[3].parse::<f32>().unwrap_or(0.0);
            let h = cap[4].parse::<f32>().unwrap_or(0.0);

            let context_start = (cap.get(0).expect("capture group 0 always exists").start() as i64
                - 50)
                .max(0) as usize;
            let context_end =
                (cap.get(0).expect("capture group 0 always exists").end() + 50).min(text.len());
            let context = &text[context_start..context_end];

            let region_type = REGION_TYPE_REGEX
                .captures(context)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_lowercase())
                .map(|s| match s.as_str() {
                    "title" => RegionType::Title,
                    "body" => RegionType::Body,
                    "table" => RegionType::Table,
                    "image" => RegionType::Image,
                    "caption" => RegionType::Caption,
                    _ => RegionType::Body,
                })
                .unwrap_or(RegionType::Body);

            regions.push(Region {
                region_type,
                bbox: crate::types::BoundingBox {
                    x,
                    y,
                    width: w - x,
                    height: h - y,
                },
                content: cap[0].to_string(),
            });
        }

        regions
    }
}

/// Handle for graceful shutdown of the VLM gateway.
///
/// Dropping this handle or calling `shutdown()` sends a shutdown signal
/// to all in-flight `perceive_layout` calls.
pub struct VlmGatewayHandle {
    shutdown_tx: broadcast::Sender<()>,
}

impl VlmGatewayHandle {
    /// Send shutdown signal. In-flight requests will observe the signal
    /// at their next `tokio::select!` checkpoint and degrade gracefully.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

impl Drop for VlmGatewayHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Render a PDF page via Pdfium and return raw RGBA pixels + dimensions.
///
/// The caller owns the pixel data and can encode to Base64 as needed.
/// No temporary files are produced — pdfium renders directly into memory.
pub fn render_page_pixels(
    pdf_data: &[u8],
    page_index: u32,
    dpi: f32,
) -> VlmResult<(Vec<u8>, u32, u32)> {
    let pdf_data = pdf_data.to_vec();
    crate::pdfium_guard::catch_pdfium(move || {
        use pdfium_render::prelude::*;
        let pdfium = Pdfium::default();
        let document = pdfium
            .load_pdf_from_byte_slice(&pdf_data, None)
            .map_err(|e| format!("load PDF: {e}"))?;

        let pages = document.pages();
        let idx: u16 = page_index
            .try_into()
            .map_err(|_| format!("page_index {page_index} exceeds u16"))?;
        let page = pages
            .get(idx)
            .map_err(|e| format!("get page {page_index}: {e}"))?;

        // PDF points are 1/72 inch, so scale = dpi / 72.0
        let scale = dpi / 72.0;
        let config = PdfRenderConfig::new().scale_page_by_factor(scale);

        let bitmap = page
            .render_with_config(&config)
            .map_err(|e| format!("render page: {e}"))?;

        let width = bitmap.width() as u32;
        let height = bitmap.height() as u32;
        let rgba = bitmap.as_raw_bytes();

        Ok::<_, String>((rgba, width, height))
    })
    .map_err(|e| VlmError::InvalidImage(format!("Pdfium guard: {e}")))?
    .map_err(VlmError::InvalidImage)
}
