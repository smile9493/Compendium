//! MCP JSON-RPC 2.0 server over stdin/stdout.
//!
//! Implements the Model Context Protocol transport layer with `tokio::select!`
//! for concurrent stdin reading and response writing. Handles `initialize`,
//! `tools/list`, `tools/call`, `resources/list`, `resources/read`, and
//! `sampling/createMessage` requests.
//!
//! Uses `CancellationToken` for graceful shutdown on SIGTERM/SIGINT.

use crate::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use crate::sampling::{
    OutgoingRequest, SamplingClient, SamplingClientConfig, create_sampling_jsonrpc_request,
    parse_sampling_response,
};
use crate::tools;
use crate::upload::UploadStore;
use pdf_core::McpPdfPipeline;
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::signal;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

#[derive(Debug, Default)]
pub struct ToolMetric {
    pub calls: AtomicU64,
    pub latency_ms: AtomicU64,
    pub errors: AtomicU64,
}

impl ToolMetric {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn to_json(&self) -> serde_json::Value {
        let calls = self.calls.load(Ordering::Relaxed);
        let latency = self.latency_ms.load(Ordering::Relaxed);
        let errors = self.errors.load(Ordering::Relaxed);
        serde_json::json!({
            "calls": calls,
            "latency_ms": latency,
            "errors": errors,
            "avg_latency_ms": latency.checked_div(calls).unwrap_or(0)
        })
    }
}

pub struct ToolStats {
    pub total_calls: AtomicU64,
    pub total_latency_ms: AtomicU64,
    pub total_errors: AtomicU64,
    pub files_processed: AtomicU64,
    pub start_time: u64,
    metrics: HashMap<&'static str, ToolMetric>,
}

impl ToolStats {
    pub fn new() -> Self {
        let metrics = HashMap::from([
            ("extract_text", ToolMetric::new()),
            ("extract_structured", ToolMetric::new()),
            ("get_page_count", ToolMetric::new()),
            ("search_keywords", ToolMetric::new()),
            ("compile_to_wiki", ToolMetric::new()),
            ("incremental_compile", ToolMetric::new()),
            ("search_knowledge", ToolMetric::new()),
            ("rebuild_index", ToolMetric::new()),
            ("get_entry_context", ToolMetric::new()),
            ("find_orphans", ToolMetric::new()),
            ("suggest_links", ToolMetric::new()),
            ("export_concept_map", ToolMetric::new()),
            ("check_quality", ToolMetric::new()),
            ("micro_compile", ToolMetric::new()),
            ("aggregate_entries", ToolMetric::new()),
            ("hypothesis_test", ToolMetric::new()),
            ("recompile_entry", ToolMetric::new()),
            ("compile_uploaded_pdf", ToolMetric::new()),
            ("get_config", ToolMetric::new()),
            ("set_config", ToolMetric::new()),
            ("get_health_report", ToolMetric::new()),
            ("trigger_incremental_compile", ToolMetric::new()),
            ("get_compile_status", ToolMetric::new()),
        ]);

        Self {
            total_calls: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            files_processed: AtomicU64::new(0),
            start_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("System time is before UNIX epoch")
                .as_secs(),
            metrics,
        }
    }

    pub fn record_success(&self, tool: &str, latency_ms: u64) {
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
        self.files_processed.fetch_add(1, Ordering::Relaxed);

        if let Some(m) = self.metrics.get(tool) {
            m.calls.fetch_add(1, Ordering::Relaxed);
            m.latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
        }
    }

