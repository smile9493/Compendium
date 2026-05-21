//! Code Mode API catalog — lightweight method index without full JSON Schemas.
//!
//! Agents discover methods via `search_compendium_api` or the `compendium://sdk/typescript`
//! resource; execution goes through `execute_compendium` with `{ method, args }` batches.

use serde::{Deserialize, Serialize};

use crate::registry::{McpToolSpec, all_tool_specs};

/// MCP mode: `full` exposes all tools; `code` exposes search + execute only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompendiumMcpMode {
    Full,
    Code,
}

impl CompendiumMcpMode {
    pub fn from_env() -> Self {
        match std::env::var("COMPENDIUM_MCP_MODE").ok().as_deref().map(str::trim) {
            Some(s) if s.eq_ignore_ascii_case("code") => Self::Code,
            _ => Self::Full,
        }
    }
}

/// One API method in the Code Mode catalog (no input/output JSON Schema).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
pub struct ApiMethodEntry {
    pub name: String,
    pub description: String,
    pub category: String,
    /// TypeScript-style one-liner for agents, e.g. `searchKnowledge(args): Promise<unknown>`
    pub ts_signature: String,
}

/// Category for a tool name based on manifest module grouping.
pub fn tool_category(name: &str) -> &'static str {
    match name {
        "extract_text"
        | "extract_structured"
        | "get_page_count"
        | "search_keywords"
        | "extrude_to_server_wiki"
        | "extrude_to_agent_payload" => "extract",
        "init_knowledge_base"
        | "lint_wiki"
        | "detect_stale_entries"
        | "ingest"
        | "query"
        | "lint"
        | "load_tools"
        | "archive_answer"
        | "compile_to_wiki"
        | "compile_image"
        | "incremental_compile"
        | "micro_compile"
        | "aggregate_entries"
        | "hypothesis_test"
        | "recompile_entry"
        | "save_wiki_entry"
        | "complete_compile_job"
        | "generate_compile_plan"
        | "get_compile_plan"
        | "mark_plan_task_done"
        | "compile_uploaded_pdf" => "knowledge",
        "search_knowledge"
        | "rebuild_index"
        | "get_entry_context"
        | "get_agent_context"
        | "preview_wiki_patch"
        | "patch_wiki_entry"
        | "apply_wiki_patch"
        | "get_compilation_context"
        | "find_orphans"
        | "suggest_links"
        | "export_concept_map"
        | "check_quality" => "index",
        "get_config"
        | "set_config"
        | "get_health_report"
        | "trigger_incremental_compile"
        | "get_compile_status"
        | "list_quality_issues"
        | "fix_suggest"
        | "apply_quality_gate"
        | "show_wiki_browser" => "management",
        "list_workspaces"
        | "set_active_workspace"
        | "register_workspace"
        | "list_extraction_plugins"
        | "probe_extraction"
        | "sync_status"
        | "sync_push"
        | "sync_pull"
        | "submit_patch_proposal"
        | "apply_patch_proposal"
        | "list_patch_proposals" => "platform",
        _ => "other",
    }
}

