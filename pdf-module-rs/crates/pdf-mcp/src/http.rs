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
//! | GET | `/api/config` | Runtime config |
//! | POST | `/api/config` | Set config key |
//! | DELETE | `/api/config/{key}` | Remove config key |
//! | GET | `/api/compile/status` | Compile status |
//! | POST | `/api/index/rebuild` | Rebuild indexes |
//!
//! ### SPA
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | GET | `/` | Redirect to SPA |
//! | GET | `/app/{*path}` | Vue3 SPA (with history fallback) |

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Json, Redirect};
use axum::routing::{delete, get, post};
use axum::Router;
use serde::Deserialize;
use tokio::sync::oneshot;
use tracing::{info, instrument};

use pdf_core::knowledge::index::{FulltextIndex, GraphIndex, MetadataStore};
use pdf_core::knowledge::index::fulltext::SearchHit;
use pdf_core::knowledge::renderer::WikiRenderer;
use pdf_core::management::{ConfigManager, HealthReporter};

use crate::embed::Assets;
use crate::metrics::{self, HttpMetrics, MetricsLayer};
use crate::upload::UploadStore;

#[derive(Clone)]
pub struct HttpState {
    pub kb_path: Option<PathBuf>,
    pub upload_store: Option<Arc<UploadStore>>,
    pub http_metrics: Option<Arc<HttpMetrics>>,
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

    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("HTTP server error: {}", e))
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
        .route("/api/compile/status", get(api_compile_status))
        .route("/api/index/rebuild", post(api_index_rebuild))
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
async fn serve_spa(
    uri: axum::extract::OriginalUri,
) -> impl IntoResponse {
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
        return (
            [("Content-Type", mime)],
            content.data.into_owned(),
        )
            .into_response();
    }

    // SPA fallback: serve index.html for all unmatched non-API paths
    if let Some(content) = Assets::get("index.html") {
        return (
            [("Content-Type", "text/html; charset=utf-8")],
            content.data.into_owned(),
        )
            .into_response();
    }

    (
        axum::http::StatusCode::NOT_FOUND,
        "SPA not found (pdf-web-ui not built?)",
    )
        .into_response()
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
async fn api_wiki_tree(State(state): State<Arc<HttpState>>) -> Json<serde_json::Value> {
    let kb = match &state.kb_path {
        Some(p) => p.clone(),
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
) -> Json<serde_json::Value> {
    let kb = match &state.kb_path {
        Some(p) => p.clone(),
        None => return Json(serde_json::json!({"error": "No knowledge base configured"})),
    };

    let wiki_dir = kb.join("wiki");
    let renderer = WikiRenderer::new(&wiki_dir);

    match renderer.render_entry(&path) {
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
}

fn default_limit() -> usize {
    20
}

#[instrument(skip(state))]
async fn api_wiki_search(
    State(state): State<Arc<HttpState>>,
    Query(query): Query<SearchQuery>,
) -> Json<serde_json::Value> {
    let kb = match &state.kb_path {
        Some(p) => p.clone(),
        None => return Json(serde_json::json!({"results": [], "total": 0})),
    };

    let wiki_dir = kb.join("wiki");
    if !wiki_dir.exists() {
        return Json(serde_json::json!({"results": [], "total": 0}));
    }

    let search_query = query.q.clone();
    let search_limit = query.limit;
    let wd = wiki_dir.clone();

    let hits = fs_fallback_search(&wd, &search_query, search_limit);

    let mut domain_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    let results: Vec<serde_json::Value> = hits
        .into_iter()
        .filter(|h| {
            if let Some(ref domain) = query.domain {
                h.domain == *domain
            } else {
                true
            }
        })
        .map(|h| {
            *domain_counts.entry(h.domain.clone()).or_insert(0) += 1;
            serde_json::json!({
                "path": h.path,
                "title": h.title,
                "domain": h.domain,
                "score": h.score,
                "snippet": highlight_snippet(&h.snippet, &query.q),
                "match_count": h.match_count,
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
    }))
}

#[instrument(skip(state))]
async fn api_wiki_graph(
    State(state): State<Arc<HttpState>>,
    Path(path): Path<String>,
) -> Json<serde_json::Value> {
    let kb = match &state.kb_path {
        Some(p) => p.clone(),
        None => return Json(serde_json::json!({"error": "No knowledge base configured"})),
    };

    let wiki_dir = kb.join("wiki");
    if !wiki_dir.exists() {
        return Json(serde_json::json!({"error": "Wiki directory not found"}));
    }

    let (graph, _) = match GraphIndex::load_from_disk_or_rebuild(&kb, &wiki_dir) {
        Ok(g) => g,
        Err(e) => {
            return Json(serde_json::json!({"error": format!("Graph load failed: {}", e)}));
        }
    };

    let mermaid = graph.export_concept_map(&path, 2);
    Json(serde_json::json!({"mermaid": mermaid, "entry": path}))
}

#[instrument(skip(state))]
async fn api_wiki_stats(State(state): State<Arc<HttpState>>) -> Json<serde_json::Value> {
    let kb = match &state.kb_path {
        Some(p) => p.clone(),
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
async fn api_wiki_domains(State(state): State<Arc<HttpState>>) -> Json<serde_json::Value> {
    let kb = match &state.kb_path {
        Some(p) => p.clone(),
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

#[instrument(skip(state))]
async fn api_health(State(state): State<Arc<HttpState>>) -> impl IntoResponse {
    let kb = match &state.kb_path {
        Some(p) => p.clone(),
        None => return Json(serde_json::json!({"error": "No knowledge base configured"})).into_response(),
    };

    let reporter = HealthReporter::new(&kb);
    match reporter.report() {
        Ok(report) => Json(serde_json::json!({
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
        }))
        .into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[instrument(skip(state))]
async fn api_config_get(State(state): State<Arc<HttpState>>) -> impl IntoResponse {
    let kb = match &state.kb_path {
        Some(p) => p.clone(),
        None => return (axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "No KB"}))).into_response(),
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
    Json(body): Json<SetConfigBody>,
) -> impl IntoResponse {
    let kb = match &state.kb_path {
        Some(p) => p.clone(),
        None => return (axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "No KB"}))).into_response(),
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
) -> impl IntoResponse {
    let kb = match &state.kb_path {
        Some(p) => p.clone(),
        None => return (axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "No KB"}))).into_response(),
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

#[instrument(skip(state))]
async fn api_compile_status(State(state): State<Arc<HttpState>>) -> impl IntoResponse {
    let kb = match &state.kb_path {
        Some(p) => p.clone(),
        None => return (axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "No KB"}))).into_response(),
    };

    let status_path = kb.join(".rsut_index").join("compile_status.json");
    if !status_path.exists() {
        return Json(serde_json::json!({
            "running": false, "last_started": null, "last_finished": null,
            "last_duration_ms": null, "last_outcome": null,
            "message": "No compile performed yet.", "history": [],
        }))
        .into_response();
    }
    match std::fs::read_to_string(&status_path) {
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(v) => Json(v).into_response(),
            Err(e) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Parse error: {}", e)})),
            )
                .into_response(),
        },
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[instrument(skip(state))]
async fn api_index_rebuild(State(state): State<Arc<HttpState>>) -> impl IntoResponse {
    let kb = match &state.kb_path {
        Some(p) => p.clone(),
        None => return (axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "No KB"}))).into_response(),
    };

    let wiki_dir = kb.join("wiki");
    if !wiki_dir.exists() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Wiki directory not found"})),
        )
            .into_response();
    }

    let ft_idx = match FulltextIndex::open_or_create(&kb) {
        Ok(idx) => idx,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };
    let ft_count = match ft_idx.rebuild(&wiki_dir) {
        Ok(c) => c,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let mut g_idx = GraphIndex::new();
    let g_count = match g_idx.rebuild(&wiki_dir) {
        Ok(c) => c,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    Json(serde_json::json!({
        "status": "success",
        "fulltext_entries_indexed": ft_count,
        "graph_nodes": g_count,
        "graph_edges": g_idx.edge_count(),
    }))
    .into_response()
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
            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    let name = name.to_string_lossy().to_string();
                    if !name.starts_with('.') {
                        domains.push(name);
                    }
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
            if dp.is_dir() {
                if let Some(dn) = dp.file_name() {
                    let dn = dn.to_string_lossy().to_string();
                    if dn.starts_with('.') {
                        continue;
                    }
                    let mut count = 0usize;
                    if let Ok(files) = std::fs::read_dir(&dp) {
                        for f in files.flatten() {
                            let fp = f.path();
                            if fp.extension().map_or(false, |e| e == "md")
                                && fp.file_name().map_or(false, |n| !n.to_string_lossy().starts_with('.'))
                            {
                                count += 1;
                            }
                        }
                    }
                    counts.insert(dn, count);
                }
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
        if s_chars[i..].len() >= q_chars.len()
            && s_chars[i..i + q_chars.len()] == q_chars[..]
        {
            result.push_str(&orig_chars[last_end..i].iter().map(|&c| esc_char(c)).collect::<String>());
            result.push_str("<mark>");
            result.push_str(&orig_chars[i..i + q_chars.len()].iter().map(|&c| esc_char(c)).collect::<String>());
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

fn try_tantivy_search(
    kb_path: &PathBuf,
    wiki_dir: &PathBuf,
    query: &str,
) -> Result<Vec<SearchHit>, String> {
    let index = FulltextIndex::open_or_create(kb_path).map_err(|e| e.to_string())?;
    let needs_rebuild = index.is_empty().unwrap_or(true);
    if needs_rebuild {
        index.rebuild(wiki_dir).map_err(|e| e.to_string())?;
    }
    index.search(query, 30).map_err(|e| e.to_string())
}

struct FsSearchHit {
    path: String,
    title: String,
    domain: String,
    score: f64,
    snippet: String,
    match_count: usize,
}

fn fs_fallback_search(wiki_dir: &PathBuf, query: &str, limit: usize) -> Vec<FsSearchHit> {
    let mut results: Vec<FsSearchHit> = Vec::new();
    let lower_q = query.to_lowercase();

    if !wiki_dir.exists() {
        return results;
    }

    if let Ok(domain_entries) = std::fs::read_dir(wiki_dir) {
        for de in domain_entries.flatten() {
            let dp = de.path();
            if !dp.is_dir() {
                continue;
            }
            let domain = dp.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if domain.starts_with('.') {
                continue;
            }

            if let Ok(files) = std::fs::read_dir(&dp) {
                for f in files.flatten() {
                    let fp = f.path();
                    if !fp.extension().map_or(false, |e| e == "md") {
                        continue;
                    }
                    let rel_path = format!("{}/{}", domain, fp.file_name().unwrap().to_string_lossy());
                    let title = fp.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

                    if let Ok(content) = std::fs::read_to_string(&fp) {
                        let lower_content = content.to_lowercase();
                        let count = lower_content.matches(&lower_q).count();
                        if count == 0 {
                            continue;
                        }

                        let score = count as f64 / (content.len().max(1) as f64).sqrt() * 100.0;
                        let (snippet, match_count) = extract_snippets_multi(&content, &lower_q, 80, 3);
                        results.push(FsSearchHit {
                            path: rel_path,
                            title,
                            domain: domain.clone(),
                            score,
                            snippet,
                            match_count,
                        });
                    }
                }
            }
        }
    }

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    results
}

fn extract_snippet_fs(content: &str, lower_q: &str, window: usize) -> String {
    let lower_content = content.to_lowercase();
    if let Some(byte_pos) = lower_content.find(lower_q) {
        let pre = if byte_pos > 0 { "..." } else { "" };
        let post = if byte_pos + lower_q.len() + window < content.len() {
            "..."
        } else {
            ""
        };

        let byte_start = byte_pos.saturating_sub(window / 2);
        let begin = floor_char_boundary(content, byte_start);

        let byte_end = (byte_pos + lower_q.len() + window).min(content.len());
        let end = ceil_char_boundary(content, byte_end);

        format!("{}{}{}", pre, &content[begin..end], post)
    } else {
        let s: String = content.chars().take(window).collect();
        format!("{}...", s)
    }
}

fn extract_snippets_multi(content: &str, lower_q: &str, window: usize, max_snippets: usize) -> (String, usize) {
    let lower_content = content.to_lowercase();
    let q_len = lower_q.len();
    let mut positions: Vec<usize> = Vec::new();
    let mut search_start = 0usize;
    
    while let Some(pos) = lower_content[search_start..].find(lower_q) {
        let abs_pos = search_start + pos;
        positions.push(abs_pos);
        search_start = abs_pos + q_len;
        if positions.len() >= max_snippets * 3 {
            break;
        }
    }
    
    let match_count = positions.len();
    if match_count == 0 {
        let s: String = content.chars().take(window).collect();
        return (format!("{}...", s), 0);
    }
    
    let mut snippets: Vec<String> = Vec::new();
    let mut last_end = 0usize;
    
    for &pos in positions.iter() {
        if snippets.len() >= max_snippets {
            break;
        }
        
        let byte_start = pos.saturating_sub(window / 2);
        let begin = floor_char_boundary(content, byte_start);
        
        if begin < last_end + 10 {
            continue;
        }
        
        let byte_end = (pos + q_len + window / 2).min(content.len());
        let end = ceil_char_boundary(content, byte_end);
        
        let snippet = &content[begin..end];
        let pre = if begin > 0 { "..." } else { "" };
        let post = if end < content.len() { "..." } else { "" };
        
        snippets.push(format!("{}{}{}", pre, snippet, post));
        last_end = end;
    }
    
    if snippets.is_empty() {
        let pos = positions[0];
        let byte_start = pos.saturating_sub(window / 2);
        let begin = floor_char_boundary(content, byte_start);
        let byte_end = (pos + q_len + window / 2).min(content.len());
        let end = ceil_char_boundary(content, byte_end);
        let snippet = &content[begin..end];
        let pre = if begin > 0 { "..." } else { "" };
        let post = if end < content.len() { "..." } else { "" };
        snippets.push(format!("{}{}{}", pre, snippet, post));
    }
    
    (snippets.join(" ··· "), match_count)
}

fn floor_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos.min(s.len());
    while p > 0 && !s.is_char_boundary(p) {
        p -= 1;
    }
    p
}

fn ceil_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos.min(s.len());
    while p < s.len() && !s.is_char_boundary(p) {
        p += 1;
    }
    p
}
