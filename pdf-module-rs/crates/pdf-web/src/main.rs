//! # pdf-web (deprecated)
//!
//! Legacy management API sidecar. **Use `pdf-mcp` instead**, which serves the same
//! management endpoints plus the embedded Vue3 wiki UI (`pdf-web-ui`).
//!
//! ## Endpoints
//!
//! - `GET /api/health` → JSON health report
//! - `GET /api/config` → Runtime configuration
//! - `POST /api/config` → Set configuration key
//! - `DELETE /api/config/:key` → Remove configuration key
//! - `GET /api/compile/status` → Compile status
//! - `POST /api/compile` → Trigger incremental compile
//! - `POST /api/index/rebuild` → Rebuild indexes

#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(clippy::all)]

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use clap::Parser;
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};

use pdf_core::knowledge::rebuild_all;
use pdf_core::management::{ConfigManager, HealthReporter};

#[derive(Clone)]
struct AppState {
    kb_path: PathBuf,
}

#[derive(Parser)]
#[command(name = "pdf-web", version, about = "Lightweight web panel for rsut-pdf-mcp")]
struct Cli {
    /// Path to the knowledge base directory
    #[arg(long, env = "KNOWLEDGE_BASE", default_value = ".")]
    knowledge_base: PathBuf,

    /// Port to listen on
    #[arg(short, long, default_value = "8070")]
    port: u16,

    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tracing::warn!("pdf-web is deprecated; use pdf-mcp for unified HTTP + wiki UI (pdf-web-ui)");

    let cli = Cli::parse();
    let state = AppState { kb_path: cli.knowledge_base };

    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    let app = Router::new()
        .route("/api/health", get(api_health))
        .route("/api/config", get(api_config_get).post(api_config_set))
        .route("/api/config/{key}", delete(api_config_remove))
        .route("/api/compile/status", get(api_compile_status))
        .route("/api/compile", post(api_compile_trigger))
        .route("/api/index/rebuild", post(api_index_rebuild))
        .layer(cors)
        .with_state(Arc::new(state));

    let addr: SocketAddr = format!("{}:{}", cli.host, cli.port).parse()?;
    tracing::info!("Web panel listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
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
    tracing::info!("Shutting down...");
}

async fn api_health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let reporter = HealthReporter::new(&state.kb_path);
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
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()})))
                .into_response()
        }
    }
}

async fn api_config_get(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut cm = ConfigManager::new(&state.kb_path);
    if let Err(e) = cm.load() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }
    Json(serde_json::json!({
        "config": cm.all(),
        "total_keys": cm.all().len(),
    }))
    .into_response()
}

#[derive(Deserialize)]
struct SetConfigBody {
    key: String,
    value: String,
}

async fn api_config_set(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetConfigBody>,
) -> impl IntoResponse {
    let mut cm = ConfigManager::new(&state.kb_path);
    if let Err(e) = cm.load() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }
    match cm.set(&body.key, &body.value) {
        Ok(()) => Json(serde_json::json!({"status": "ok", "key": body.key})).into_response(),
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()})))
                .into_response()
        }
    }
}

async fn api_config_remove(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    let mut cm = ConfigManager::new(&state.kb_path);
    if let Err(e) = cm.load() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }
    match cm.remove(&key) {
        Ok(()) => Json(serde_json::json!({"status": "ok", "removed": key})).into_response(),
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()})))
                .into_response()
        }
    }
}

async fn api_compile_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match pdf_core::management::build_compile_status_json(&state.kb_path) {
        Ok(v) => Json(v).into_response(),
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()})))
                .into_response()
        }
    }
}

async fn api_compile_trigger(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let config = pdf_core::ServerConfig::from_env().unwrap_or_default();
    let pipeline = match pdf_core::McpPdfPipeline::new(&config) {
        Ok(p) => Arc::new(p),
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Pipeline init failed: {}", e)})),
            )
                .into_response()
        }
    };
    let engine = match pdf_core::KnowledgeEngine::new(pipeline, &state.kb_path) {
        Ok(e) => e,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Engine init failed: {}", e)})),
            )
                .into_response()
        }
    };
    let job_store = pdf_core::management::CompileJobStore::new(&state.kb_path);
    match pdf_core::knowledge::run_incremental_extract(&engine, &job_store).await {
        Ok((job_id, result)) => Json(serde_json::json!({
            "job_id": job_id,
            "pipeline_status": "awaiting_agent",
            "incremental_result": result,
        }))
        .into_response(),
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()})))
                .into_response()
        }
    }
}

async fn api_index_rebuild(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if !state.kb_path.join("wiki").exists() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Wiki directory not found"})),
        )
            .into_response();
    }

    match rebuild_all(&state.kb_path) {
        Ok(stats) => Json(serde_json::json!({
            "status": "success",
            "fulltext_entries_indexed": stats.fulltext_entries_indexed,
            "graph_nodes": stats.graph_nodes,
            "graph_edges": stats.graph_edges,
        }))
        .into_response(),
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()})))
                .into_response()
        }
    }
}
