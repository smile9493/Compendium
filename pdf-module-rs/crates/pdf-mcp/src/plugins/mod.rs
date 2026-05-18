//! Extraction plugin loading from `extraction.plugins.toml`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use pdf_core::error::PdfResult;
use pdf_core::extraction::RemoteExtractionConfig;
use pdf_core::{McpPdfPipeline, ServerConfig};
use serde::Deserialize;
use tracing::warn;
use vlm_visual_gateway::MetricsCollector;

#[derive(Debug, Deserialize)]
struct PluginsFile {
    #[serde(default, rename = "plugin")]
    plugins: Vec<PluginEntry>,
}

#[derive(Debug, Deserialize)]
struct PluginEntry {
    id: String,
    #[serde(rename = "type")]
    plugin_type: String,
    endpoint: Option<String>,
    #[serde(default)]
    priority: i32,
    #[serde(default = "default_timeout_ms")]
    timeout_ms: u64,
    #[serde(default)]
    capabilities: Vec<String>,
}

fn default_timeout_ms() -> u64 {
    30_000
}

/// Load remote extraction plugins and build pipeline with extended router.
pub fn build_pipeline_with_plugins(
    config: &ServerConfig,
    metrics: Arc<MetricsCollector>,
    plugins_path: Option<&Path>,
) -> PdfResult<Arc<McpPdfPipeline>> {
    let remote_configs = load_remote_configs(plugins_path)?;
    let mut pipeline = McpPdfPipeline::new_with_metrics(config, metrics)?;
    if !remote_configs.is_empty() {
        pipeline.reconfigure_extraction_router(remote_configs)?;
    }
    Ok(Arc::new(pipeline))
}

pub fn load_remote_configs(plugins_path: Option<&Path>) -> PdfResult<Vec<RemoteExtractionConfig>> {
    let path = plugins_path.map(Path::to_path_buf).unwrap_or_else(|| {
        std::env::var("EXTRACTION_PLUGINS_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("extraction.plugins.toml"))
    });

    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw = std::fs::read_to_string(&path).map_err(|e| {
        pdf_core::error::PdfModuleError::Storage(format!("read {}: {e}", path.display()))
    })?;
    let file: PluginsFile = toml::from_str(&raw).map_err(|e| {
        pdf_core::error::PdfModuleError::Storage(format!("parse {}: {e}", path.display()))
    })?;

    let mut configs = Vec::new();
    for p in file.plugins {
        if p.plugin_type != "remote" {
            continue;
        }
        let Some(endpoint) = p.endpoint else {
            warn!(id = %p.id, "remote plugin missing endpoint");
            continue;
        };
        let _ = p.capabilities;
        configs.push(RemoteExtractionConfig {
            id: p.id,
            endpoint,
            timeout: Duration::from_millis(p.timeout_ms),
            priority: p.priority,
        });
    }
    Ok(configs)
}

pub fn default_plugins_path() -> PathBuf {
    PathBuf::from("extraction.plugins.toml")
}