    pub fn record_error(&self, tool: &str) {
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        self.total_errors.fetch_add(1, Ordering::Relaxed);

        if let Some(m) = self.metrics.get(tool) {
            m.calls.fetch_add(1, Ordering::Relaxed);
            m.errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("System time is before UNIX epoch")
            .as_secs()
            - self.start_time
    }

    pub fn to_json(&self) -> serde_json::Value {
        let total = self.total_calls.load(Ordering::Relaxed);
        let errors = self.total_errors.load(Ordering::Relaxed);
        let latency = self.total_latency_ms.load(Ordering::Relaxed);

        let tools_json: serde_json::Map<String, serde_json::Value> =
            self.metrics.iter().map(|(k, v)| ((*k).to_string(), v.to_json())).collect();

        serde_json::json!({
            "uptime_secs": self.uptime_secs(),
            "total_calls": total,
            "total_errors": errors,
            "total_latency_ms": latency,
            "avg_latency_ms": latency.checked_div(total).unwrap_or(0),
            "success_rate_pct": if total > 0 {
                ((total - errors) as f64 / total as f64 * 100.0).round()
            } else {
                100.0
            },
            "files_processed": self.files_processed.load(Ordering::Relaxed),
            "tools": tools_json
        })
    }
}

impl Default for ToolStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Run the MCP stdio server with an externally provided ToolContext.
#[tracing::instrument(skip(_pipeline, ctx, _upload_store))]
pub async fn run_stdio_with_tool_ctx(
    _pipeline: Arc<McpPdfPipeline>,
    ctx: tools::ToolContext,
    _upload_store: Arc<UploadStore>,
) -> anyhow::Result<()> {
    info!("MCP server listening on stdio");

    let stats = Arc::new(ToolStats::new());
    let shutdown_token = CancellationToken::new();
    let shutdown_token_clone = shutdown_token.clone();

    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                shutdown_token_clone.cancel();
                info!("Received shutdown signal, finishing current request...");
            }
            Err(err) => {
                error!("Unable to listen for shutdown signal: {}", err);
            }
        }
    });

    let sampling_config = SamplingClientConfig::default();
    let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<OutgoingRequest>(100);
    let sampling_client =
        Arc::new(SamplingClient::with_sender(sampling_config.timeout_secs, outgoing_tx.clone()));
    let pending_requests = sampling_client.pending_requests();
    let ctx = ctx.with_sampling(Arc::clone(&sampling_client));

    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();

    let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(100);

    tokio::task::spawn_blocking(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => {
                    if stdin_tx.blocking_send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    loop {
        tokio::select! {
            Some(line) = stdin_rx.recv() => {
                if shutdown_token.is_cancelled() {
                    info!("Shutting down gracefully...");
                    break;
                }

                info!(
                    "Received: {}",
                    if line.len() > 100 { &line[..100] } else { &line }
                );

                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line)
                    && value.get("method").is_none() && (value.get("result").is_some() || value.get("error").is_some()) {
                        match parse_sampling_response(&value) {
                            Ok((id, result)) => {
                                info!("Received sampling response for id={}", id);
                                let pending = pending_requests.clone();
                                tokio::spawn(async move {
                                    let response_tx = {
                                        let mut pending = pending.write().await;
                                        pending.remove(&id)
                                    };
                                    if let Some(tx) = response_tx {
                                        let _ = tx.send(result);
                                    }
                                });
                                continue;
                            }
                            Err(e) => {
                                error!("Failed to parse sampling response: {}", e);
                                continue;
                            }
                        }
                    }

                let request: JsonRpcRequest = match serde_json::from_str::<JsonRpcRequest>(&line) {
                    Ok(req) => {
                        info!("Parsed request: method={}", req.method);
                        req
                    }
                    Err(e) => {
                        error!("Failed to parse request: {}", e);
                        let response = JsonRpcResponse::error(None, JsonRpcError::parse_error());
                        write_response(&mut stdout_lock, &response)?;
                        continue;
                    }
                };

                let response = handle_request(&ctx, &stats, request).await;
                if let Some(resp) = response {
                    info!("Sending response for id={:?}", resp.id);
                    write_response(&mut stdout_lock, &resp)?;
                }
            }

            Some(outgoing) = outgoing_rx.recv() => {
                let json_request = create_sampling_jsonrpc_request(outgoing.id, outgoing.request);
                let json_str = serde_json::to_string(&json_request)?;
                info!("Sending sampling request: id={}", outgoing.id);
                writeln!(stdout_lock, "{}", json_str)?;
                stdout_lock.flush()?;
            }

            _ = shutdown_token.cancelled() => {
                break;
            }
        }
    }

    drop(stdin_rx);
    info!("Server shut down gracefully");
    Ok(())
}

fn write_response(
    stdout: &mut std::io::StdoutLock,
    response: &JsonRpcResponse,
) -> anyhow::Result<()> {
    let json = serde_json::to_string(response)?;
    debug!("Sending: {}", json);
    writeln!(stdout, "{}", json)?;
    stdout.flush()?;
    Ok(())
}

