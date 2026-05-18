//! # Commands
//!
//! Unified command dispatch for local and remote modes.
//! Every function takes `mode: Mode` and dispatches accordingly.

pub mod compile;
pub mod platform;
pub mod query;

use crate::config::CliConfig;
use crate::output::{OutputFormat, print_info, print_result};
use crate::remote::{RemoteClient, RemoteConfig};
use anyhow::Result;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Operating mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Local,
    Remote,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => write!(f, "local"),
            Self::Remote => write!(f, "remote"),
        }
    }
}

impl Mode {
    /// Resolve mode from config string. Accepts "local" (default) or "remote".
    pub fn from_config(config: &CliConfig) -> Self {
        match config.mode.as_str() {
            "remote" => Self::Remote,
            _ => Self::Local,
        }
    }

    /// Return mode as static string for config persistence.
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote => "remote",
        }
    }
}

/// Parse a string into Mode. Accepts "local"/"--local" and "remote"/"--remote".
impl std::str::FromStr for Mode {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "local" | "--local" => Ok(Self::Local),
            "remote" | "--remote" => Ok(Self::Remote),
            _ => Err(anyhow::anyhow!("Invalid mode: {}. Use 'local' or 'remote'", s)),
        }
    }
}

/// Common result from any command — a label and a JSON value.
pub struct CmdResult {
    pub label: String,
    pub value: Value,
}

impl CmdResult {
    pub fn new(label: impl Into<String>, value: Value) -> Self {
        Self { label: label.into(), value }
    }

    /// Print this result in the requested format.
    pub fn print(&self, format: OutputFormat) {
        print_result(format, &self.label, &self.value);
    }
}

/// Build a remote client from CLI config, or error if not configured.
pub fn build_remote_client(config: &CliConfig) -> Result<RemoteClient> {
    let server = config.server.as_deref().ok_or_else(|| {
        anyhow::anyhow!(
            "No remote server configured. Use 'config set server <URL>' or --server flag."
        )
    })?;

    let mut cfg = RemoteConfig { server: server.to_string(), ..Default::default() };

    if let Some(ref token) = config.token {
        cfg.token = Some(token.clone());
    }

    // Check env var override
    if let Ok(env_token) = std::env::var("RSUT_PDF_TOKEN") {
        cfg.token = Some(env_token);
    }

    RemoteClient::new(cfg)
}

/// Get the effective knowledge base path for local mode.
pub fn resolve_kb_path(config: &CliConfig, cli_kb: Option<&Path>) -> PathBuf {
    cli_kb
        .map(|p| p.to_path_buf())
        .or_else(|| config.knowledge_base.clone())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Get the effective knowledge base path for remote mode.
pub fn resolve_remote_kb(config: &CliConfig, cli_kb: Option<&str>) -> String {
    cli_kb
        .map(|s| s.to_string())
        .or_else(|| config.remote_knowledge_base.clone())
        .unwrap_or_else(|| ".".to_string())
}

// ── Shared: proxy for compile_uploaded_pdf ──

/// Remote: upload a PDF, then call compile_uploaded_pdf MCP tool.
pub async fn remote_compile_uploaded(
    client: &RemoteClient,
    pdf_path: &Path,
    domain: Option<&str>,
    kb_path: &str,
) -> Result<CmdResult> {
    // 1. Upload
    print_info(format!("Uploading {}...", pdf_path.display()));
    let upload = client.upload_pdf(pdf_path).await?;

    let file_id = upload["file_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Upload response missing file_id: {}", upload))?;

    print_info(format!("Uploaded: file_id={}", file_id));

    // 2. Call compile_uploaded_pdf MCP tool
    let mut args = serde_json::json!({
        "file_id": file_id,
        "knowledge_base": kb_path,
    });
    if let Some(d) = domain {
        args["domain"] = serde_json::Value::String(d.to_string());
    }

    let result = client.call_tool("compile_uploaded_pdf", args).await?;

    Ok(CmdResult::new("Compile Result", result))
}
