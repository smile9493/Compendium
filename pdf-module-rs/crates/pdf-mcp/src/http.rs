//! # Unified HTTP Server
//!
//! Axum-based HTTP server serving wiki REST API, management API, and the Vue3 SPA.
//!
//! ## API Endpoints
//!
//! ### Wiki (read-only)
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | GET | `/api/wiki/tree` | Wiki directory tree |
//! | GET | `/api/wiki/entries/*path` | Single entry (SSR HTML + metadata) |
//! | GET | `/api/wiki/search?q=...` | Full-text search |
//! | GET | `/api/wiki/graph/*path` | Concept graph (Mermaid) |
//! | GET | `/api/wiki/stats` | Knowledge base stats |
//! | GET | `/api/wiki/domains` | Domain list |
//!
//! ### Management (read-write)
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | GET | `/api/health` | Health report |
//! | GET | `/api/server-info` | MCP mode and Cursor config snippet |
//! | GET | `/api/config` | Runtime config |
//! | POST | `/api/config` | Set config key |
//! | DELETE | `/api/config/{key}` | Remove config key |
//! | GET | `/api/compile/status` | Compile status |
//! | POST | `/api/index/rebuild` | Rebuild indexes |
//! | POST | `/mcp` | MCP JSON-RPC (`tools/call`, `initialize`, `tools/list`) |
//!
//! ### SPA
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | GET | `/` | Redirect to SPA |
//! | GET | `/app/{*path}` | Vue3 SPA (with history fallback) |

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_stream::stream;
use axum::Router;
use axum::extract::{Multipart, Path, Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Redirect};
use axum::routing::{delete, get, post};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tracing::{info, instrument};

use pdf_core::McpPdfPipeline;
use pdf_core::knowledge::index::MetadataStore;
use pdf_core::knowledge::quality::analyze_wiki;
use pdf_core::knowledge::renderer::WikiRenderer;
use pdf_core::knowledge::{
    IndexCache, KnowledgeEngine, SearchMode, SearchOptions, SearchResponse, rebuild_all,
};
use pdf_core::knowledge::{run_incremental_extract, run_single_pdf_extract};
use pdf_core::management::{
    CompileJobStore, ConfigManager, HealthReporter, QualitySnapshotStore, WorkspaceRegistry,
    build_compile_status_json,
};

use crate::embed::Assets;
use crate::http_schemas::ServerInfoHttp;
use crate::metrics::{self, HttpMetrics, MetricsLayer};
use crate::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use crate::server::{ToolStats, handle_request};
use crate::tools::ToolContext;
use crate::tools::mcp_extraction::{extraction_health_default, extraction_health_from_pipeline};
use crate::tools::mcp_mode_label;
use crate::upload::UploadStore;
use crate::version::UpdateCache;
use crate::version::github::{GithubClient, check_for_updates as github_check};
use crate::version::{VersionInfo};
use pdf_mcp_contracts::{CONTRACT_VERSION, code_mode_tool_count, manifest_sha256, tool_count};

#[derive(Clone)]
pub struct HttpState {
    pub kb_path: Option<PathBuf>,
    pub workspace_registry: Arc<WorkspaceRegistry>,
    pub upload_store: Option<Arc<UploadStore>>,
    pub pipeline: Option<Arc<McpPdfPipeline>>,
    pub http_metrics: Option<Arc<HttpMetrics>>,
    pub index_cache: Arc<IndexCache>,
    /// Current version info (embedded at compile time)
    pub version_info: VersionInfo,
    /// GitHub API client for update checking
    pub github_client: Arc<GithubClient>,
    /// Server-side cache for update check results (1h TTL)
    pub update_cache: Arc<UpdateCache>,
}

fn resolve_kb_from_request(state: &HttpState, kb_id: Option<&str>) -> Option<PathBuf> {
    if let Some(id) = kb_id {
        state.workspace_registry.path_for_id(id).ok()
    } else if let Some(ref p) = state.kb_path {
        Some(p.clone())
    } else {
        state.workspace_registry.resolve_kb(None, None).ok()
    }
}

fn normalize_wiki_entry_path(path: &str) -> String {
    let path = path.trim().trim_start_matches('/');
    if path.is_empty() {
        return String::new();
    }
    if path.ends_with(".md") { path.to_string() } else { format!("{path}.md") }
}

#[instrument(skip(state))]
pub async fn run_http_server(
    state: HttpState,
    port: u16,
    ready_tx: Option<oneshot::Sender<()>>,
) -> anyhow::Result<()> {
    let app = build_router(state);

    let addr = format!("0.0.0.0:{}", port);
    info!(addr = %addr, "Starting unified HTTP server");

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    if let Some(tx) = ready_tx {
        let _ = tx.send(());
    }

    axum::serve(listener, app).await.map_err(|e| anyhow::anyhow!("HTTP server error: {}", e))
}

