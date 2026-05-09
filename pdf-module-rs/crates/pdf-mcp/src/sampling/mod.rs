//! MCP Sampling protocol types.
//!
//! Implements the MCP sampling protocol for Server-initiated LLM calls,
//! enabling the PDF MCP server to request LLM assistance for complex
//! document analysis tasks.

pub mod client;
pub mod manager;

#[allow(unused_imports)]
pub use client::{
    create_sampling_jsonrpc_request, parse_sampling_response, OutgoingRequest, SamplingClient,
    SamplingClientConfig,
};
#[allow(unused_imports)]
pub use manager::{SamplingError, SamplingManager};

use serde::{Deserialize, Serialize};

/// A sampling request sent from the MCP server to the client.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SamplingRequest {
    pub messages: Vec<SamplingMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<ModelPreferences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_context: Option<IncludeContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

/// A single message in a sampling request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingMessage {
    pub role: Role,
    pub content: SamplingContent,
}

/// Message role (user or assistant).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// Content types for sampling messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SamplingContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
}

/// Hints about which model to prefer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelPreferences {
    #[serde(default)]
    pub hints: Vec<ModelHint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_priority: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_priority: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence_priority: Option<f32>,
}

/// A single model hint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHint {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Context to include in the sampling request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IncludeContext {
    None,
    ThisServer,
    AllServers,
}

/// A response from the LLM to a sampling request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingResponse {
    pub model: String,
    pub role: Role,
    pub content: SamplingContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sampling_request_default() {
        let req = SamplingRequest::default();
        assert!(req.messages.is_empty());
        assert!(req.model_preferences.is_none());
    }

    #[test]
    fn test_sampling_request_serialization() {
        let req = SamplingRequest {
            messages: vec![SamplingMessage {
                role: Role::User,
                content: SamplingContent::Text {
                    text: "Analyze this page".to_string(),
                },
            }],
            max_tokens: Some(1000),
            ..Default::default()
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"text\""));
        assert!(json.contains("\"user\""));
        assert!(json.contains("1000"));
    }

    #[test]
    fn test_model_preferences_serialization() {
        let prefs = ModelPreferences {
            hints: vec![ModelHint {
                name: "claude-3".to_string(),
                version: None,
            }],
            speed_priority: Some(0.8),
            ..Default::default()
        };
        let json = serde_json::to_string(&prefs).unwrap();
        assert!(json.contains("claude-3"));
        assert!(json.contains("0.8"));
    }
}
