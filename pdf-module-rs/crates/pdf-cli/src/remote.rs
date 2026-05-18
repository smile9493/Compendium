//! # Remote Mode Client
//!
//! HTTP client for interacting with a remote `rsut-pdf-mcp` server.
//! Supports multipart file upload, JSON-RPC tool calls, and wiki API queries.
//!
//! Some client methods are public API surface for future commands.
#![allow(dead_code)]

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

/// Remote client configuration
#[derive(Debug, Clone)]
pub struct RemoteConfig {
    /// Base URL of the remote server (e.g. "http://192.168.2.50:9090")
    pub server: String,

    /// Auth token for Bearer authentication
    pub token: Option<String>,

    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self { server: "http://127.0.0.1:9090".to_string(), token: None, timeout_secs: 300 }
    }
}

/// A reusable HTTP client for remote server communication.
#[derive(Debug, Clone)]
pub struct RemoteClient {
    inner: reqwest::Client,
    cfg: RemoteConfig,
}

impl RemoteClient {
    /// Create a new remote client from configuration.
    pub fn new(cfg: RemoteConfig) -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        if let Some(ref token) = cfg.token {
            let auth_value = format!("Bearer {}", token);
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&auth_value)
                    .context("Invalid token format")?,
            );
        }

        let inner = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(cfg.timeout_secs))
            .pool_max_idle_per_host(4)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { inner, cfg })
    }

    // ── Upload API ──

    /// Upload a PDF file to the remote server via `POST /api/upload`.
    /// Uses streaming upload with a progress bar.
    /// Returns `{ "file_id": "uuid", "filename": "..." }`.
    pub async fn upload_pdf(&self, pdf_path: &Path) -> Result<Value> {
        let url = format!("{}/api/upload", self.cfg.server);

        let file_name =
            pdf_path.file_name().and_then(|n| n.to_str()).unwrap_or("document.pdf").to_string();

        let metadata = tokio::fs::metadata(pdf_path)
            .await
            .with_context(|| format!("Failed to read metadata: {}", pdf_path.display()))?;
        let file_size = metadata.len();

        let mime = mime_guess::from_path(pdf_path).first_or_octet_stream();

        // Open file for streaming
        let file = tokio::fs::File::open(pdf_path)
            .await
            .with_context(|| format!("Failed to open: {}", pdf_path.display()))?;

        // Set up progress bar
        let pb = ProgressBar::new(file_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{msg}\n[{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
                )
                .context("Invalid progress bar template")
                .map_err(|e| anyhow::anyhow!("Progress bar template error: {}", e))?
                .progress_chars("##-"),
        );
        pb.set_message(format!("Uploading {}", file_name));

        // Wrap file reader with progress
        let progress_reader = ProgressReader { inner: file, progress: pb.clone() };

        // Stream the file as multipart
        let stream = tokio_util::io::ReaderStream::new(progress_reader);
        let stream_body = reqwest::Body::wrap_stream(stream);

        let part = reqwest::multipart::Part::stream_with_length(stream_body, file_size)
            .file_name(file_name.clone())
            .mime_str(mime.as_ref())
            .context("Invalid MIME type")?;

        let form = reqwest::multipart::Form::new().part("file", part);

        let resp =
            self.inner.post(&url).multipart(form).send().await.context("Upload request failed")?;

        pb.finish_and_clear();

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Upload failed ({}): {}", status, body);
        }

        let result: Value = resp.json().await.context("Failed to parse upload response")?;

        Ok(result)
    }

    // ── MCP JSON-RPC ──

    /// Call an MCP tool via `POST /mcp` with JSON-RPC payload.
    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        let url = format!("{}/mcp", self.cfg.server);

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        });

        let resp = self
            .inner
            .post(&url)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("MCP call '{}' failed", tool_name))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("MCP server error ({}): {}", status, body);
        }

        let json_rpc: Value = resp.json().await.context("Failed to parse MCP response")?;

        // Check for JSON-RPC error
        if let Some(err) = json_rpc.get("error") {
            let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("unknown error");
            let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
            anyhow::bail!("MCP error [{}]: {}", code, msg);
        }

        // Extract result.content[0].text
        let result = json_rpc
            .pointer("/result/content/0/text")
            .and_then(|t| t.as_str())
            .map(|s| {
                serde_json::from_str::<Value>(s).unwrap_or_else(|_| Value::String(s.to_string()))
            })
            .unwrap_or_else(|| json_rpc.get("result").cloned().unwrap_or(Value::Null));

        Ok(result)
    }

    // ── Wiki REST API ──

    /// GET /api/wiki/search?q=...&limit=...
    pub async fn search_wiki(&self, query: &str, limit: usize) -> Result<Value> {
        let url =
            format!("{}/api/wiki/search?q={}&limit={}", self.cfg.server, urlencoding(query), limit);

        let resp = self.inner.get(&url).send().await.context("Search request failed")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Search failed ({}): {}", status, body);
        }

        resp.json().await.context("Failed to parse search response")
    }

    /// GET /api/wiki/stats
    pub async fn wiki_stats(&self) -> Result<Value> {
        let url = format!("{}/api/wiki/stats", self.cfg.server);

        let resp = self.inner.get(&url).send().await.context("Stats request failed")?;

        resp.json().await.context("Failed to parse stats response")
    }

    /// GET /api/wiki/entries/{path}
    pub async fn wiki_entry(&self, path: &str) -> Result<Value> {
        let url = format!("{}/api/wiki/entries/{}", self.cfg.server, path);

        let resp = self.inner.get(&url).send().await.context("Entry request failed")?;

        resp.json().await.context("Failed to parse entry response")
    }

    /// GET /api/wiki/graph/{path}?depth={depth}
    pub async fn wiki_concept_map(&self, path: &str, depth: u32) -> Result<Value> {
        let url = format!("{}/api/wiki/graph/{}?depth={}", self.cfg.server, path, depth);

        let resp = self.inner.get(&url).send().await.context("Concept map request failed")?;

        resp.json().await.context("Failed to parse concept map response")
    }

    /// GET /api/health
    pub async fn health(&self) -> Result<Value> {
        let url = format!("{}/api/health", self.cfg.server);

        let resp = self.inner.get(&url).send().await.context("Health check request failed")?;

        resp.json().await.context("Failed to parse health response")
    }

    /// GET /api/config
    pub async fn get_config(&self) -> Result<Value> {
        let url = format!("{}/api/config", self.cfg.server);

        let resp = self.inner.get(&url).send().await.context("Config request failed")?;

        resp.json().await.context("Failed to parse config response")
    }

    /// GET /api/compile/status
    pub async fn compile_status(&self) -> Result<Value> {
        let url = format!("{}/api/compile/status", self.cfg.server);

        let resp = self.inner.get(&url).send().await.context("Compile status request failed")?;

        resp.json().await.context("Failed to parse compile status response")
    }
}

/// A file reader wrapper that reports progress via indicatif.
struct ProgressReader<T> {
    inner: T,
    progress: ProgressBar,
}

impl<T: tokio::io::AsyncRead + Unpin> tokio::io::AsyncRead for ProgressReader<T> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let poll = std::pin::Pin::new(&mut self.inner).poll_read(cx, buf);
        let after = buf.filled().len();
        let delta = after - before;
        if delta > 0 {
            self.progress.inc(delta as u64);
        }
        poll
    }
}

/// URL-encode a query string (simple version).
fn urlencoding(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 16);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push_str("%20"),
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}
