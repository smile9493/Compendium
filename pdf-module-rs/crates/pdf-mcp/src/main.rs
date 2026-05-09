//! # pdf-mcp
//!
//! A MCP (Model Context Protocol) server for PDF extraction.
//!
//! ## Architecture
//!
//! This binary provides a stdio-based MCP server that exposes PDF processing
//! capabilities to AI assistants. It uses the `pdf-core` crate for PDF parsing
//! via the Pdfium engine.
//!
//! ## Features
//!
//! - Extract plain text from PDF files
//! - Extract structured data (per-page text with bounding boxes)
//! - Get page count
//! - Search for keywords with context
//!
//! ## Usage
//!
//! ```bash
//! cargo run --release --bin pdf-mcp
//! ```
//!
//! ## Environment Variables
//!
//! - `VLM_ENDPOINT`: VLM API endpoint URL
//! - `VLM_API_KEY`: VLM API key
//! - `VLM_MODEL`: Target model (default: gpt-4o)
//! - `VLM_TIMEOUT_SECS`: Request timeout in seconds (default: 30)
//! - `VLM_MAX_CONCURRENCY`: Max concurrent VLM requests (default: 5)
//! - `VLM_MAX_RETRIES`: Max retry attempts (default: 3)
//! - `VLM_RETRY_DELAY_BASE_SECS`: Base retry delay (default: 1)
//! - `VLM_RETRY_DELAY_MAX_SECS`: Max retry delay (default: 30)

#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(clippy::all)]
#![deny(clippy::await_holding_lock)]
#![deny(clippy::await_holding_refcell_ref)]
#![deny(clippy::large_stack_frames)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![cfg_attr(test, allow(clippy::undocumented_unsafe_blocks))]
#![deny(clippy::todo)]
#![deny(clippy::dbg_macro)]
#![cfg_attr(not(test), warn(clippy::unwrap_used))]
#![cfg_attr(test, allow(clippy::unwrap_used))]

use pdf_core::{McpPdfPipeline, ServerConfig};
use std::sync::Arc;
use tracing::info;

mod http;
mod protocol;
mod sampling;
mod server;
mod tools;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = ServerConfig::from_env()?;
    config.init_tracing();

    let pipeline = Arc::new(McpPdfPipeline::new(&config)?);

    let http_port = std::env::var("HTTP_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok());

    let kb_path = std::env::var("KNOWLEDGE_BASE").ok().map(std::path::PathBuf::from);

    if let Some(port) = http_port {
        info!("Starting MCP server (stdio + HTTP on port {})", port);

        let http_state = http::HttpState { kb_path };
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();

        let http_handle = tokio::spawn(async move {
            if let Err(e) = http::run_http_server(http_state, port, Some(ready_tx)).await {
                tracing::error!("HTTP server error: {}", e);
            }
        });

        match tokio::time::timeout(std::time::Duration::from_secs(3), ready_rx).await {
            Ok(Ok(())) => info!("HTTP server started successfully on port {}", port),
            _ => tracing::error!("HTTP server failed to start within 3s. Running stdio-only as fallback."),
        }

        let stdio_result = server::run_stdio(pipeline).await;
        http_handle.abort();
        stdio_result
    } else {
        info!("Starting MCP server (stdio only, pdfium engine)");
        server::run_stdio(pipeline).await
    }
}
