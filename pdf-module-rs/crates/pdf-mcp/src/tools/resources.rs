//! MCP Resources protocol — serves embedded UI assets (dashboard, wiki browser).
//!
//! Uses `rust_embed` to compile HTML/CSS/JS into the binary at build time,
//! enabling zero-dependency single-binary deployment. Implements
//! `resources/list` and `resources/read` per the MCP specification.

use crate::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src/ui/"]
struct UiAssets;

pub fn handle_resources_list(request: &JsonRpcRequest) -> JsonRpcResponse {
    let resources = serde_json::json!({
        "resources": [
            {
                "uri": "ui://dashboard/health",
                "name": "Knowledge Health Dashboard",
                "description": "Interactive dashboard showing knowledge base health metrics, domain distribution, and index statistics.",
                "mimeType": "text/html;profile=mcp-app"
            },
            {
                "uri": "ui://wiki/browser",
                "name": "Wiki Browser",
                "description": "Interactive wiki knowledge browser with tree navigation, full-text search, concept maps, and backlinks.",
                "mimeType": "text/html;profile=mcp-app"
            }
        ]
    });
    JsonRpcResponse::success(request.id.clone(), resources)
}

pub fn handle_resources_read(request: &JsonRpcRequest) -> JsonRpcResponse {
    let uri = request
        .params
        .get("uri")
        .and_then(|u| u.as_str())
        .unwrap_or("");

    match uri {
        "ui://dashboard/health" => {
            let html = UiAssets::get("dashboard.html")
                .map(|f| String::from_utf8_lossy(&f.data).into_owned())
                .unwrap_or_else(|| "<html><body>Dashboard not available</body></html>".to_string());

            let result = serde_json::json!({
                "contents": [
                    {
                        "uri": uri,
                        "mimeType": "text/html;profile=mcp-app",
                        "text": html
                    }
                ]
            });
            JsonRpcResponse::success(request.id.clone(), result)
        }
        "ui://wiki/browser" => {
            let html = UiAssets::get("wiki.html")
                .map(|f| String::from_utf8_lossy(&f.data).into_owned())
                .unwrap_or_else(|| "<html><body>Wiki browser not available</body></html>".to_string());

            let result = serde_json::json!({
                "contents": [
                    {
                        "uri": uri,
                        "mimeType": "text/html;profile=mcp-app",
                        "text": html
                    }
                ]
            });
            JsonRpcResponse::success(request.id.clone(), result)
        }
        _ => JsonRpcResponse::error(
            request.id.clone(),
            JsonRpcError::invalid_params(&format!("Unknown resource URI: {}", uri)),
        ),
    }
}