fn build_router(state: HttpState) -> Router {
    let mut router = Router::new()
        // ── Wiki API ──
        .route("/api/wiki/tree", get(api_wiki_tree))
        .route("/api/wiki/entries/*path", get(api_wiki_entry))
        .route("/api/wiki/search", get(api_wiki_search))
        .route("/api/wiki/graph/*path", get(api_wiki_graph))
        .route("/api/wiki/stats", get(api_wiki_stats))
        .route("/api/wiki/domains", get(api_wiki_domains))
        // ── Management API ──
        .route("/api/config", get(api_config_get).post(api_config_set))
        .route("/api/config/{key}", delete(api_config_remove))
        .route("/api/health", get(api_health))
        .route("/api/server-info", get(api_server_info))
        .route("/api/compile/status", get(api_compile_status))
        .route("/api/compile/incremental", post(api_compile_incremental))
        .route("/api/compile/upload", post(api_compile_upload))
        .route("/api/upload", post(api_upload))
        .route("/api/quality/summary", get(api_quality_summary))
        .route("/api/quality/issues", get(api_quality_issues))
        .route("/api/index/rebuild", post(api_index_rebuild))
        .route("/api/index/status", get(api_index_status))
        .route("/api/compile/events", get(api_compile_events))
        .route("/api/v1/shares", post(api_shares_create))
        .route("/api/share/{token}/wiki/entries/*path", get(api_share_wiki_entry))
        .route("/api/v1/workspaces", get(api_workspaces_list).post(api_workspaces_upsert))
        .route("/api/v1/workspaces/active", post(api_workspaces_set_active))
        // ── Version & Update ──
        .route("/api/version", get(api_version))
        .route("/api/update/check", get(api_update_check))
        .route("/api/update/prepare", post(api_update_prepare))
        // ── MCP over HTTP (LAN / public — same JSON-RPC as stdio) ──
        .route("/mcp", post(api_mcp_jsonrpc))
        // ── SPA (legacy redirects) ──
        .route("/", get(|| async { Redirect::permanent("/app/") }))
        .route("/settings", get(|| async { Redirect::permanent("/app/") }))
        // ── SPA fallback (catches /app/* and serves SPA) ──
        .fallback(serve_spa);

    if let Some(ref metrics) = state.http_metrics {
        router = router
            .route("/metrics", get(metrics::metrics_endpoint))
            .layer(axum::extract::Extension(Arc::clone(metrics)))
            .layer(MetricsLayer::new(Arc::clone(metrics)));
    }

    router.with_state(Arc::new(state))
}

// ── SPA serving ──

/// Serve the Vue3 SPA using OriginalUri to extract the request path.
/// For requested paths under `/app/`, we serve static files from the
/// embedded dist and fall back to `index.html` for SPA client-side routing.
async fn serve_spa(uri: axum::extract::OriginalUri) -> impl IntoResponse {
    let path = uri.path();

    // Don't interfere with API paths (return 404 for unknown API endpoints)
    if path.starts_with("/api/") {
        return (axum::http::StatusCode::NOT_FOUND, "Not found").into_response();
    }

    // Strip leading slash and optional /app/ prefix for asset lookup
    let lookup = if let Some(rest) = path.strip_prefix("/app/") {
        if rest.is_empty() { "index.html" } else { rest }
    } else if path == "/app" {
        "index.html"
    } else {
        // For root or other paths, serve index.html
        // (most paths under / will redirect to /app/, but just in case)
        path.strip_prefix('/').unwrap_or("index.html")
    };

    if let Some(content) = Assets::get(lookup) {
        let mime = mime_from_path(lookup);
        return ([("Content-Type", mime)], content.data.into_owned()).into_response();
    }

    // SPA fallback: serve index.html for all unmatched non-API paths
    if let Some(content) = Assets::get("index.html") {
        return ([("Content-Type", "text/html; charset=utf-8")], content.data.into_owned())
            .into_response();
    }

    (axum::http::StatusCode::NOT_FOUND, "SPA not found (pdf-web-ui not built?)").into_response()
}

