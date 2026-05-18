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
// HTTP/OpenAPI modules are wired when `HTTP_PORT` is set; utoipa types are not always referenced in stdio-only builds.
#![allow(dead_code)]
#![recursion_limit = "256"]

use pdf_core::management::WorkspaceRegistry;
use pdf_core::ServerConfig;
use std::sync::Arc;
use tracing::info;
use vlm_visual_gateway::MetricsCollector;

mod api_doc;
mod embed;
mod http;
mod http_schemas;
mod metrics;
mod plugins;
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
    let workspace_registry = Arc::new(WorkspaceRegistry::load_default()?);
    let pipeline = plugins::build_pipeline_with_plugins(&config, Arc::clone(&metrics), None)?;

    let http_port = std::env::var("HTTP_PORT").ok().and_then(|s| s.parse::<u16>().ok());

    let kb_path = workspace_registry.resolve_kb(None, None).ok().or_else(|| {
        std::env::var("KNOWLEDGE_BASE_PATH")
            .or_else(|_| std::env::var("KNOWLEDGE_BASE"))
            .ok()
            .map(std::path::PathBuf::from)
    });

    // Create shared upload store (used by both HTTP and MCP tools)
    let upload_store = Arc::new(upload::UploadStore::new()?);

    let index_cache = Arc::new(pdf_core::knowledge::IndexCache::new());

    // Build ToolContext with upload store reference
    let tool_ctx = tools::ToolContext::new_with_upload_store(
        Arc::clone(&pipeline),
        Some(Arc::clone(&upload_store)),
        Arc::clone(&workspace_registry),
        Arc::clone(&index_cache),
    );

    if let Some(port) = http_port {
        info!("Starting MCP server (stdio + HTTP on port {})", port);

        let http_metrics = Arc::new(metrics::HttpMetrics::new());

        let http_state = http::HttpState {
            kb_path,
            workspace_registry: Arc::clone(&workspace_registry),
            upload_store: Some(Arc::clone(&upload_store)),
            pipeline: Some(Arc::clone(&pipeline)),
            http_metrics: Some(Arc::clone(&http_metrics)),
            index_cache: Arc::clone(&index_cache),
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
