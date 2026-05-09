use serde::{Deserialize, Serialize};
use std::time::Duration;

// ──────────────────────────────────────────────
//  VLM Model & Config
// ──────────────────────────────────────────────

/// Supported VLM model variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VlmModel {
    /// GPT-4o (OpenAI)
    Gpt4o,
    /// Claude 3.5 Sonnet (Anthropic)
    Claude35Sonnet,
    /// GLM-4.6V - high performance multimodal model
    Glm46v,
    /// GLM-4.6V-FlashX - lightweight high-speed version
    Glm46vFlashX,
    /// GLM-4.6V-Flash - free tier version
    Glm46vFlash,
    /// GLM-OCR - lightweight professional OCR (0.9B parameters, SOTA)
    GlmOcr,
}

/// Page complexity level for smart routing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageComplexity {
    /// Simple page with good text extraction (>500 chars)
    Simple,
    /// Moderate page with some text (50-500 chars)
    Moderate,
    /// Complex page with little/no text (<50 chars, scanned, tables)
    Complex,
}

impl VlmModel {
    /// Model identifier string for API calls
    pub fn model_id(&self) -> &'static str {
        match self {
            VlmModel::Gpt4o => "gpt-4o",
            VlmModel::Claude35Sonnet => "claude-3.5-sonnet",
            VlmModel::Glm46v => "glm-4.6v",
            VlmModel::Glm46vFlashX => "glm-4.6v-flashx",
            VlmModel::Glm46vFlash => "glm-4.6v-flash",
            VlmModel::GlmOcr => "glm-ocr",
        }
    }

    /// Whether this model supports the thinking parameter
    pub fn supports_thinking(&self) -> bool {
        matches!(
            self,
            VlmModel::Glm46v | VlmModel::Glm46vFlashX | VlmModel::Glm46vFlash
        )
    }

    /// Whether this model supports native function calling
    pub fn supports_function_call(&self) -> bool {
        matches!(
            self,
            VlmModel::Glm46v | VlmModel::Glm46vFlashX | VlmModel::Glm46vFlash
        )
    }

    /// Whether this model uses the dedicated layout_parsing endpoint
    pub fn uses_layout_parsing_endpoint(&self) -> bool {
        matches!(self, VlmModel::GlmOcr)
    }

    /// Maximum context window in tokens
    pub fn max_context_tokens(&self) -> usize {
        match self {
            VlmModel::Gpt4o => 128_000,
            VlmModel::Claude35Sonnet => 200_000,
            VlmModel::Glm46v | VlmModel::Glm46vFlashX | VlmModel::Glm46vFlash => 128_000,
            VlmModel::GlmOcr => 8_192,
        }
    }

    /// API path for this model
    pub fn api_path(&self) -> &'static str {
        if self.uses_layout_parsing_endpoint() {
            "/api/paas/v4/layout_parsing"
        } else {
            "/api/paas/v4/chat/completions"
        }
    }

    /// Select the best GLM-4.6V variant based on page complexity
    pub fn select_for_complexity(complexity: PageComplexity) -> Self {
        match complexity {
            PageComplexity::Simple => VlmModel::Glm46vFlash,
            PageComplexity::Moderate => VlmModel::Glm46vFlashX,
            PageComplexity::Complex => VlmModel::Glm46v,
        }
    }
}

/// VLM gateway configuration (immutable after construction)
#[derive(Debug, Clone)]
pub struct VlmConfig {
    /// API endpoint URL
    pub endpoint: String,
    /// API Key (read from env or secure config)
    pub api_key: String,
    /// Target model
    pub model: VlmModel,
    /// Request timeout
    pub timeout: Duration,
    /// Max concurrent requests
    pub max_concurrency: usize,
    /// Maximum number of retry attempts (default: 3)
    pub max_retries: u32,
    /// Base delay between retries (default: 1s)
    pub retry_delay_base: Duration,
    /// Maximum delay cap for backoff (default: 30s)
    pub retry_delay_max: Duration,
    /// Enable thinking mode for GLM-4.6V series (default: true)
    pub enable_thinking: bool,
    /// Enable function calling for GLM-4.6V series (default: false, falls back to text parsing)
    pub enable_function_call: bool,
    /// Enable multi-model smart routing (default: true)
    /// When true, automatically selects the best GLM-4.6V variant based on page complexity
    pub enable_multi_model_routing: bool,
}