fn mime_from_path(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

// ── Wiki API handlers ──

#[instrument(skip(state))]
async fn api_wiki_tree(
    State(state): State<Arc<HttpState>>,
    Query(q): Query<KbQuery>,
) -> Json<serde_json::Value> {
    let kb = match resolve_kb_from_request(&state, q.kb_id.as_deref()) {
        Some(p) => p,
        None => return Json(serde_json::json!({"tree": {"name": "", "children": []}, "total": 0})),
    };

    let wiki_dir = kb.join("wiki");
    let renderer = WikiRenderer::new(&wiki_dir);

    match renderer.render_tree() {
        Ok(tree) => {
            let total = count_tree_entries(&tree);
            Json(serde_json::json!({"tree": tree, "total": total}))
        }
        Err(_) => Json(serde_json::json!({"tree": {"name": "", "children": []}, "total": 0})),
    }
}

#[instrument(skip(state))]
async fn api_wiki_entry(
    State(state): State<Arc<HttpState>>,
    Path(path): Path<String>,
    Query(q): Query<KbQuery>,
) -> Json<serde_json::Value> {
    let kb = match resolve_kb_from_request(&state, q.kb_id.as_deref()) {
        Some(p) => p,
        None => return Json(serde_json::json!({"error": "No knowledge base configured"})),
    };

    let wiki_dir = kb.join("wiki");
    let renderer = WikiRenderer::new(&wiki_dir);
    let entry_path = normalize_wiki_entry_path(&path);

    match renderer.render_entry(&entry_path) {
        Ok(entry) => Json(serde_json::json!({"entry": entry})),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    domain: Option<String>,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    kb_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KbQuery {
    #[serde(default)]
    kb_id: Option<String>,
}

fn default_limit() -> usize {
    20
}

#[instrument(skip(state))]
async fn api_wiki_search(
    State(state): State<Arc<HttpState>>,
    Query(query): Query<SearchQuery>,
) -> Json<serde_json::Value> {
    let kb = match resolve_kb_from_request(&state, query.kb_id.as_deref()) {
        Some(p) => p,
        None => return Json(serde_json::json!({"results": [], "total": 0})),
    };

    let wiki_dir = kb.join("wiki");
    if !wiki_dir.exists() {
        return Json(serde_json::json!({"results": [], "total": 0}));
    }

    let mode = query.mode.as_deref().map(SearchMode::parse).unwrap_or(SearchMode::Hybrid);
    let mut opts = SearchOptions::for_api();
    opts.domain = query.domain.clone();

    let SearchResponse { hits, meta } = match state.index_cache.search(
        &kb,
        &query.q,
        query.limit,
        mode,
        opts,
    ) {
        Ok(r) => r,
        Err(e) => {
            return Json(serde_json::json!({
                "results": [],
                "total": 0,
                "error": e.to_string(),
                "meta": { "index_empty": false, "used_fallback": false, "mode": format!("{:?}", mode).to_lowercase() },
            }));
        }
    };

    let lower_q = query.q.to_lowercase();
    let mut domain_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    let results: Vec<serde_json::Value> = hits
        .into_iter()
        .map(|h| {
            *domain_counts.entry(h.domain.clone()).or_insert(0) += 1;
            // Display-only highlight count; ranking uses Tantivy `score`.
            let highlight_count = h.snippet.to_lowercase().matches(&lower_q).count();
            serde_json::json!({
                "path": h.path,
                "title": h.title,
                "domain": h.domain,
                "score": h.score,
                "snippet": highlight_snippet(&h.snippet, &query.q),
                "highlight_count": highlight_count,
            })
        })
        .collect();

    let domain_facets: Vec<serde_json::Value> = domain_counts
        .into_iter()
        .map(|(d, c)| serde_json::json!({"domain": d, "count": c}))
        .collect();

    Json(serde_json::json!({
        "results": results,
        "total": results.len(),
        "query": query.q,
        "domain_facets": domain_facets,
        "meta": meta,
    }))
}

#[instrument(skip(state))]
async fn api_wiki_graph(
    State(state): State<Arc<HttpState>>,
    Path(path): Path<String>,
    Query(q): Query<KbQuery>,
) -> Json<serde_json::Value> {
    let kb = match resolve_kb_from_request(&state, q.kb_id.as_deref()) {
        Some(p) => p,
        None => return Json(serde_json::json!({"error": "No knowledge base configured"})),
    };

    if !kb.join("wiki").exists() {
        return Json(serde_json::json!({"error": "Wiki directory not found"}));
    }

    let indexes = match state.index_cache.graph(&kb) {
        Ok(g) => g,
        Err(e) => {
            return Json(serde_json::json!({"error": format!("Graph load failed: {}", e)}));
        }
    };

    let mermaid = indexes.graph.export_concept_map(&path, 2);
    Json(serde_json::json!({"mermaid": mermaid, "entry": path}))
}

#[instrument(skip(state))]
async fn api_wiki_stats(
    State(state): State<Arc<HttpState>>,
    Query(q): Query<KbQuery>,
) -> Json<serde_json::Value> {
    let kb = match resolve_kb_from_request(&state, q.kb_id.as_deref()) {
        Some(p) => p,
        None => {
            return Json(serde_json::json!({
                "total_entries": 0, "domains": [],
                "index_size_bytes": 0, "graph_node_count": 0, "graph_edge_count": 0,
            }));
        }
    };

    let reporter = HealthReporter::new(&kb);
    match reporter.report() {
        Ok(report) => Json(serde_json::json!({
            "total_entries": report.total_entries,
            "orphan_count": report.orphan_count,
            "contradiction_count": report.contradiction_count,
            "broken_link_count": report.broken_link_count,
            "index_size_bytes": report.index_size_bytes,
            "graph_node_count": report.graph_node_count,
            "graph_edge_count": report.graph_edge_count,
            "avg_quality_score": report.avg_quality_score,
            "domains": report.domains,
            "last_compile": report.last_compile.map(|t| t.to_rfc3339()),
        })),
        Err(e) => Json(serde_json::json!({"total_entries": 0, "error": e.to_string()})),
    }
}

#[instrument(skip(state))]
async fn api_wiki_domains(
    State(state): State<Arc<HttpState>>,
    Query(q): Query<KbQuery>,
) -> Json<serde_json::Value> {
    let kb = match resolve_kb_from_request(&state, q.kb_id.as_deref()) {
        Some(p) => p,
        None => return Json(serde_json::json!({"domains": []})),
    };

    let wiki_dir = kb.join("wiki");

    let mut domains = match MetadataStore::open(&kb) {
        Ok(store) => store.all_domains().unwrap_or_default(),
        Err(_) => vec![],
    };

    if domains.is_empty() {
        domains = scan_domains_from_fs(&wiki_dir);
    }

    let entries_by_domain = count_entries_by_domain(&wiki_dir);

    let domain_status: Vec<serde_json::Value> = domains
        .into_iter()
        .map(|d| {
            let count = entries_by_domain.get(&d).copied().unwrap_or(0);
            serde_json::json!({"domain": d, "entry_count": count})
        })
        .collect();

    Json(serde_json::json!({"domains": domain_status}))
}

// ── Management API handlers ──

fn build_mcp_config_snippet(mode: &str, knowledge_base_hint: &str) -> serde_json::Value {
    serde_json::json!({
        "mcpServers": {
            "pdf-mcp": {
                "command": "/path/to/pdf-mcp",
                "env": {
                    "COMPENDIUM_MCP_MODE": mode,
                    "KNOWLEDGE_BASE_PATH": knowledge_base_hint,
                    "PDFIUM_LIB_PATH": "/path/to/libpdfium.so"
                }
            }
        }
    })
}

#[instrument(skip(state))]
async fn api_server_info(State(state): State<Arc<HttpState>>) -> Json<ServerInfoHttp> {
    let mode = mcp_mode_label();
    let mcp_tool_count = if mode == "code" { code_mode_tool_count() } else { tool_count() };
    let knowledge_base_hint = state
        .kb_path
        .clone()
        .or_else(|| state.workspace_registry.resolve_kb(None, None).ok())
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "/path/to/my-kb".to_string());

    Json(ServerInfoHttp {
        mcp_mode: mode.to_string(),
        mcp_tool_count,
        api_catalog_size: tool_count(),
        contract_version: CONTRACT_VERSION.to_string(),
        manifest_sha256: manifest_sha256(),
        http_running: true,
        knowledge_base_hint: knowledge_base_hint.clone(),
        mcp_config_snippet: build_mcp_config_snippet(mode, &knowledge_base_hint),
    })
}

#[instrument(skip(state))]
async fn api_health(
    State(state): State<Arc<HttpState>>,
    Query(q): Query<KbQuery>,
) -> impl IntoResponse {
    let kb = match resolve_kb_from_request(&state, q.kb_id.as_deref()) {
        Some(p) => p,
        None => {
            return Json(serde_json::json!({"error": "No knowledge base configured"}))
                .into_response();
        }
    };

    let reporter = HealthReporter::new(&kb);
    match reporter.report() {
        Ok(report) => {
            let quality_snapshot = QualitySnapshotStore::new(&kb).read().unwrap_or_default();
            let extraction = state
                .pipeline
                .as_ref()
                .map(|p| extraction_health_from_pipeline(p))
                .unwrap_or_else(extraction_health_default);
            Json(serde_json::json!({
                "total_entries": report.total_entries,
                "orphan_count": report.orphan_count,
                "contradiction_count": report.contradiction_count,
                "broken_link_count": report.broken_link_count,
                "index_size_mb": report.index_size_bytes / 1024 / 1024,
                "graph_nodes": report.graph_node_count,
                "graph_edges": report.graph_edge_count,
                "avg_quality_score": format!("{:.1}%", report.avg_quality_score * 100.0),
                "domains": report.domains,
                "last_compile": report.last_compile.map(|t| t.to_rfc3339()),
                "generated_at": report.generated_at.to_rfc3339(),
                "report_text": report.to_string(),
                "quality_snapshot": quality_snapshot,
                "extraction": extraction,
            }))
            .into_response()
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[instrument(skip(state))]
async fn api_config_get(
    State(state): State<Arc<HttpState>>,
    Query(q): Query<KbQuery>,
) -> impl IntoResponse {
    let kb = match resolve_kb_from_request(&state, q.kb_id.as_deref()) {
        Some(p) => p,
        None => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "No KB"})),
            )
                .into_response();
        }
    };

    let mut cm = ConfigManager::new(&kb);
    if let Err(e) = cm.load() {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }
    Json(serde_json::json!({"config": cm.all(), "total_keys": cm.all().len()})).into_response()
}

