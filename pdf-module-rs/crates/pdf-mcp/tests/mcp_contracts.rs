//! Integration tests: MCP manifest consistency and golden output shapes.

use pdf_mcp::tools::all_tool_definitions;
use pdf_mcp_contracts::{
    CompileToWikiOutput, ExtractTextOutput, GetAgentContextOutput, GetCompilationContextOutput,
    GetCompileStatusOutput, GetHealthReportOutput, ListWorkspacesOutput, PatchWikiEntryOutput,
    ProbeExtractionOutput, RebuildIndexOutput, SaveWikiEntryOutput, SearchKnowledgeOutput,
    ShowWikiBrowserOutput,
};

#[test]
fn manifest_matches_tool_definitions() {
    let defs = all_tool_definitions();
    let specs = pdf_mcp_contracts::all_tool_specs();
    let core_specs: Vec<_> =
        specs.iter().filter(|s| pdf_mcp_contracts::listed_in_default_manifest(&s.name)).collect();
    assert_eq!(defs.len(), core_specs.len());
    assert_eq!(
        defs.len(),
        pdf_mcp_contracts::tools_in_tier(pdf_mcp_contracts::ToolExposureTier::Core).len()
    );
    for spec in core_specs {
        let def = defs.iter().find(|d| d.name == spec.name).expect("core tool in default list");
        assert_eq!(def.input_schema, spec.input_schema);
        assert_eq!(def.output_schema.as_ref(), Some(&spec.output_schema));
    }
}

#[test]
fn golden_extract_text_output_deserializes() {
    let raw = include_str!("fixtures/mcp_outputs/extract_text.json");
    let _: ExtractTextOutput = serde_json::from_str(raw).expect("extract_text golden");
}

#[test]
fn golden_search_knowledge_output_deserializes() {
    let raw = include_str!("fixtures/mcp_outputs/search_knowledge.json");
    let _: SearchKnowledgeOutput = serde_json::from_str(raw).expect("search_knowledge golden");
}

#[test]
fn golden_health_report_output_deserializes() {
    let raw = include_str!("fixtures/mcp_outputs/get_health_report.json");
    let _: GetHealthReportOutput = serde_json::from_str(raw).expect("get_health_report golden");
}

#[test]
fn golden_probe_extraction_output_deserializes() {
    let raw = include_str!("fixtures/mcp_outputs/probe_extraction.json");
    let _: ProbeExtractionOutput = serde_json::from_str(raw).expect("probe_extraction golden");
}

#[test]
fn golden_compilation_context_output_deserializes() {
    let raw = include_str!("fixtures/mcp_outputs/get_compilation_context.json");
    let _: GetCompilationContextOutput =
        serde_json::from_str(raw).expect("get_compilation_context golden");
}

#[test]
fn golden_list_workspaces_output_deserializes() {
    let raw = include_str!("fixtures/mcp_outputs/list_workspaces.json");
    let _: ListWorkspacesOutput = serde_json::from_str(raw).expect("list_workspaces golden");
}

#[test]
fn golden_show_wiki_browser_output_deserializes() {
    let raw = include_str!("fixtures/mcp_outputs/show_wiki_browser.json");
    let _: ShowWikiBrowserOutput = serde_json::from_str(raw).expect("show_wiki_browser golden");
}

#[test]
fn golden_compile_to_wiki_output_deserializes() {
    let raw = include_str!("fixtures/mcp_outputs/compile_to_wiki.json");
    let _: CompileToWikiOutput = serde_json::from_str(raw).expect("compile_to_wiki golden");
}

#[test]
fn golden_get_compile_status_output_deserializes() {
    let raw = include_str!("fixtures/mcp_outputs/get_compile_status.json");
    let _: GetCompileStatusOutput = serde_json::from_str(raw).expect("get_compile_status golden");
}

#[test]
fn golden_patch_wiki_entry_output_deserializes() {
    let raw = include_str!("fixtures/mcp_outputs/patch_wiki_entry.json");
    let _: PatchWikiEntryOutput = serde_json::from_str(raw).expect("patch_wiki_entry golden");
}

#[test]
fn golden_get_agent_context_output_deserializes() {
    let raw = include_str!("fixtures/mcp_outputs/get_agent_context.json");
    let _: GetAgentContextOutput = serde_json::from_str(raw).expect("get_agent_context golden");
}

#[test]
fn golden_save_wiki_entry_output_deserializes() {
    let raw = include_str!("fixtures/mcp_outputs/save_wiki_entry.json");
    let _: SaveWikiEntryOutput = serde_json::from_str(raw).expect("save_wiki_entry golden");
}

#[test]
fn golden_rebuild_index_output_deserializes() {
    let raw = include_str!("fixtures/mcp_outputs/rebuild_index.json");
    let _: RebuildIndexOutput = serde_json::from_str(raw).expect("rebuild_index golden");
}
