//! # pdf-dashboard
//!
//! Lightweight HTTP server for PDF module monitoring dashboard.
//! Exposes real-time metrics, system status, and tool usage via REST API.

use axum::{
    extract::State,
    http::{header, Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::net::SocketAddr;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    stats: Arc<ToolStats>,
    activity_log: Arc<ActivityLog>,
    start_time: u64,
}

/// Per-tool metrics, identified by name (same pattern as server.rs).
#[derive(Debug, Default)]
pub struct ToolMetric {
    pub calls: AtomicU64,
    pub latency_ms: AtomicU64,
    pub errors: AtomicU64,
}

#[derive(Default)]
struct ToolStats {
    tools: std::sync::RwLock<Vec<(&'static str, ToolMetric)>>,
    files_processed: AtomicU64,
}

impl ToolStats {
    fn new() -> Self {
        let tool_names: &[&str] = &[
            "extract_text",
            "extract_structured",
            "get_page_count",
            "search_keywords",
            "extrude_to_server_wiki",
            "extrude_to_agent_payload",
            "compile_to_wiki",
            "incremental_compile",
            "search_knowledge",
            "rebuild_index",
            "get_entry_context",
            "find_orphans",
            "suggest_links",
            "export_concept_map",
            "check_quality",
            "micro_compile",
            "aggregate_entries",
            "hypothesis_test",
            "recompile_entry",
        ];
        let tools = tool_names
            .iter()
            .map(|name| (*name, ToolMetric::default()))
            .collect();
        Self {
            tools: std::sync::RwLock::new(tools),
            ..Default::default()
        }
    }

    fn record(&self, tool: &str, latency_ms: u64, success: bool) {
        self.files_processed.fetch_add(1, Ordering::Relaxed);
        if let Ok(tools) = self.tools.read() {
            if let Some((_, metric)) = tools.iter().find(|(name, _)| *name == tool) {
                metric.calls.fetch_add(1, Ordering::Relaxed);
                metric.latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
                if !success {
                    metric.errors.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
}

#[derive(Serialize)]
struct ToolStat {
    name: String,
    calls: u64,
    latency: u64,
    success_rate: f64,
}

#[derive(Serialize)]
struct DashboardMetrics {
    total_calls: u64,
    avg_latency_ms: u64,
    success_rate: f64,
    files_processed: u64,
    tools: Vec<ToolStat>,
    uptime_secs: u64,
    start_timestamp: u64,
}

#[derive(Serialize)]
struct SystemStatus {
    memory_percent: f64,
    pdfium_ready: bool,
    pdfium_version: String,
    queue_length: u32,
    vlm_enabled: bool,
    vlm_model: String,
    vlm_thinking: bool,
    vlm_function_call: bool,
    vlm_multi_model_routing: bool,
}

#[derive(Serialize)]
struct HealthCheck {
    status: String,
    mcp_healthy: bool,
    client_connections: u32,
    uptime_secs: u64,
    version: String,
}

#[derive(Deserialize)]
struct McpProxyRequest {
    command: String,
    request: serde_json::Value,
}

#[derive(Serialize)]
struct McpProxyResponse {
    result: Option<serde_json::Value>,
    error: Option<String>,
}

#[derive(Clone)]
struct ActivityLog {
    entries: Arc<parking_lot::RwLock<VecDeque<LogEntry>>>,
}

#[derive(Serialize, Clone)]
struct LogEntry {
    level: String,
    time: String,
    message: String,
}

impl ActivityLog {
    fn new() -> Self {
        Self {
            entries: Arc::new(parking_lot::RwLock::new(VecDeque::with_capacity(200))),
        }
    }

    fn add(&self, level: &str, message: &str) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let secs = now.as_secs();
        let time = format!(
            "{:02}:{:02}:{:02}",
            (secs / 3600) % 24,
            (secs / 60) % 60,
            secs % 60
        );
        let mut entries = self.entries.write();
        entries.push_back(LogEntry {
            level: level.to_string(),
            time,
            message: message.to_string(),
        });
        if entries.len() > 200 {
            entries.pop_front();
        }
    }

    fn get(&self) -> Vec<LogEntry> {
        self.entries.read().iter().cloned().collect()
    }
}

async fn health(State(state): State<AppState>) -> Json<HealthCheck> {
    Json(HealthCheck {
        status: "ok".to_string(),
        mcp_healthy: true,
        client_connections: 1,
        uptime_secs: current_uptime(&state),
        version: "0.3.0".to_string(),
    })
}

#[tracing::instrument(skip(state, req), fields(command = %req.command))]
async fn mcp_proxy(
    State(state): State<AppState>,
    Json(req): Json<McpProxyRequest>,
) -> Result<Json<McpProxyResponse>, (StatusCode, String)> {
    let request_str = serde_json::to_string(&req.request)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let mut child = Command::new(&req.command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to spawn MCP server: {}", e),
            )
        })?;

    let stdin = child
        .stdin
        .as_mut()
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "No stdin".into()))?;

    // MCP uses line protocol, not LSP Content-Length format
    let line_message = format!("{}\n", request_str);
    stdin
        .write_all(line_message.as_bytes())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    stdin
        .flush()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let stdout = child
        .stdout
        .as_mut()
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "No stdout".into()))?;

    // Read line protocol response using BufReader for efficiency
    let reader = BufReader::new(stdout);
    let mut lines_read = 0;
    let mut response: Option<serde_json::Value> = None;

    for line in reader.lines() {
        match line {
            Ok(line_str) => {
                lines_read += 1;

                // Check if this is a JSON-RPC response (starts with {"jsonrpc")
                if line_str.starts_with("{\"jsonrpc") {
                    match serde_json::from_str::<serde_json::Value>(&line_str) {
                        Ok(val) => {
                            // Check if this is a response to our request (has matching id)
                            if let Some(id) = req.request.get("id") {
                                if val.get("id") == Some(id) {
                                    response = Some(val);
                                    break;
                                }
                            } else {
                                // For requests without id, take the first valid response
                                response = Some(val);
                                break;
                            }
                        }
                        Err(_) => continue,
                    }
                }

                if lines_read > 1000 {
                    let _ = child.kill();
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Too many log lines before response".into(),
                    ));
                }
            }
            Err(e) => {
                let _ = child.kill();
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Read error: {}", e),
                ));
            }
        }
    }

    let _ = child.kill();

    let response = response.ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "No JSON-RPC response".into(),
    ))?;

    if let Some(params) = req.request.get("params") {
        if let Some(tool_name) = params.get("name").and_then(|n| n.as_str()) {
            if response.get("result").is_some() {
                state
                    .activity_log
                    .add("info", &format!("Tool {} executed successfully", tool_name));
            }
        }
    }

    Ok(Json(McpProxyResponse {
        result: Some(response),
        error: None,
    }))
}