#[derive(Debug, Deserialize)]
struct SetConfigBody {
    key: String,
    value: String,
}

#[instrument(skip(state))]
async fn api_config_set(
    State(state): State<Arc<HttpState>>,
    Query(q): Query<KbQuery>,
    Json(body): Json<SetConfigBody>,
) -> impl IntoResponse {
    let kb = match resolve_kb_from_request(&state, q.kb_id.as_deref()) {
        Some(p) => p,
        None => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "No KB"})),
            )
                .into_response();
        }
    };

    let mut cm = ConfigManager::new(&kb);
    if let Err(e) = cm.load() {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }
    match cm.set(&body.key, &body.value) {
        Ok(()) => Json(serde_json::json!({"status": "ok", "key": body.key})).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[instrument(skip(state))]
async fn api_config_remove(
    State(state): State<Arc<HttpState>>,
    Path(key): Path<String>,
    Query(q): Query<KbQuery>,
) -> impl IntoResponse {
    let kb = match resolve_kb_from_request(&state, q.kb_id.as_deref()) {
        Some(p) => p,
        None => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "No KB"})),
            )
                .into_response();
        }
    };

    let mut cm = ConfigManager::new(&kb);
    if let Err(e) = cm.load() {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }
    match cm.remove(&key) {
        Ok(()) => Json(serde_json::json!({"status": "ok", "removed": key})).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[instrument(skip(state, multipart))]
async fn api_upload(
    State(state): State<Arc<HttpState>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let store = match &state.upload_store {
        Some(s) => s,
        None => {
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "Upload not configured"})),
            )
                .into_response();
        }
    };

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() != Some("file") {
            continue;
        }
        let filename = field.file_name().unwrap_or("upload.pdf").to_string();
        let data = match field.bytes().await {
            Ok(b) => b,
            Err(e) => {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response();
            }
        };
        match store.store(&data, &filename) {
            Ok(file_id) => {
                return Json(serde_json::json!({
                    "file_id": file_id,
                    "filename": filename,
                    "size_bytes": data.len(),
                }))
                .into_response();
            }
            Err(e) => {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response();
            }
        }
    }

    (
        axum::http::StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": "No file field in multipart body"})),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
struct CompileUploadBody {
    file_id: String,
    domain: Option<String>,
}

#[instrument(skip(state, body))]
async fn api_compile_upload(
    State(state): State<Arc<HttpState>>,
    Query(q): Query<KbQuery>,
    Json(body): Json<CompileUploadBody>,
) -> impl IntoResponse {
    let kb = match kb_or_error(&state, q.kb_id.as_deref()) {
        Ok(k) => k,
        Err(r) => return *r,
    };
    let pipeline = match &state.pipeline {
        Some(p) => Arc::clone(p),
        None => {
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "Pipeline not configured"})),
            )
                .into_response();
        }
    };
    let upload_store = match &state.upload_store {
        Some(s) => s,
        None => {
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "Upload not configured"})),
            )
                .into_response();
        }
    };

    let uploaded = match upload_store.get(&body.file_id) {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "File not found or expired"})),
            )
                .into_response();
        }
    };

    let engine = match KnowledgeEngine::new(pipeline, &kb) {
        Ok(e) => e,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    let job_store = CompileJobStore::new(&kb);
    let compile_result =
        run_single_pdf_extract(&engine, &job_store, &uploaded.temp_path, body.domain.as_deref())
            .await;
    match compile_result {
        Ok((job_id, result)) => {
            upload_store.remove(&body.file_id);
            Json(serde_json::json!({
                "job_id": job_id,
                "pipeline_status": "awaiting_agent",
                "compile_result": result,
            }))
            .into_response()
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[instrument(skip(state))]
async fn api_compile_incremental(
    State(state): State<Arc<HttpState>>,
    Query(q): Query<KbQuery>,
) -> impl IntoResponse {
    let kb = match kb_or_error(&state, q.kb_id.as_deref()) {
        Ok(k) => k,
        Err(r) => return *r,
    };
    let pipeline = match &state.pipeline {
        Some(p) => Arc::clone(p),
        None => {
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "Pipeline not configured"})),
            )
                .into_response();
        }
    };

    let engine = match KnowledgeEngine::new(pipeline, &kb) {
        Ok(e) => e,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    let job_store = CompileJobStore::new(&kb);
    match run_incremental_extract(&engine, &job_store).await {
        Ok((job_id, result)) => Json(serde_json::json!({
            "job_id": job_id,
            "pipeline_status": "awaiting_agent",
            "incremental_result": result,
        }))
        .into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

fn kb_or_error(
    state: &HttpState,
    kb_id: Option<&str>,
) -> Result<PathBuf, Box<axum::response::Response>> {
    resolve_kb_from_request(state, kb_id).ok_or_else(|| {
        Box::new(
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "No knowledge base configured"})),
            )
                .into_response(),
        )
    })
}