/// Convert snake_case tool name to camelCase for TypeScript signatures.
pub fn snake_to_camel(name: &str) -> String {
    let mut out = String::new();
    let mut upper_next = false;
    for ch in name.chars() {
        if ch == '_' {
            upper_next = true;
        } else if upper_next {
            out.extend(ch.to_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn entry_from_spec(spec: &McpToolSpec) -> ApiMethodEntry {
    let camel = snake_to_camel(&spec.name);
    ApiMethodEntry {
        name: spec.name.clone(),
        description: spec.description.clone(),
        category: tool_category(&spec.name).to_string(),
        ts_signature: format!("{camel}(args: Record<string, unknown>): Promise<unknown>"),
    }
}

/// Flat catalog of all invocable MCP methods (53 in full manifest).
pub fn all_api_methods() -> Vec<ApiMethodEntry> {
    all_tool_specs().iter().map(entry_from_spec).collect()
}

/// Allowed method names for `execute_compendium` whitelist.
pub fn allowed_tool_names() -> std::collections::HashSet<String> {
    all_tool_specs().into_iter().map(|s| s.name).collect()
}

/// Keyword search over method name and description (name match scores higher).
pub fn search_api(query: &str, limit: usize) -> Vec<ApiMethodEntry> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return all_api_methods().into_iter().take(limit).collect();
    }

    let mut scored: Vec<(i32, ApiMethodEntry)> = all_api_methods()
        .into_iter()
        .filter_map(|entry| {
            let name_l = entry.name.to_ascii_lowercase();
            let desc_l = entry.description.to_ascii_lowercase();
            let camel = snake_to_camel(&entry.name).to_ascii_lowercase();
            let mut score = 0i32;
            if name_l == q || camel == q {
                score += 100;
            } else if name_l.contains(&q) || camel.contains(&q) {
                score += 50;
            }
            if desc_l.contains(&q) {
                score += 10;
            }
            if entry.category.contains(&q) {
                score += 5;
            }
            (score > 0).then_some((score, entry))
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
    scored.into_iter().take(limit).map(|(_, e)| e).collect()
}

/// Code Mode MCP tool specs (2 tools only).
pub fn code_mode_tool_specs() -> Vec<McpToolSpec> {
    vec![
        McpToolSpec::new::<SearchCompendiumApiInput, SearchCompendiumApiOutput>(
            "search_compendium_api",
            "Search the Compendium API catalog by keyword (method name or description). Use before execute_compendium when unsure which method to call.",
        ),
        McpToolSpec::new::<ExecuteCompendiumInput, ExecuteCompendiumOutput>(
            "execute_compendium",
            "Run one or more Compendium API methods in-process. Pass calls: [{ method, args }]. Read compendium://sdk/typescript for signatures. Prefer batches for multi-step ingest/query workflows.",
        ),
    ]
}

pub fn code_mode_tool_count() -> usize {
    code_mode_tool_specs().len()
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SearchCompendiumApiInput {
    pub query: String,
    #[serde(default = "default_search_limit")]
    pub limit: u32,
}

fn default_search_limit() -> u32 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SearchCompendiumApiOutput {
    pub hits: Vec<ApiMethodEntry>,
    pub total_catalog: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CompendiumCall {
    pub method: String,
    #[serde(default)]
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ExecuteCompendiumInput {
    pub calls: Vec<CompendiumCall>,
    #[serde(default)]
    pub stop_on_error: bool,
    #[serde(default = "default_max_calls")]
    pub max_calls: u32,
    #[serde(default = "default_max_result_chars")]
    pub max_result_chars: u32,
}

fn default_max_calls() -> u32 {
    10
}

fn default_max_result_chars() -> u32 {
    8192
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CompendiumCallResult {
    pub method: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ExecuteCompendiumOutput {
    pub results: Vec<CompendiumCallResult>,
    pub executed: usize,
}

/// Generate TypeScript declaration file content for the full API catalog.
pub fn generate_typescript_sdk() -> String {
    let methods = all_api_methods();
    let mut out = String::from(
        "/**\n * Compendium Code Mode API — generated from pdf-mcp-contracts.\n *\n * Invoke via MCP tool `execute_compendium` with:\n *   calls: [{ method: \"search_knowledge\", args: { ... } }]\n */\n\ndeclare namespace Compendium {\n  type Args = Record<string, unknown>;\n\n",
    );
    let mut last_cat = "";
    for m in &methods {
        if m.category != last_cat {
            out.push_str(&format!("\n  // --- {} ---\n", m.category));
            last_cat = &m.category;
        }
        let desc = m.description.replace('\n', " ");
        out.push_str(&format!(
            "  /** {} */\n  function {}(args: Args): Promise<unknown>;\n",
            desc,
            snake_to_camel(&m.name)
        ));
    }
    out.push_str("\n}\n\nexport = Compendium;\n");
    out
}

/// JSON API index for embedding or code generation checks.
pub fn generate_api_index_json() -> String {
    serde_json::to_string_pretty(&all_api_methods()).expect("api index serializes")
}

/// `initialize` instructions when COMPENDIUM_MCP_MODE=code.
pub fn code_mode_instructions() -> &'static str {
    "Compendium Code Mode (contract 1.1.0). Only two MCP tools: search_compendium_api, execute_compendium. \
     Read knowledge_base/schema/AGENTS.md for ingest/query/lint workflows. \
     Discover methods: search_compendium_api or resource compendium://sdk/typescript. \
     Execute: execute_compendium with calls: [{ method, args }]. Example methods: compile_to_wiki, \
     search_knowledge, get_agent_context, lint_wiki, save_wiki_entry, complete_compile_job. \
     Use batches for multi-step flows; each result is { ok, data | error }."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_api_prefers_name_match() {
        let hits = search_api("search_knowledge", 5);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].name, "search_knowledge");
    }

    #[test]
    fn all_api_methods_count_matches_tools() {
        assert_eq!(all_api_methods().len(), crate::tool_count());
    }

    #[test]
    fn code_mode_tool_count_is_two() {
        assert_eq!(code_mode_tool_count(), 2);
    }

    #[test]
    fn generate_typescript_contains_search_knowledge() {
        let dts = generate_typescript_sdk();
        assert!(dts.contains("searchKnowledge"));
        assert!(dts.contains("search_knowledge") || dts.contains("searchKnowledge"));
    }
}