impl VlmConfig {
    /// Build from environment variables
    pub fn from_env() -> Result<Self, crate::error::VlmError> {
        let endpoint = std::env::var("VLM_ENDPOINT")
            .map_err(|_| crate::error::VlmError::Config("VLM_ENDPOINT not set".into()))?;
        let api_key = std::env::var("VLM_API_KEY")
            .map_err(|_| crate::error::VlmError::Config("VLM_API_KEY not set".into()))?;

        let model = match std::env::var("VLM_MODEL").as_deref() {
            Ok("claude-3.5-sonnet") => VlmModel::Claude35Sonnet,
            Ok("glm-4.6v") => VlmModel::Glm46v,
            Ok("glm-4.6v-flashx") => VlmModel::Glm46vFlashX,
            Ok("glm-4.6v-flash") => VlmModel::Glm46vFlash,
            Ok("glm-ocr") => VlmModel::GlmOcr,
            _ => VlmModel::Gpt4o,
        };

        let timeout_secs = std::env::var("VLM_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30);

        let max_concurrency = std::env::var("VLM_MAX_CONCURRENCY")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(5);

        let max_retries = std::env::var("VLM_MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(3);

        let retry_delay_base_secs = std::env::var("VLM_RETRY_DELAY_BASE_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(1);

        let retry_delay_max_secs = std::env::var("VLM_RETRY_DELAY_MAX_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30);

        let enable_thinking = std::env::var("VLM_ENABLE_THINKING")
            .ok()
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(true);

        let enable_function_call = std::env::var("VLM_ENABLE_FUNCTION_CALL")
            .ok()
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

        let enable_multi_model_routing = std::env::var("VLM_ENABLE_MULTI_MODEL_ROUTING")
            .ok()
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(true);

        Ok(Self {
            endpoint,
            api_key,
            model,
            timeout: Duration::from_secs(timeout_secs),
            max_concurrency,
            max_retries,
            retry_delay_base: Duration::from_secs(retry_delay_base_secs),
            retry_delay_max: Duration::from_secs(retry_delay_max_secs),
            enable_thinking,
            enable_function_call,
            enable_multi_model_routing,
        })
    }
}

// ──────────────────────────────────────────────
//  Layout Result (VLM response)
// ──────────────────────────────────────────────

/// Semantic region type returned by VLM
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegionType {
    Title,
    Body,
    Table,
    Image,
    Caption,
}

/// Bounding box in page coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// A semantic region identified by VLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub region_type: RegionType,
    pub bbox: BoundingBox,
    pub content: String,
}

/// Complete layout understanding result from VLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutResult {
    /// Semantic regions identified on the page
    pub regions: Vec<Region>,
    /// Reading order (indices into regions)
    pub reading_order: Vec<usize>,
    /// Overall confidence [0.0, 1.0]
    pub confidence: f32,
}

// ──────────────────────────────────────────────
//  VLM Request/Response Payloads (OpenAI Chat Completions format)
// ──────────────────────────────────────────────

/// Thinking configuration for GLM-4.6V series
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ThinkingConfig {
    #[serde(rename = "type")]
    pub thinking_type: String,
}

/// Function definition for native tool calling (GLM-4.6V supports Function Call)
#[derive(Debug, Clone, Serialize)]
pub(crate) struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

/// OpenAI chat completions content part with image URL
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ImageUrlObject {
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum ChatMessageContentPart {
    ImageUrl { image_url: ImageUrlObject },
    Text { text: String },
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChatMessage {
    pub role: String,
    pub content: Vec<ChatMessageContentPart>,
}

/// OpenAI chat completions request with GLM-4.6V extensions
#[derive(Debug, Clone, Serialize)]
pub(crate) struct VlmChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    /// Enable thinking mode (GLM-4.6V only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    /// Tools for function calling (GLM-4.6V native support)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
}

/// Page metadata injected into VLM request
#[derive(Debug, Clone, Serialize)]
pub struct PayloadMetadata {
    pub page_width: f32,
    pub page_height: f32,
    pub page_number: u32,
}

/// VLM request payload (legacy, kept for internal use)
#[derive(Debug, Clone, Serialize)]
pub struct VlmPayload {
    /// Base64-encoded image data
    pub image: String,
    /// Optional hint text (truncated to 10 000 chars)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// Page metadata
    pub metadata: PayloadMetadata,
    /// Target model identifier
    pub model: String,
}

impl VlmPayload {
    /// Convert to OpenAI-compatible chat completions request
    /// Enables thinking mode and function calling for GLM-4.6V series
    pub(crate) fn to_chat_request(
        &self,
        system_prompt: &str,
        enable_thinking: bool,
        enable_function_call: bool,
    ) -> VlmChatRequest {
        let mut content = Vec::new();

        content.push(ChatMessageContentPart::ImageUrl {
            image_url: ImageUrlObject {
                url: format!("data:image/png;base64,{}", self.image),
            },
        });

        let mut text = String::new();
        if let Some(ref hint) = self.hint {
            text.push_str(&format!("Extracted text from the page: {}\n\n", hint));
        }
        text.push_str(system_prompt);
        text.push_str(&format!(
            "\n\nPage {} dimensions: {:.0} x {:.0} points",
            self.metadata.page_number, self.metadata.page_width, self.metadata.page_height
        ));

        content.push(ChatMessageContentPart::Text { text });

        let tools = if enable_function_call {
            Some(vec![ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "extract_layout_regions".to_string(),
                    description: "Extract layout regions from the PDF page image and return structured layout analysis".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "regions": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "region_type": {
                                            "type": "string",
                                            "enum": ["title", "body", "table", "image", "caption"]
                                        },
                                        "bbox": {
                                            "type": "object",
                                            "properties": {
                                                "x": {"type": "number"},
                                                "y": {"type": "number"},
                                                "width": {"type": "number"},
                                                "height": {"type": "number"}
                                            },
                                            "required": ["x", "y", "width", "height"]
                                        },
                                        "content": {"type": "string"}
                                    },
                                    "required": ["region_type", "bbox", "content"]
                                }
                            },
                            "reading_order": {
                                "type": "array",
                                "items": {"type": "integer"}
                            },
                            "confidence": {"type": "number", "minimum": 0.0, "maximum": 1.0}
                        },
                        "required": ["regions"]
                    }),
                },
            }])
        } else {
            None
        };

        VlmChatRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content,
            }],
            thinking: if enable_thinking {
                Some(ThinkingConfig {
                    thinking_type: "enabled".to_string(),
                })
            } else {
                None
            },
            tools,
        }
    }
}

