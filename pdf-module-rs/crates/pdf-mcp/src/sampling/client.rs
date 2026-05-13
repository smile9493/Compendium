//! MCP Sampling client for server-initiated LLM calls.
//!
//! This module implements the client-side of MCP sampling, allowing the server
//! to send `sampling/createMessage` requests to the MCP client and receive responses.

use super::{SamplingRequest, SamplingResponse};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::debug;

#[derive(Debug, Error)]
pub enum SamplingError {
    #[error("Channel closed")]
    ChannelClosed,

    #[error("Response timeout")]
    ResponseTimeout,

    #[error("Internal error: {0}")]
    Internal(String),
}

type PendingRequests =
    Arc<RwLock<HashMap<u64, oneshot::Sender<Result<SamplingResponse, SamplingError>>>>>;

pub struct SamplingClient {
    request_tx: mpsc::Sender<OutgoingRequest>,
    pending: PendingRequests,
    next_id: AtomicU64,
    timeout_secs: u64,
}

pub struct OutgoingRequest {
    pub id: u64,
    pub request: SamplingRequest,
}

impl SamplingClient {
    pub fn with_sender(timeout_secs: u64, sender: mpsc::Sender<OutgoingRequest>) -> Self {
        let pending = Arc::new(RwLock::new(HashMap::new()));

        Self {
            request_tx: sender,
            pending,
            next_id: AtomicU64::new(1),
            timeout_secs,
        }
    }

    pub fn pending_requests(&self) -> PendingRequests {
        Arc::clone(&self.pending)
    }

    pub async fn request_sampling(
        &self,
        request: SamplingRequest,
    ) -> Result<SamplingResponse, SamplingError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (response_tx, response_rx) = oneshot::channel();

        {
            let mut pending = self.pending.write().await;
            pending.insert(id, response_tx);
        }

        let outgoing = OutgoingRequest { id, request };
        if self.request_tx.send(outgoing).await.is_err() {
            let mut pending = self.pending.write().await;
            pending.remove(&id);
            return Err(SamplingError::ChannelClosed);
        }

        let timeout_duration = std::time::Duration::from_secs(self.timeout_secs);
        match tokio::time::timeout(timeout_duration, response_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(SamplingError::ResponseTimeout),
            Err(_) => {
                let mut pending = self.pending.write().await;
                pending.remove(&id);
                Err(SamplingError::ResponseTimeout)
            }
        }
    }

    pub async fn handle_response(
        &self,
        id: u64,
        response: Result<SamplingResponse, SamplingError>,
    ) -> bool {
        let response_tx = {
            let mut pending = self.pending.write().await;
            pending.remove(&id)
        };

        if let Some(tx) = response_tx {
            let _ = tx.send(response);
            true
        } else {
            debug!("No pending request found for response id={}", id);
            false
        }
    }
}

pub fn create_sampling_jsonrpc_request(id: u64, request: SamplingRequest) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "sampling/createMessage",
        "params": request
    })
}

pub fn parse_sampling_response(
    value: &serde_json::Value,
) -> Result<(u64, Result<SamplingResponse, SamplingError>), String> {
    let id = value
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("Missing or invalid id in response")?;

    if let Some(error) = value.get("error") {
        let message = error
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return Ok((id, Err(SamplingError::Internal(message.to_string()))));
    }

    let result = value.get("result").ok_or("Missing result in response")?;

    let response: SamplingResponse = serde_json::from_value(result.clone())
        .map_err(|e| format!("Failed to parse SamplingResponse: {}", e))?;

    Ok((id, Ok(response)))
}

#[derive(Debug, Clone)]
pub struct SamplingClientConfig {
    pub timeout_secs: u64,
    pub max_concurrent: usize,
}

impl Default for SamplingClientConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 60,
            max_concurrent: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sampling::{Role, SamplingContent, SamplingMessage};

    #[test]
    fn test_create_sampling_jsonrpc_request() {
        let request = SamplingRequest {
            messages: vec![SamplingMessage {
                role: Role::User,
                content: SamplingContent::Text {
                    text: "Hello".to_string(),
                },
            }],
            max_tokens: Some(100),
            ..Default::default()
        };

        let json = create_sampling_jsonrpc_request(42, request);
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], 42);
        assert_eq!(json["method"], "sampling/createMessage");
        assert!(json["params"]["messages"].is_array());
    }

    #[test]
    fn test_parse_sampling_response_success() {
        let response_json = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 123,
            "result": {
                "model": "claude-3",
                "role": "assistant",
                "content": {
                    "type": "text",
                    "text": "Hello, how can I help?"
                }
            }
        });

        let (id, result) = parse_sampling_response(&response_json).unwrap();
        assert_eq!(id, 123);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.model, "claude-3");
    }

    #[test]
    fn test_parse_sampling_response_error() {
        let response_json = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 456,
            "error": {
                "code": -32600,
                "message": "Invalid request"
            }
        });

        let (id, result) = parse_sampling_response(&response_json).unwrap();
        assert_eq!(id, 456);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sampling_client_request_response() {
        let (tx, mut rx) = mpsc::channel::<OutgoingRequest>(10);
        let client = SamplingClient::with_sender(5, tx.clone());
        let _pending = client.pending_requests();

        let request = SamplingRequest {
            messages: vec![SamplingMessage {
                role: Role::User,
                content: SamplingContent::Text {
                    text: "Test".to_string(),
                },
            }],
            ..Default::default()
        };

        let client_clone = SamplingClient::with_sender(5, tx);
        let request_handle =
            tokio::spawn(async move { client_clone.request_sampling(request).await });

        if let Some(outgoing) = rx.recv().await {
            let response = SamplingResponse {
                model: "test-model".to_string(),
                role: Role::Assistant,
                content: SamplingContent::Text {
                    text: "Test response".to_string(),
                },
                stop_reason: Some("end".to_string()),
            };
            client.handle_response(outgoing.id, Ok(response)).await;
        }

        drop(request_handle);
    }
}
