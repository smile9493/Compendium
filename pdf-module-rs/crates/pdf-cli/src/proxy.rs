//! # Stdio Proxy Mode
//!
//! Listens on stdio for MCP JSON-RPC messages and forwards them to a remote
//! HTTP MCP server. Enables legacy MCP clients that only speak stdio to
//! transparently connect to a remote server.
//!
//! Usage: `compendium proxy --server http://192.168.2.50:9090`

use crate::config::CliConfig;
use crate::remote::{RemoteClient, RemoteConfig};
use anyhow::Result;
use serde_json::Value;
use std::io::{BufRead, Write};
use std::sync::Arc;

/// Run stdio proxy, forwarding all JSON-RPC lines to the remote HTTP server.
pub async fn run_proxy(config: &CliConfig) -> Result<()> {
    let server = config.server.as_deref().ok_or_else(|| {
        anyhow::anyhow!(
            "No remote server configured. Use --server flag or 'config set server <URL>'."
        )
    })?;

    let token = config.token.clone().or_else(|| std::env::var("RSUT_PDF_TOKEN").ok());

    let remote_cfg = RemoteConfig { server: server.to_string(), token, timeout_secs: 600 };

    let client = RemoteClient::new(remote_cfg)?;
    let client = Arc::new(client);

    eprintln!("Proxy mode: forwarding stdio ↔ {}", server);
    eprintln!("Waiting for MCP JSON-RPC messages on stdin...");

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        // Parse JSON-RPC request
        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let error = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {}", e)
                    }
                });
                let _ = writeln!(stdout_lock, "{}", json_to_string(&error));
                let _ = stdout_lock.flush();
                continue;
            }
        };

        // Extract method and params
        let method = request["method"].as_str().unwrap_or("");
        let id = request.get("id").cloned();

        // Handle initialize locally
        if method == "initialize" {
            let response = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": "rust-pdf-mcp (proxy)",
                        "version": "1.0.0"
                    },
                    "capabilities": {
                        "tools": { "listChanged": false },
                        "resources": { "listChanged": false }
                    },
                    "instructions": "Connected via stdio proxy. All tools available."
                }
            });
            let _ = writeln!(stdout_lock, "{}", json_to_string(&response));
            let _ = stdout_lock.flush();
            continue;
        }

        // Handle tools/list locally
        if method == "tools/list" {
            let response = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [
                        {"name": "list_tools", "description": "List available tools", "inputSchema": {"type": "object", "properties": {}}}
                    ]
                }
            });
            let _ = writeln!(stdout_lock, "{}", json_to_string(&response));
            let _ = stdout_lock.flush();
            continue;
        }

        // Forward tools/call to remote server
        if method == "tools/call" {
            let tool_name = request["params"]["name"].as_str().unwrap_or("");
            let arguments = request["params"]["arguments"].clone();
            let arguments = if arguments.is_null() { serde_json::json!({}) } else { arguments };

            match client.call_tool(tool_name, arguments).await {
                Ok(result) => {
                    let response = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string(&result).unwrap_or_default()
                            }]
                        }
                    });
                    let _ = writeln!(stdout_lock, "{}", json_to_string(&response));
                }
                Err(e) => {
                    let response = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32603,
                            "message": format!("Proxy error: {}", e)
                        }
                    });
                    let _ = writeln!(stdout_lock, "{}", json_to_string(&response));
                }
            }
            let _ = stdout_lock.flush();
            continue;
        }

        // Pass through notifications (no response expected)
        if method.starts_with("notifications/") {
            continue;
        }

        // Unknown method
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": format!("Method not found in proxy: {}", method)
            }
        });
        let _ = writeln!(stdout_lock, "{}", json_to_string(&response));
        let _ = stdout_lock.flush();
    }

    eprintln!("Proxy: stdin closed, shutting down.");
    Ok(())
}

/// Serialize a JSON value to string, panicking only on unrecoverable serialization failure.
fn json_to_string(value: &Value) -> String {
    serde_json::to_string(value).expect("JSON serialization should not fail for proxy responses")
}