async fn metrics(State(state): State<AppState>) -> Json<DashboardMetrics> {
    let stats = &state.stats;
    let tools_lock = stats.tools.read().unwrap();

    let mut total_calls: u64 = 0;
    let mut total_latency: u64 = 0;
    let mut total_errors: u64 = 0;
    let mut tool_stats: Vec<ToolStat> = Vec::with_capacity(tools_lock.len());

    for (name, metric) in tools_lock.iter() {
        let calls = metric.calls.load(Ordering::Relaxed);
        let latency = metric.latency_ms.load(Ordering::Relaxed);
        let errors = metric.errors.load(Ordering::Relaxed);

        total_calls += calls;
        total_latency += latency;
        total_errors += errors;

        let avg = latency.checked_div(calls).unwrap_or(0);
        let rate = if calls > 0 {
            ((calls - errors) as f64 / calls as f64) * 100.0
        } else {
            100.0
        };

        tool_stats.push(ToolStat {
            name: name.to_string(),
            calls,
            latency: avg,
            success_rate: rate,
        });
    }

    let avg_latency = total_latency.checked_div(total_calls).unwrap_or(0);
    let success_rate = if total_calls > 0 {
        ((total_calls - total_errors) as f64 / total_calls as f64) * 100.0
    } else {
        100.0
    };
    let files_processed = stats.files_processed.load(Ordering::Relaxed);

    Json(DashboardMetrics {
        total_calls,
        avg_latency_ms: avg_latency,
        success_rate,
        files_processed,
        tools: tool_stats,
        uptime_secs: current_uptime(&state),
        start_timestamp: state.start_time,
    })
}

