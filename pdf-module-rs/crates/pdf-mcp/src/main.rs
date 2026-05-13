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
#![recursion_limit = "256"]

use pdf_core::{McpPdfPipeline, ServerConfig};
use std::sync::Arc;
use tracing::info;
use vlm_visual_gateway::MetricsCollector;

mod embed;
mod http;
mod protocol;
mod sampling;
mod server;
mod tools;
mod upload;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = ServerConfig::from_env()?;
    config.init_tracing();

    // Facade owns the shared MetricsCollector so metrics from all components
    // (VLM gateway, pipeline, tools) are collected into a single registry.
    let metrics = Arc::new(MetricsCollector::with_default_registry());
    let pipeline = Arc::new(McpPdfPipeline::new_with_metrics(&config, metrics)?);

    let http_port = std::env::var("HTTP_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok());

    let kb_path = std::env::var("KNOWLEDGE_BASE")
        .ok()
        .map(std::path::PathBuf::from);

    // Create shared upload store (used by both HTTP and MCP tools)
    let upload_store = Arc::new(upload::UploadStore::new()?);

    // Build ToolContext with upload store reference
    let tool_ctx = tools::ToolContext::new_with_upload_store(
        Arc::clone(&pipeline),
        Some(Arc::clone(&upload_store)),
    );

    if let Some(port) = http_port {
        info!("Starting MCP server (stdio + HTTP on port {})", port);

        let http_state = http::HttpState {
            kb_path,
            upload_store: Some(Arc::clone(&upload_store)),
        };
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();

        let http_handle = tokio::spawn(async move {
            if let Err(e) = http::run_http_server(http_state, port, Some(ready_tx)).await {
                tracing::error!("HTTP server error: {}", e);
            }
        });

        match tokio::time::timeout(std::time::Duration::from_secs(3), ready_rx).await {
            Ok(Ok(())) => info!("HTTP server started successfully on port {}", port),
            _ => tracing::error!(
                "HTTP server failed to start within 3s. Running stdio-only as fallback."
            ),
        }

        let stdio_result = server::run_stdio_with_tool_ctx(pipeline, tool_ctx, upload_store).await;
        http_handle.abort();
        stdio_result
    } else {
        info!("Starting MCP server (stdio only, pdfium engine)");
        server::run_stdio_with_tool_ctx(pipeline, tool_ctx, upload_store).await
    }
}