#[tracing::instrument(skip(ctx, stats, request), fields(method = %request.method))]
pub async fn handle_request(
    ctx: &tools::ToolContext,
    stats: &Arc<ToolStats>,
    request: JsonRpcRequest,
) -> Option<JsonRpcResponse> {
    if request.method.starts_with("notifications/") {
        return None;
    }

    let response = match request.method.as_str() {
        "initialize" => handle_initialize(stats, &request),
        "tools/list" => handle_tools_list(&request),
        "tools/call" => handle_tools_call(ctx, stats, &request).await,
        "resources/list" => tools::handle_resources_list(&request),
        "resources/read" => tools::handle_resources_read(&request),
        _ => JsonRpcResponse::error(request.id, JsonRpcError::method_not_found(&request.method)),
    };
    Some(response)
}

fn handle_initialize(stats: &Arc<ToolStats>, request: &JsonRpcRequest) -> JsonRpcResponse {
    let stats_json = stats.to_json();
    let mode = tools::mcp_mode_label();
    let tool_count = if mode == "code" {
        pdf_mcp_contracts::code_mode_tool_count()
    } else {
        pdf_mcp_contracts::tool_count()
    };
    let instructions = if mode == "code" {
        pdf_mcp_contracts::code_mode_instructions()
    } else {
        "Knowledge engine (contract 1.1.0). FIRST read schema/AGENTS.md in the knowledge base. Three commands: ingest → compile_to_wiki/incremental_compile → save_wiki_entry → complete_compile_job; query → read wiki/index.md then get_agent_context or search_knowledge (mode wiki_first); lint → lint_wiki. Also: init_knowledge_base, archive_answer for query write-back. Wiki: patch_wiki_entry, search_knowledge. PDF: extract_*."
    };
    let result = serde_json::json!({
        "protocolVersion": "2024-11-05",
        "serverInfo": {
            "name": "rust-pdf-mcp",
            "version": "0.6.0",
            "description": "AI-native knowledge compilation engine — PDF extraction, Karpathy compiler pattern, Tantivy fulltext search (CJK-aware), petgraph knowledge graph, hierarchical compilation, dynamic reasoning. Pure Rust, single binary."
        },
        "capabilities": {
            "tools": { "listChanged": false },
            "resources": { "listChanged": false },
            "sampling": {
                "supported": true,
                "messageTypes": ["text", "image"]
            },
            "extensions": {
                "compendium": {
                    "mode": mode,
                    "outputSchema": true,
                    "contractVersion": pdf_mcp_contracts::CONTRACT_VERSION,
                    "toolCount": tool_count,
                    "manifestSha256": pdf_mcp_contracts::manifest_sha256(),
                    "apiCatalogSize": pdf_mcp_contracts::tool_count()
                }
            }
        },
        "instructions": instructions,
        "stats": stats_json
    });
    JsonRpcResponse::success(request.id.clone(), result)
}

fn handle_tools_list(request: &JsonRpcRequest) -> JsonRpcResponse {
    let tools = tools::all_tool_definitions();
    JsonRpcResponse::success(request.id.clone(), serde_json::json!({ "tools": tools }))
}

#[tracing::instrument(skip(ctx, stats, request), fields(tool = ?request.params.get("name")))]
async fn handle_tools_call(
    ctx: &tools::ToolContext,
    stats: &Arc<ToolStats>,
    request: &JsonRpcRequest,
) -> JsonRpcResponse {
    let params = &request.params;

    let tool_name = match params.get("name").and_then(|n| n.as_str()) {
        Some(name) => name,
        None => {
            return JsonRpcResponse::error(
                request.id.clone(),
                JsonRpcError::invalid_params("Missing tool name"),
            );
        }
    };

    let arguments = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));

    let start = std::time::Instant::now();
    let result = tools::dispatch_tool(ctx, tool_name, &arguments).await;
    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(content) => {
            stats.record_success(tool_name, latency_ms);
            JsonRpcResponse::success(request.id.clone(), serde_json::json!({ "content": content }))
        }
        Err(e) => {
            stats.record_error(tool_name);
            JsonRpcResponse::error(request.id.clone(), JsonRpcError::internal_error(&e.to_string()))
        }
    }
}