#[instrument(skip(state))]
async fn api_compile_status(
    State(state): State<Arc<HttpState>>,
    Query(q): Query<KbQuery>,
) -> impl IntoResponse {
    let kb = match resolve_kb_from_request(&state, q.kb_id.as_deref()) {
        Some(p) => p,
        None => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "No KB"})),
            )
                .into_response();
        }
    };

    match build_compile_status_json(&kb) {
        Ok(v) => Json(v).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[instrument(skip(state))]
async fn api_index_rebuild(
    State(state): State<Arc<HttpState>>,
    Query(q): Query<KbQuery>,
) -> impl IntoResponse {
    let kb = match resolve_kb_from_request(&state, q.kb_id.as_deref()) {
        Some(p) => p,
        None => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "No KB"})),
            )
                .into_response();
        }
    };

    if !kb.join("wiki").exists() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Wiki directory not found"})),
        )
            .into_response();
    }

    match rebuild_all(&kb) {
        Ok(stats) => {
            state.index_cache.invalidate(&kb);
            let payload = serde_json::json!({
                "status": "success",
                "fulltext_entries_indexed": stats.fulltext_entries_indexed,
                "graph_nodes": stats.graph_nodes,
                "graph_edges": stats.graph_edges,
                "vector_entries_indexed": stats.vector_entries_indexed,
            });
            let _ = write_index_meta(&kb, &payload);
            Json(payload).into_response()
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct QualityIssuesQuery {
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    severity: Option<String>,
    #[serde(default)]
    kb_id: Option<String>,
}

#[instrument(skip(state))]
async fn api_quality_summary(
    State(state): State<Arc<HttpState>>,
    Query(q): Query<KbQuery>,
) -> impl IntoResponse {
    let kb = match resolve_kb_from_request(&state, q.kb_id.as_deref()) {
        Some(p) => p,
        None => return Json(serde_json::json!({})).into_response(),
    };
    match QualitySnapshotStore::new(&kb).read() {
        Ok(snapshot) => Json(snapshot).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[instrument(skip(state))]
async fn api_quality_issues(
    State(state): State<Arc<HttpState>>,
    Query(query): Query<QualityIssuesQuery>,
) -> impl IntoResponse {
    let kb = match resolve_kb_from_request(&state, query.kb_id.as_deref()) {
        Some(p) => p,
        None => return Json(serde_json::json!({"issues": []})).into_response(),
    };
    let wiki_dir = kb.join("wiki");
    if !wiki_dir.exists() {
        return Json(serde_json::json!({"issues": []})).into_response();
    }
    match analyze_wiki(&wiki_dir) {
        Ok(report) => {
            let mut issues: Vec<serde_json::Value> = report
                .issues
                .iter()
                .map(|i| {
                    serde_json::json!({
                        "severity": i.severity.to_string(),
                        "entry_path": i.entry_path,
                        "message": i.message,
                    })
                })
                .collect();
            if let Some(ref sev) = query.severity {
                let s = sev.to_uppercase();
                issues.retain(|i| {
                    i.get("severity")
                        .and_then(|v| v.as_str())
                        .is_some_and(|x| x.to_uppercase().starts_with(&s))
                });
            }
            issues.truncate(query.limit);
            Json(serde_json::json!({"issues": issues, "total": issues.len()})).into_response()
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

// ── Internal helpers ──

fn count_tree_entries(node: &pdf_core::knowledge::renderer::TreeNode) -> usize {
    if node.is_entry {
        return 1;
    }
    node.children.iter().map(count_tree_entries).sum()
}

fn scan_domains_from_fs(wiki_dir: &PathBuf) -> Vec<String> {
    let mut domains = Vec::new();
    if !wiki_dir.exists() {
        return domains;
    }
    if let Ok(entries) = std::fs::read_dir(wiki_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && let Some(name) = path.file_name()
            {
                let name = name.to_string_lossy().to_string();
                if !name.starts_with('.') {
                    domains.push(name);
                }
            }
        }
    }
    domains.sort();
    domains
}

fn count_entries_by_domain(wiki_dir: &PathBuf) -> std::collections::HashMap<String, usize> {
    let mut counts = std::collections::HashMap::new();
    if !wiki_dir.exists() {
        return counts;
    }
    if let Ok(domain_entries) = std::fs::read_dir(wiki_dir) {
        for de in domain_entries.flatten() {
            let dp = de.path();
            if dp.is_dir()
                && let Some(dn) = dp.file_name()
            {
                let dn = dn.to_string_lossy().to_string();
                if dn.starts_with('.') {
                    continue;
                }
                let mut count = 0usize;
                if let Ok(files) = std::fs::read_dir(&dp) {
                    for f in files.flatten() {
                        let fp = f.path();
                        if fp.extension().is_some_and(|e| e == "md")
                            && fp.file_name().is_some_and(|n| !n.to_string_lossy().starts_with('.'))
                        {
                            count += 1;
                        }
                    }
                }
                counts.insert(dn, count);
            }
        }
    }
    counts
}

fn highlight_snippet(snippet: &str, query: &str) -> String {
    if snippet.is_empty() || query.is_empty() {
        return snippet.to_string();
    }
    let q = query;
    let s = snippet;
    let lower_s = s.to_lowercase();
    let lower_q = q.to_lowercase();

    let mut result = String::with_capacity(s.len() + 50);
    let mut last_end = 0usize;

    let q_chars: Vec<char> = lower_q.chars().collect();
    let s_chars: Vec<char> = lower_s.chars().collect();
    let orig_chars: Vec<char> = s.chars().collect();

    fn esc_char(c: char) -> String {
        match c {
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '&' => "&amp;".into(),
            '"' => "&quot;".into(),
            _ => c.to_string(),
        }
    }

    let mut i = 0usize;
    while i < s_chars.len() {
        if s_chars[i..].len() >= q_chars.len() && s_chars[i..i + q_chars.len()] == q_chars[..] {
            result.push_str(
                &orig_chars[last_end..i].iter().map(|&c| esc_char(c)).collect::<String>(),
            );
            result.push_str("<mark>");
            result.push_str(
                &orig_chars[i..i + q_chars.len()].iter().map(|&c| esc_char(c)).collect::<String>(),
            );
            result.push_str("</mark>");
            i += q_chars.len();
            last_end = i;
        } else {
            i += 1;
        }
    }
    result.push_str(&orig_chars[last_end..].iter().map(|&c| esc_char(c)).collect::<String>());
    result
}

// ── Workspace API ──

async fn api_workspaces_list(State(state): State<Arc<HttpState>>) -> Json<serde_json::Value> {
    let payload = match state.workspace_registry.list() {
        Ok(workspaces) => {
            let active = state.workspace_registry.active_id().ok().flatten();
            serde_json::json!({ "workspaces": workspaces, "active_kb_id": active })
        }
        Err(e) => serde_json::json!({ "error": e.to_string() }),
    };
    Json(payload)
}

#[derive(Debug, Deserialize)]
struct WorkspaceUpsertBody {
    kb_id: String,
    name: String,
    path: String,
    #[serde(default)]
    active: bool,
}

async fn api_workspaces_upsert(
    State(state): State<Arc<HttpState>>,
    Json(body): Json<WorkspaceUpsertBody>,
) -> Json<serde_json::Value> {
    let entry = pdf_core::management::WorkspaceEntry {
        id: body.kb_id,
        name: body.name,
        path: PathBuf::from(body.path),
        active: body.active,
    };
    match state.workspace_registry.upsert(entry) {
        Ok(()) => Json(serde_json::json!({ "ok": true })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string() })),
    }
}

#[derive(Debug, Deserialize)]
struct SetActiveBody {
    kb_id: String,
}

async fn api_workspaces_set_active(
    State(state): State<Arc<HttpState>>,
    Json(body): Json<SetActiveBody>,
) -> Json<serde_json::Value> {
    match state.workspace_registry.set_active(&body.kb_id) {
        Ok(()) => Json(serde_json::json!({ "ok": true, "active_kb_id": body.kb_id })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string() })),
    }
}

// ── Index metadata ──

fn index_meta_path(kb: &std::path::Path) -> PathBuf {
    kb.join(".rsut_index").join("index_meta.json")
}

fn write_index_meta(kb: &std::path::Path, stats: &serde_json::Value) -> std::io::Result<()> {
    let path = index_meta_path(kb);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let meta = serde_json::json!({
        "rebuilt_at": Utc::now().to_rfc3339(),
        "stats": stats,
    });
    std::fs::write(path, serde_json::to_string_pretty(&meta).unwrap_or_default())
}

fn read_index_meta(kb: &std::path::Path) -> Option<serde_json::Value> {
    let raw = std::fs::read_to_string(index_meta_path(kb)).ok()?;
    serde_json::from_str(&raw).ok()
}

#[instrument(skip(state))]
async fn api_index_status(
    State(state): State<Arc<HttpState>>,
    Query(q): Query<KbQuery>,
) -> impl IntoResponse {
    let kb = match resolve_kb_from_request(&state, q.kb_id.as_deref()) {
        Some(p) => p,
        None => {
            return Json(serde_json::json!({"error": "No knowledge base configured"}))
                .into_response();
        }
    };
    match read_index_meta(&kb) {
        Some(meta) => Json(meta).into_response(),
        None => Json(serde_json::json!({ "rebuilt_at": null, "stats": null })).into_response(),
    }
}

// ── Compile SSE ──

#[instrument(skip(state))]
async fn api_compile_events(
    State(state): State<Arc<HttpState>>,
    Query(q): Query<KbQuery>,
) -> impl IntoResponse {
    let kb = match resolve_kb_from_request(&state, q.kb_id.as_deref()) {
        Some(p) => p,
        None => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "No KB"})),
            )
                .into_response();
        }
    };

    let kb_path = kb.clone();
    let event_stream = stream! {
        let mut last_payload: Option<String> = None;
        let mut ticks: u64 = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            ticks += 1;
            if ticks > 3600 {
                break;
            }
            let Ok(value) = build_compile_status_json(&kb_path) else {
                continue;
            };
            let Ok(serialized) = serde_json::to_string(&value) else {
                continue;
            };
            if last_payload.as_deref() != Some(serialized.as_str()) {
                last_payload = Some(serialized.clone());
                yield Ok::<Event, std::convert::Infallible>(
                    Event::default().event("compile-status").data(serialized),
                );
            }
        }
    };

    Sse::new(event_stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}

