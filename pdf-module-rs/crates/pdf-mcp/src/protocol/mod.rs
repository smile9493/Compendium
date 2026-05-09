//! Shared JSON-RPC types for the pdf-mcp crate.
//!
//! Single source of truth for protocol types used by both the stdio server
//! and any future plugin-based MCP server.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Default JSON-RPC version string
fn default_jsonrpc() -> String {
    "2.0".to_string()
}

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    #[serde(default = "default_jsonrpc")]
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    #[allow(dead_code)]
    pub fn parse_error() -> Self {
        Self {
            code: -32700,
            message: "Parse error".to_string(),
            data: None,
        }
    }

    #[allow(dead_code)]
    pub fn invalid_request() -> Self {
        Self {
            code: -32600,
            message: "Invalid request".to_string(),
            data: None,
        }
    }

    #[allow(dead_code)]
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method),
            data: None,
        }
    }

    #[allow(dead_code)]
    pub fn invalid_params(message: &str) -> Self {
        Self {
            code: -32602,
            message: format!("Invalid params: {}", message),
            data: None,
        }
    }

    #[allow(dead_code)]
    pub fn internal_error(message: &str) -> Self {
        Self {
            code: -32603,
            message: message.to_string(),
            data: None,
        }
    }
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// Tool definition for tools/list responses
#[derive(Debug, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Text content block for tool call results
#[derive(Debug, Serialize)]
pub struct Content {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

impl Content {
    pub fn text(text: String) -> Self {
        Self {
            content_type: "text".to_string(),
            text,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_error_codes() {
        let parse_err = JsonRpcError::parse_error();
        assert_eq!(parse_err.code, -32700);

        let method_err = JsonRpcError::method_not_found("test");
        assert_eq!(method_err.code, -32601);

        let params_err = JsonRpcError::invalid_params("bad");
        assert_eq!(params_err.code, -32602);

        let internal_err = JsonRpcError::internal_error("fail");
        assert_eq!(internal_err.code, -32603);
    }

    #[test]
    fn test_json_rpc_response_success() {
        let response = JsonRpcResponse::success(
            Some(Value::Number(serde_json::Number::from(1))),
            serde_json::json!({"result": "ok"}),
        );
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_json_rpc_response_error() {
        let response = JsonRpcResponse::error(
            Some(Value::Number(serde_json::Number::from(1))),
            JsonRpcError::method_not_found("test"),
        );
        assert!(response.result.is_none());
        assert!(response.error.is_some());
    }
}