async fn system_status(State(_state): State<AppState>) -> Json<SystemStatus> {
    let mem = read_memory_usage().unwrap_or(0.0);

    Json(SystemStatus {
        memory_percent: mem,
        pdfium_ready: true,
        pdfium_version: "4.04.0".to_string(),
        queue_length: 0,
        vlm_enabled: std::env::var("VLM_MODEL").is_ok(),
        vlm_model: std::env::var("VLM_MODEL").unwrap_or_else(|_| "none".to_string()),
        vlm_thinking: std::env::var("VLM_ENABLE_THINKING")
            .map(|v| v == "true")
            .unwrap_or(true),
        vlm_function_call: std::env::var("VLM_ENABLE_FUNCTION_CALL")
            .map(|v| v == "true")
            .unwrap_or(false),
        vlm_multi_model_routing: std::env::var("VLM_ENABLE_MULTI_MODEL_ROUTING")
            .map(|v| v != "false")
            .unwrap_or(true),
    })
}

async fn get_activity_log(State(state): State<AppState>) -> Json<Vec<LogEntry>> {
    Json(state.activity_log.get())
}

async fn clear_log(State(state): State<AppState>) -> impl IntoResponse {
    state.activity_log.entries.write().clear();
    (StatusCode::OK, "cleared")
}

async fn record_tool_call(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let tool = body.get("tool").and_then(|v| v.as_str()).unwrap_or("");
    let latency = body.get("latency_ms").and_then(|v| v.as_u64()).unwrap_or(0);
    let success = body
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    state.stats.record(tool, latency, success);

    let level = if success { "info" } else { "error" };
    let msg = if success {
        format!("{} completed in {}ms", tool, latency)
    } else {
        format!("{} failed after {}ms", tool, latency)
    };
    state.activity_log.add(level, &msg);

    StatusCode::OK
}

fn current_uptime(state: &AppState) -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() - state.start_time)
        .unwrap_or(0)
}

fn read_memory_usage() -> Option<f64> {
    // Read from /proc/self/status on Linux
    #[cfg(target_os = "linux")]
    {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        let file = File::open("/proc/self/status").ok()?;
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<f64>() {
                        let total_mem = total_memory_kb().unwrap_or(1_000_000);
                        return Some((kb / total_mem as f64) * 100.0);
                    }
                }
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn total_memory_kb() -> Option<u64> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open("/proc/meminfo").ok()?;
    let reader = BufReader::new(file);
    for line in reader.lines().map_while(Result::ok) {
        if line.starts_with("MemTotal:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse().ok();
            }
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn total_memory_kb() -> Option<u64> {
    None
}

#[tracing::instrument]
pub async fn run_dashboard(bind: &str) -> anyhow::Result<()> {
    let addr: SocketAddr = bind.parse()?;

    let stats = Arc::new(ToolStats::new());
    let activity_log = Arc::new(ActivityLog::new());

    let start_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let state = AppState {
        stats,
        activity_log: activity_log.clone(),
        start_time,
    };

    activity_log.add("info", "Dashboard server starting");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/metrics", get(metrics))
        .route("/api/status", get(system_status))
        .route("/api/logs", get(get_activity_log))
        .route("/api/logs/clear", post(clear_log))
        .route("/api/record", post(record_tool_call))
        .route("/api/mcp", post(mcp_proxy))
        .layer(cors)
        .with_state(state);

    info!("Dashboard listening on http://{}", addr);
    info!("Dashboard endpoints: /api/health, /api/metrics, /api/status, /api/logs");

    let listener = tokio::net::TcpListener::bind(addr).await?;

    activity_log.add("info", "Dashboard server started");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    warn!("Shutdown signal received, stopping dashboard...");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let bind = std::env::var("DASHBOARD_BIND").unwrap_or_else(|_| "0.0.0.0:8000".to_string());

    info!("Starting PDF Module Dashboard Server");
    info!("Bind address: {}", bind);

    run_dashboard(&bind).await
}