// ── Share links ──

#[derive(Debug, Serialize, Deserialize)]
struct ShareRecord {
    kb_id: String,
    path: String,
    expires_at: String,
    read_only: bool,
}

fn shares_dir(kb: &std::path::Path) -> PathBuf {
    kb.join(".rsut_index").join("shares")
}

fn share_path(kb: &std::path::Path, token: &str) -> PathBuf {
    shares_dir(kb).join(format!("{token}.json"))
}

fn load_share(kb: &std::path::Path, token: &str) -> Option<ShareRecord> {
    let raw = std::fs::read_to_string(share_path(kb, token)).ok()?;
    let record: ShareRecord = serde_json::from_str(&raw).ok()?;
    if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(&record.expires_at)
        && exp < Utc::now()
    {
        let _ = std::fs::remove_file(share_path(kb, token));
        return None;
    }
    Some(record)
}

#[derive(Debug, Deserialize)]
struct ShareCreateBody {
    kb_id: String,
    path: String,
    #[serde(default = "default_share_ttl_hours")]
    ttl_hours: u64,
}

fn default_share_ttl_hours() -> u64 {
    168
}

async fn api_shares_create(
    State(state): State<Arc<HttpState>>,
    Json(body): Json<ShareCreateBody>,
) -> impl IntoResponse {
    let kb = match state.workspace_registry.path_for_id(&body.kb_id) {
        Ok(p) => p,
        Err(e) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    let token = uuid::Uuid::new_v4().to_string();
    let expires_at = (Utc::now() + chrono::Duration::hours(body.ttl_hours as i64)).to_rfc3339();
    let record = ShareRecord {
        kb_id: body.kb_id.clone(),
        path: normalize_wiki_entry_path(&body.path),
        expires_at,
        read_only: true,
    };

    let dir = shares_dir(&kb);
    if std::fs::create_dir_all(&dir).is_err() {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to create shares directory"})),
        )
            .into_response();
    }

    if serde_json::to_string_pretty(&record)
        .ok()
        .and_then(|s| std::fs::write(share_path(&kb, &token), s).ok())
        .is_none()
    {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to write share record"})),
        )
            .into_response();
    }

    Json(serde_json::json!({
        "token": token,
        "url_path": format!("/share/{token}"),
    }))
    .into_response()
}

