use gloo_net::http::Request;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

fn api_base() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default()
}

pub async fn fetch_health() -> Result<HealthData, String> {
    let resp = Request::get(&format!("{}/api/health", api_base()))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn fetch_config() -> Result<ConfigData, String> {
    let resp = Request::get(&format!("{}/api/config", api_base()))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn set_config(key: &str, value: &str) -> Result<(), String> {
    let body = serde_json::json!({"key": key, "value": value});
    let resp = Request::post(&format!("{}/api/config", api_base()))
        .json(&body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.ok() {
        Ok(())
    } else {
        Err(resp.status_text())
    }
}

pub async fn delete_config(key: &str) -> Result<(), String> {
    let resp = Request::delete(&format!("{}/api/config/{}", api_base(), key))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.ok() {
        Ok(())
    } else {
        Err(resp.status_text())
    }
}

pub async fn fetch_compile_status() -> Result<CompileStatusData, String> {
    let resp = Request::get(&format!("{}/api/compile/status", api_base()))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn trigger_compile() -> Result<CompileResultData, String> {
    let resp = Request::post(&format!("{}/api/compile", api_base()))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn rebuild_index() -> Result<RebuildResultData, String> {
    let resp = Request::post(&format!("{}/api/index/rebuild", api_base()))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn fetch_wiki_tree() -> Result<WikiTreeData, String> {
    let resp = Request::get(&format!("{}/api/wiki/tree", api_base()))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn fetch_wiki_entry(path: &str) -> Result<WikiEntryData, String> {
    let resp = Request::get(&format!(
        "{}/api/wiki/entries/{}",
        api_base(),
        url_escape(path)
    ))
    .send()
    .await
    .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn search_wiki(query: &str) -> Result<SearchResults, String> {
    let resp = Request::get(&format!(
        "{}/api/wiki/search?q={}",
        api_base(),
        url_escape(query)
    ))
    .send()
    .await
    .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn fetch_wiki_stats() -> Result<WikiStatsData, String> {
    let resp = Request::get(&format!("{}/api/wiki/stats", api_base()))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn fetch_wiki_domains() -> Result<WikiDomainsData, String> {
    let resp = Request::get(&format!("{}/api/wiki/domains", api_base()))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn fetch_concept_map(path: &str) -> Result<ConceptMapData, String> {
    let resp = Request::get(&format!(
        "{}/api/wiki/concept-map/{}",
        api_base(),
        url_escape(path)
    ))
    .send()
    .await
    .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

fn url_escape(s: &str) -> String {
    web_sys::js_sys::encode_uri_component(s)
        .as_string()
        .unwrap_or_else(|| s.to_string())
}

// ── Data types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthData {
    pub total_entries: Option<u64>,
    pub orphan_count: Option<u64>,
    pub contradiction_count: Option<u64>,
    pub broken_link_count: Option<u64>,
    pub index_size_mb: Option<f64>,
    pub avg_quality_score: Option<String>,
    pub graph_nodes: Option<u64>,
    pub graph_edges: Option<u64>,
    pub domains: Option<Vec<String>>,
    pub last_compile: Option<String>,
    pub generated_at: Option<String>,
    pub report_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigData {
    pub config: Option<std::collections::HashMap<String, String>>,
    pub total_keys: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileStatusData {
    pub running: Option<bool>,
    pub last_started: Option<String>,
    pub last_finished: Option<String>,
    pub last_duration_ms: Option<u64>,
    pub last_outcome: Option<String>,
    pub message: Option<String>,
    pub history: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileResultData {
    pub status: Option<String>,
    pub message: Option<String>,
    pub compiled: Option<u64>,
    pub skipped: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuildResultData {
    pub status: Option<String>,
    pub fulltext_entries_indexed: Option<u64>,
    pub graph_nodes: Option<u64>,
    pub graph_edges: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiTreeData {
    pub tree: Option<TreeNode>,
    pub total: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode {
    pub name: Option<String>,
    pub title: Option<String>,
    pub path: Option<String>,
    pub is_entry: Option<bool>,
    pub children: Option<Vec<TreeNode>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiEntryData {
    pub entry: Option<WikiEntry>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiEntry {
    pub title: Option<String>,
    pub domain: Option<String>,
    pub tags: Option<Vec<String>>,
    pub level: Option<String>,
    pub status: Option<String>,
    pub quality_score: Option<f64>,
    pub version: Option<u32>,
    pub source: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub body_html: Option<String>,
    pub related: Option<Vec<String>>,
    pub contradictions: Option<Vec<String>>,
    pub backlinks: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub results: Option<Vec<SearchHit>>,
    pub total: Option<u64>,
    pub query: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub path: Option<String>,
    pub title: Option<String>,
    pub domain: Option<String>,
    pub score: Option<f64>,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiStatsData {
    pub total_entries: Option<u64>,
    pub orphan_count: Option<u64>,
    pub contradiction_count: Option<u64>,
    pub broken_link_count: Option<u64>,
    pub index_size_bytes: Option<u64>,
    pub graph_node_count: Option<u64>,
    pub graph_edge_count: Option<u64>,
    pub avg_quality_score: Option<f64>,
    pub domains: Option<Vec<String>>,
    pub last_compile: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiDomainsData {
    pub domains: Option<Vec<DomainStatus>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainStatus {
    pub domain: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptMapData {
    pub mermaid: Option<String>,
    pub entry: Option<String>,
    pub error: Option<String>,
}