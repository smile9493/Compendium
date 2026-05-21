//! MCP tool exposure tiers (hybrid manifest per ADR-006 / improvement directions).
//!
//! - **Core** (~15): listed in default `tools/list` for instant agent feedback.
//! - **Deferred** (~28): hidden from default `tools/list` but **directly callable** by name (no unlock).
//! - **CodeOnly** (~14): hidden; require Code Mode `execute_compendium` or `COMPENDIUM_UNLOCK_CODE_TOOLS=1`.
//! - **`load_tools`**: optional discovery only — never gates execution.

use serde::{Deserialize, Serialize};

/// How a tool is exposed on the MCP wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolExposureTier {
    /// Always listed in full-mode `tools/list`.
    Core,
    /// Omitted from default list; still dispatchable when invoked by name.
    Deferred,
    /// Requires Code Mode (`execute_compendium`) unless `COMPENDIUM_UNLOCK_CODE_TOOLS=1`.
    CodeOnly,
}

/// Classify every manifest tool (57 total).
pub fn tool_exposure_tier(name: &str) -> ToolExposureTier {
    match name {
        // --- Core (15) ---
        "search_knowledge"
        | "get_agent_context"
        | "lint_wiki"
        | "ingest"
        | "query"
        | "lint"
        | "compile_to_wiki"
        | "incremental_compile"
        | "save_wiki_entry"
        | "complete_compile_job"
        | "init_knowledge_base"
        | "detect_stale_entries"
        | "check_quality"
        | "get_compilation_context"
        | "rebuild_index"
        | "load_tools" => ToolExposureTier::Core,

        // --- CodeOnly (17) ---
        "extract_text"
        | "extract_structured"
        | "get_page_count"
        | "search_keywords"
        | "extrude_to_server_wiki"
        | "extrude_to_agent_payload"
        | "register_workspace"
        | "set_active_workspace"
        | "submit_patch_proposal"
        | "apply_patch_proposal"
        | "list_patch_proposals"
        | "show_wiki_browser"
        | "search_compendium_api"
        | "execute_compendium" => ToolExposureTier::CodeOnly,

        // --- Deferred (25) — everything else in the manifest ---
        _ => ToolExposureTier::Deferred,
    }
}

/// Whether `tools/list` should include this tool in full MCP mode.
pub fn listed_in_default_manifest(name: &str) -> bool {
    matches!(tool_exposure_tier(name), ToolExposureTier::Core)
}

/// Whether direct `tools/call` is allowed outside Code Mode execute batches.
pub fn direct_call_allowed(name: &str) -> bool {
    match tool_exposure_tier(name) {
        ToolExposureTier::Core | ToolExposureTier::Deferred => true,
        ToolExposureTier::CodeOnly => code_only_tools_unlocked(),
    }
}

/// Env escape hatch for local debugging of high-risk tools in full mode.
pub fn code_only_tools_unlocked() -> bool {
    std::env::var("COMPENDIUM_UNLOCK_CODE_TOOLS")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// Tool names in a tier (stable-sorted).
pub fn tools_in_tier(tier: ToolExposureTier) -> Vec<String> {
    crate::all_tool_specs()
        .into_iter()
        .filter(|s| tool_exposure_tier(&s.name) == tier)
        .map(|s| s.name)
        .collect()
}

/// Progressive index: core tools plus deferred names (no schemas).
pub fn progressive_tool_index() -> serde_json::Value {
    serde_json::json!({
        "core": tools_in_tier(ToolExposureTier::Core),
        "deferred": tools_in_tier(ToolExposureTier::Deferred),
        "code_only": tools_in_tier(ToolExposureTier::CodeOnly),
        "hint": "Deferred tools are hidden from tools/list but callable via tools/call. load_tools is discovery-only. Code-only tools need execute_compendium or COMPENDIUM_UNLOCK_CODE_TOOLS=1."
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_counts_sum_to_manifest() {
        let core = tools_in_tier(ToolExposureTier::Core).len();
        let def = tools_in_tier(ToolExposureTier::Deferred).len();
        let code = tools_in_tier(ToolExposureTier::CodeOnly).len();
        assert_eq!(core + def + code, crate::tool_count());
        assert_eq!(core, 16); // 15 atomic + load_tools
        assert_eq!(code, 12); // high-risk manifest tools (Code Mode adds +2)
        assert_eq!(def, 31); // includes sync_push / sync_pull
    }
}