fn resolve_share_record(
    registry: &WorkspaceRegistry,
    token: &str,
) -> Option<(PathBuf, ShareRecord)> {
    if let Ok(workspaces) = registry.list() {
        for ws in workspaces {
            if let Some(record) = load_share(&ws.path, token) {
                return Some((ws.path, record));
            }
        }
    }
    None
}

async fn api_share_wiki_entry(
    State(state): State<Arc<HttpState>>,
    Path((token, path)): Path<(String, String)>,
) -> impl IntoResponse {
    let Some((kb, record)) = resolve_share_record(&state.workspace_registry, &token) else {
        return (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Share link invalid or expired"})),
        )
            .into_response();
    };

    let entry_path = normalize_wiki_entry_path(&path);
    if entry_path != record.path {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Path not allowed for this share token"})),
        )
            .into_response();
    }

    let wiki_dir = kb.join("wiki");
    let renderer = WikiRenderer::new(&wiki_dir);
    match renderer.render_entry(&entry_path) {
        Ok(entry) => Json(serde_json::json!({"entry": entry, "read_only": true})).into_response(),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})).into_response(),
    }
}

/// JSON-RPC MCP bridge for remote clients (`pdf-cli --remote`, Cursor over HTTP).
#[instrument(skip(state, request))]
async fn api_mcp_jsonrpc(
    State(state): State<Arc<HttpState>>,
    Json(request): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let Some(pipeline) = state.pipeline.clone() else {
        return Json(JsonRpcResponse::error(
            request.id,
            JsonRpcError::internal_error("HTTP MCP requires PDF pipeline (server misconfigured)"),
        ));
    };

    let tool_ctx = ToolContext::new_with_upload_store(
        pipeline,
        state.upload_store.clone(),
        Arc::clone(&state.workspace_registry),
        Arc::clone(&state.index_cache),
    );
    let stats = Arc::new(ToolStats::new());

    match handle_request(&tool_ctx, &stats, request).await {
        Some(response) => Json(response),
        None => Json(JsonRpcResponse::success(None, serde_json::json!({}))),
    }
}

