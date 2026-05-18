//! MCP Resources protocol — serves embedded SPA via rust_embed.
//!
//! Resources return the full SPA HTML from the embedded dist,
//! enabling MCP clients (Claude Desktop, Cursor) to render
//! the wiki browser and dashboard in their sidebar.

use crate::embed::Assets;
use crate::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};

pub fn handle_resources_list(request: &JsonRpcRequest) -> JsonRpcResponse {
    let resources = serde_json::json!({
        "resources": [
            {
                "uri": "ui://wiki/browser",
                "name": "Wiki Browser",
                "description": "Interactive wiki knowledge browser with tree navigation, full-text search, concept maps, and backlinks. Built with Vue3 SPA.",
                "mimeType": "text/html",
                "annotations": {
                    "audience": ["user"],
                    "priority": 0.8
                }
            },
            {
                "uri": "ui://dashboard/health",
                "name": "Knowledge Health Dashboard",
                "description": "Dashboard showing knowledge base health metrics, domain distribution, index statistics, and server configuration.",
                "mimeType": "text/html",
                "annotations": {
                    "audience": ["user"],
                    "priority": 0.7
                }
            }
        ]
    });
    JsonRpcResponse::success(request.id.clone(), resources)
}

pub fn handle_resources_read(request: &JsonRpcRequest) -> JsonRpcResponse {
    let uri = request.params.get("uri").and_then(|u| u.as_str()).unwrap_or("");

    // Both resources serve the same SPA (routes internally via Vue Router)
    match uri {
        "ui://wiki/browser" | "ui://dashboard/health" => match Assets::get("index.html") {
            Some(content) => {
                let html = String::from_utf8_lossy(&content.data).into_owned();
                let result = serde_json::json!({
                    "contents": [
                        {
                            "uri": uri,
                            "mimeType": "text/html",
                            "text": html
                        }
                    ]
                });
                JsonRpcResponse::success(request.id.clone(), result)
            }
            None => JsonRpcResponse::error(
                request.id.clone(),
                JsonRpcError::internal_error("SPA index.html not found (pdf-web-ui not built?)"),
            ),
        },
        _ => JsonRpcResponse::error(
            request.id.clone(),
            JsonRpcError::invalid_params(&format!("Unknown resource URI: {}", uri)),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::JsonRpcRequest;
    use serde_json::Value;

    fn create_request(method: &str, params: Value) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(serde_json::Number::from(1))),
            method: method.to_string(),
            params,
        }
    }

    #[test]
    fn test_handle_resources_list() {
        let request = create_request("resources/list", serde_json::json!({}));
        let response = handle_resources_list(&request);

        assert!(response.result.is_some());
        let result = response.result.unwrap();
        let resources = result.get("resources").expect("Should have resources");
        let resources_arr = resources.as_array().expect("Resources should be array");

        assert!(resources_arr.len() >= 2);

        let uris: Vec<&str> =
            resources_arr.iter().filter_map(|r| r.get("uri").and_then(|u| u.as_str())).collect();
        assert!(uris.contains(&"ui://wiki/browser"));
        assert!(uris.contains(&"ui://dashboard/health"));
    }

    #[test]
    fn test_handle_resources_read_wiki_browser() {
        let request = create_request(
            "resources/read",
            serde_json::json!({
                "uri": "ui://wiki/browser"
            }),
        );
        let response = handle_resources_read(&request);

        assert!(response.result.is_some());
        let result = response.result.unwrap();
        let contents = result.get("contents").expect("Should have contents");
        let contents_arr = contents.as_array().expect("Contents should be array");

        assert_eq!(contents_arr.len(), 1);
        assert_eq!(contents_arr[0]["uri"], "ui://wiki/browser");
        assert_eq!(contents_arr[0]["mimeType"], "text/html");
    }

    #[test]
    fn test_handle_resources_read_dashboard() {
        let request = create_request(
            "resources/read",
            serde_json::json!({
                "uri": "ui://dashboard/health"
            }),
        );
        let response = handle_resources_read(&request);

        assert!(response.result.is_some());
        let result = response.result.unwrap();
        let contents = result.get("contents").expect("Should have contents");
        let contents_arr = contents.as_array().expect("Contents should be array");

        assert_eq!(contents_arr.len(), 1);
        assert_eq!(contents_arr[0]["uri"], "ui://dashboard/health");
        assert_eq!(contents_arr[0]["mimeType"], "text/html");
    }

    #[test]
    fn test_handle_resources_read_unknown_uri() {
        let request = create_request(
            "resources/read",
            serde_json::json!({
                "uri": "ui://unknown/resource"
            }),
        );
        let response = handle_resources_read(&request);

        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32602);
        assert!(error.message.contains("Unknown resource URI"));
    }

    #[test]
    fn test_handle_resources_read_missing_uri() {
        let request = create_request("resources/read", serde_json::json!({}));
        let response = handle_resources_read(&request);

        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32602);
    }
}