/// OpenAI chat completions response
#[derive(Debug, Deserialize)]
pub(crate) struct ChatCompletionResponse {
    pub choices: Vec<ChatCompletionChoice>,
    /// Usage statistics (for monitoring and cost tracking)
    #[serde(default)]
    #[allow(dead_code)]
    pub usage: Option<UsageStats>,
}

/// GLM-OCR layout parsing request
#[derive(Debug, Clone, Serialize)]
pub(crate) struct GlmOcrRequest {
    pub model: String,
    pub file: String,
}

/// GLM-OCR layout parsing response
#[derive(Debug, Deserialize)]
pub(crate) struct GlmOcrResponse {
    /// Parsed text content from the document (aggregated text)
    pub text: Option<String>,
    /// Document metadata
    #[allow(dead_code)]
    pub data_info: Option<GlmOcrDataInfo>,
    /// Detailed layout regions
    pub layout_details: Vec<Vec<GlmOcrLayoutItem>>,
}

/// GLM-OCR document metadata
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct GlmOcrDataInfo {
    pub num_pages: usize,
    pub pages: Vec<GlmOcrPageInfo>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct GlmOcrPageInfo {
    pub width: usize,
    pub height: usize,
}

/// GLM-OCR layout item (one region per page)
#[derive(Debug, Deserialize)]
pub(crate) struct GlmOcrLayoutItem {
    /// Region label: text, image, title, table, caption, header, footer, etc.
    pub label: String,
    /// Bounding box coordinates [xmin, ymin, xmax, ymax]
    pub bbox_2d: Option<[i64; 4]>,
    /// Recognized text content (only for text-type regions)
    pub content: Option<String>,
    /// Page dimensions (for reference, not currently used)
    #[allow(dead_code)]
    pub width: Option<i64>,
    #[allow(dead_code)]
    pub height: Option<i64>,
}

/// Token usage statistics
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct UsageStats {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatCompletionChoice {
    pub message: ChatCompletionMessage,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatCompletionMessage {
    pub content: Option<String>,
}

// ──────────────────────────────────────────────
//  Detector config
// ──────────────────────────────────────────────

/// Configuration for the escalation detector
#[derive(Debug, Clone)]
pub struct DetectorConfig {
    /// Character count threshold below which VLM OCR is triggered (default 0)
    pub zero_text_threshold: u32,
    /// Layout confidence below which VLM layout understanding is triggered (default 0.3)
    pub layout_confidence_threshold: f32,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            zero_text_threshold: 0,
            layout_confidence_threshold: 0.3,
        }
    }
}

// ──────────────────────────────────────────────
//  Pdfium Extraction (input to detector)
// ──────────────────────────────────────────────

/// Pdfium extraction summary used by the escalation detector
#[derive(Debug, Clone)]
pub struct PdfiumExtraction {
    pub character_count: u32,
    pub layout_confidence: f32,
    pub text: String,
    pub page_width: f32,
    pub page_height: f32,
}

// ──────────────────────────────────────────────
//  Degradation
// ──────────────────────────────────────────────

/// Degradation reason code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DegradationReason {
    Timeout,
    Unavailable,
    ParseError,
    RateLimit,
}

/// Degradation record attached to a result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationRecord {
    pub reason: DegradationReason,
    pub trace_id: String,
    pub message: String,
}