// ── Version & Update API handlers ──

/// Return the current version information embedded at compile time.
#[instrument(skip(state))]
async fn api_version(State(state): State<Arc<HttpState>>) -> Json<serde_json::Value> {
    let v = &state.version_info;
    let version_json = serde_json::json!({
        "version": v.display,
        "semver": v.semver,
        "major": v.major,
        "minor": v.minor,
        "build": v.build,
        "patch": v.patch,
        "deployment_mode": serde_json::to_value(&v.deployment_mode).unwrap_or_default(),
    });
    Json(version_json)
}

/// Check for updates from GitHub Releases.
/// Uses server-side cache (1h TTL) to avoid excessive GitHub API calls.
#[instrument(skip(state))]
async fn api_update_check(
    State(state): State<Arc<HttpState>>,
) -> impl IntoResponse {
    // Return cached result if still valid
    if let Some(cached) = state.update_cache.get() {
        return Json(serde_json::to_value(&cached).unwrap_or_default()).into_response();
    }

    // Perform the check (runs in spawn_blocking since ureq is sync)
    let client = Arc::clone(&state.github_client);
    let version = state.version_info.clone();
    let cache = Arc::clone(&state.update_cache);

    let result = tokio::task::spawn_blocking(move || {
        match github_check(&client, &version) {
            Ok(result) => {
                cache.set(result.clone());
                Ok(result)
            }
            Err(e) => Err(e),
        }
    })
    .await
    .map_err(|e| format!("Update check task panicked: {e}"));

    match result {
        Ok(Ok(check_result)) => {
            Json(serde_json::to_value(&check_result).unwrap_or_default()).into_response()
        }
        Ok(Err(msg)) | Err(msg) => (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": msg})),
        )
            .into_response(),
    }
}

/// Prepare an update: download the latest release asset.
/// This is a potentially long-running operation.
#[instrument(skip(state))]
async fn api_update_prepare(
    State(state): State<Arc<HttpState>>,
) -> impl IntoResponse {
    let client = Arc::clone(&state.github_client);
    let version = state.version_info.clone();

    let result = tokio::task::spawn_blocking(move || {
        crate::version::github::prepare_update(&client, &version, |downloaded, total| {
            let pct = if total > 0 {
                (downloaded as f64 / total as f64 * 100.0) as u32
            } else {
                0
            };
            tracing::info!(
                downloaded_bytes = downloaded,
                total_bytes = total,
                pct,
                "Update download progress"
            );
        })
    })
    .await
    .map_err(|e| format!("Update prepare task panicked: {e}"));

    match result {
        Ok(prepare_result) => {
            Json(serde_json::to_value(&prepare_result).unwrap_or_default()).into_response()
        }
        Err(msg) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": msg})),
        )
            .into_response(),
    }
}
