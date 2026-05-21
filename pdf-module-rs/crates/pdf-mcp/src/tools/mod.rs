mod code_mode;
mod compile_sampling;
mod extract;
mod index;
mod json;
mod knowledge;
mod management;
pub mod mcp_extraction;
mod platform;
pub mod post_compile;
mod resources;

pub use extract::*;
pub use index::*;
pub use knowledge::*;
pub use management::*;
pub use platform::*;
pub use resources::*;

use crate::protocol::{Content, ToolDefinition};
use crate::sampling::SamplingClient;
use crate::upload::UploadStore;
use pdf_core::knowledge::IndexCache;
use pdf_core::management::WorkspaceRegistry;
use pdf_core::{McpPdfPipeline, PathValidationConfig};
use std::sync::Arc;

pub fn default_path_config() -> PathValidationConfig {
    PathValidationConfig { require_absolute: true, allow_traversal: false, base_dir: None }
}

pub struct ToolContext {
    pub pipeline: Arc<McpPdfPipeline>,
    pub path_config: PathValidationConfig,
    pub upload_store: Option<Arc<UploadStore>>,
    pub workspace_registry: Arc<WorkspaceRegistry>,
    pub index_cache: Arc<IndexCache>,
    /// Set on stdio MCP server when sampling loop is active.
    pub sampling: Option<Arc<SamplingClient>>,
}

impl ToolContext {
    pub fn new(
        pipeline: Arc<McpPdfPipeline>,
        workspace_registry: Arc<WorkspaceRegistry>,
        index_cache: Arc<IndexCache>,
    ) -> Self {
        Self {
            pipeline,
            path_config: default_path_config(),
            upload_store: None,
            workspace_registry,
            index_cache,
            sampling: None,
        }
    }

    pub fn with_sampling(mut self, sampling: Arc<SamplingClient>) -> Self {
        self.sampling = Some(sampling);
        self
    }

    pub fn new_with_upload_store(
        pipeline: Arc<McpPdfPipeline>,
        upload_store: Option<Arc<UploadStore>>,
        workspace_registry: Arc<WorkspaceRegistry>,
        index_cache: Arc<IndexCache>,
    ) -> Self {
        Self {
            pipeline,
            path_config: default_path_config(),
            upload_store,
            workspace_registry,
            index_cache,
            sampling: None,
        }
    }
}

pub fn all_tool_definitions() -> Vec<ToolDefinition> {
    if code_mode::is_code_mode() {
        return code_mode::tool_definitions();
    }
    pdf_mcp_contracts::all_tool_specs().into_iter().map(ToolDefinition::from).collect()
}

pub fn mcp_mode_label() -> &'static str {
    if code_mode::is_code_mode() { "code" } else { "full" }
}

#[tracing::instrument(skip(ctx, args), fields(tool = %tool_name))]
pub async fn dispatch_tool(
    ctx: &ToolContext,
    tool_name: &str,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    match tool_name {
        "search_compendium_api" if code_mode::is_code_mode() => {
            code_mode::handle_search_compendium_api(args).await
        }
        "execute_compendium" if code_mode::is_code_mode() => {
            code_mode::handle_execute_compendium(ctx, args).await
        }
        _ => dispatch_api_tool(ctx, tool_name, args).await,
    }
}

/// Dispatch a manifest API method (used by full mode and `execute_compendium` batches).
#[tracing::instrument(skip(ctx, args), fields(tool = %tool_name))]
pub async fn dispatch_api_tool(
    ctx: &ToolContext,
    tool_name: &str,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    match tool_name {
        "extract_text" => handle_extract_text(ctx, args).await,
        "extract_structured" => handle_extract_structured(ctx, args).await,
        "get_page_count" => handle_get_page_count(ctx, args).await,
        "search_keywords" => handle_search_keywords(ctx, args).await,
        "extrude_to_server_wiki" => handle_extrude_to_server_wiki(ctx, args).await,
        "extrude_to_agent_payload" => handle_extrude_to_agent_payload(ctx, args).await,
        "init_knowledge_base" => handle_init_knowledge_base(args).await,
        "lint_wiki" => handle_lint_wiki(args).await,
        "archive_answer" => handle_archive_answer(args).await,
        "compile_to_wiki" => handle_compile_to_wiki(ctx, args).await,
        "compile_uploaded_pdf" => handle_compile_uploaded_pdf(ctx, args).await,
        "incremental_compile" => handle_incremental_compile(ctx, args).await,
        "search_knowledge" => handle_search_knowledge(ctx, args).await,
        "rebuild_index" => handle_rebuild_index(ctx, args).await,
        "get_entry_context" => handle_get_entry_context(&ctx.workspace_registry, args).await,
        "get_agent_context" => handle_get_agent_context(&ctx.workspace_registry, args).await,
        "preview_wiki_patch" => handle_preview_wiki_patch(&ctx.workspace_registry, args).await,
        "patch_wiki_entry" | "apply_wiki_patch" => {
            handle_patch_wiki_entry(&ctx.workspace_registry, args).await
        }
        "get_compilation_context" => {
            handle_get_compilation_context(&ctx.workspace_registry, args).await
        }
        "find_orphans" => handle_find_orphans(&ctx.workspace_registry, args).await,
        "suggest_links" => handle_suggest_links(&ctx.workspace_registry, args).await,
        "export_concept_map" => handle_export_concept_map(&ctx.workspace_registry, args).await,
        "check_quality" => handle_check_quality(&ctx.workspace_registry, args).await,
        "micro_compile" => handle_micro_compile(ctx, args).await,
        "aggregate_entries" => handle_aggregate_entries(ctx, args).await,
        "hypothesis_test" => handle_hypothesis_test(ctx, args).await,
        "recompile_entry" => handle_recompile_entry(ctx, args).await,
        "save_wiki_entry" => handle_save_wiki_entry(ctx, args).await,
        "complete_compile_job" => handle_complete_compile_job(ctx, args).await,
        "generate_compile_plan" => handle_generate_compile_plan(ctx, args).await,
        "get_compile_plan" => handle_get_compile_plan(ctx, args).await,
        "mark_plan_task_done" => handle_mark_plan_task_done(ctx, args).await,
        "get_config" => handle_get_config(&ctx.workspace_registry, args).await,
        "set_config" => handle_set_config(&ctx.workspace_registry, args).await,
        "get_health_report" => handle_get_health_report(ctx, args).await,
        "trigger_incremental_compile" => handle_trigger_incremental_compile(ctx, args).await,
        "get_compile_status" => handle_get_compile_status(&ctx.workspace_registry, args).await,
        "list_quality_issues" => handle_list_quality_issues(&ctx.workspace_registry, args).await,
        "fix_suggest" => handle_fix_suggest(&ctx.workspace_registry, args).await,
        "apply_quality_gate" => handle_apply_quality_gate(&ctx.workspace_registry, args).await,
        "show_wiki_browser" => handle_show_wiki_browser().await,
        "list_workspaces" => handle_list_workspaces(&ctx.workspace_registry).await,
        "set_active_workspace" => handle_set_active_workspace(&ctx.workspace_registry, args).await,
        "register_workspace" => handle_register_workspace(&ctx.workspace_registry, args).await,
        "list_extraction_plugins" => handle_list_extraction_plugins(ctx).await,
        "probe_extraction" => handle_probe_extraction(ctx, args).await,
        "sync_status" => handle_sync_status(&ctx.workspace_registry, args).await,
        "sync_push" => handle_sync_push(&ctx.workspace_registry, args).await,
        "sync_pull" => handle_sync_pull(&ctx.workspace_registry, args).await,
        "submit_patch_proposal" => {
            handle_submit_patch_proposal(&ctx.workspace_registry, args).await
        }
        "apply_patch_proposal" => handle_apply_patch_proposal(&ctx.workspace_registry, args).await,
        "list_patch_proposals" => handle_list_patch_proposals(&ctx.workspace_registry, args).await,
        _ => Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
    }
}

/// Resolve knowledge base path: `kb_id` (preferred) or `knowledge_base` or active workspace.
pub fn parse_kb_path(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<std::path::PathBuf> {
    let kb_id = args["kb_id"].as_str();
    let knowledge_base = args["knowledge_base"].as_str();
    registry.resolve_kb(kb_id, knowledge_base).map_err(|e| anyhow::anyhow!("{e}"))
}

#[cfg(test)]
pub(crate) fn create_test_tool_context() -> ToolContext {
    use pdf_core::management::WorkspaceEntry;
    use pdf_core::{McpPdfPipeline, ServerConfig};
    use std::sync::Arc;

    let config = ServerConfig::from_env().unwrap_or_default();
    let pipeline = Arc::new(McpPdfPipeline::new(&config).expect("Failed to create pipeline"));
    static NEXT_TEST_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = NEXT_TEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let cfg_path =
        std::env::temp_dir().join(format!("rsut_test_workspaces_{}_{id}.toml", std::process::id()));
    let registry = Arc::new(WorkspaceRegistry::load(&cfg_path).expect("registry"));
    let kb_path =
        std::env::temp_dir().join(format!("rsut_mcp_test_kb_{}_{id}", std::process::id()));
    std::fs::create_dir_all(&kb_path).expect("kb dir");
    let _ = registry.upsert(WorkspaceEntry {
        id: "default".into(),
        name: "Test KB".into(),
        path: kb_path,
        active: true,
    });
    ToolContext::new(pipeline, registry, Arc::new(IndexCache::new()))
}

/// Merge optional sampling summary into a compile tool result object.
pub async fn attach_compile_sampling(
    ctx: &ToolContext,
    kb_path: &std::path::Path,
    job_id: &str,
    result: &mut serde_json::Value,
) {
    if let Some(summary) = compile_sampling::maybe_run_compile_sampling(ctx, kb_path, job_id).await
        && let Some(obj) = result.as_object_mut()
    {
        obj.insert(
            "sampling_summary".to_string(),
            serde_json::to_value(&summary).unwrap_or(serde_json::Value::Null),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> ToolContext {
        create_test_tool_context()
    }

    #[test]
    fn test_all_tool_definitions_count_full_mode() {
        let tools = pdf_mcp_contracts::all_tool_specs()
            .into_iter()
            .map(ToolDefinition::from)
            .collect::<Vec<_>>();
        assert_eq!(tools.len(), pdf_mcp_contracts::tool_count());
    }

    #[test]
    fn test_code_mode_tool_definitions_count() {
        let tools = code_mode::tool_definitions();
        assert_eq!(tools.len(), pdf_mcp_contracts::code_mode_tool_count());
    }

    #[test]
    fn test_all_tool_definitions_unique_names() {
        let tools = all_tool_definitions();
        let names: std::collections::HashSet<&str> =
            tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names.len(), tools.len(), "All tool names should be unique");
    }

    #[test]
    fn test_all_tool_definitions_have_required_fields() {
        let tools = all_tool_definitions();
        for tool in &tools {
            assert!(!tool.name.is_empty(), "Tool name should not be empty");
            assert!(!tool.description.is_empty(), "Tool description should not be empty");
            assert!(tool.input_schema.is_object(), "Tool input_schema should be an object");
            assert!(
                tool.output_schema.as_ref().is_some_and(|s| s.is_object()),
                "Tool {} missing outputSchema",
                tool.name
            );
        }
    }

    #[test]
    fn test_default_path_config() {
        let config = default_path_config();
        assert!(config.require_absolute);
        assert!(!config.allow_traversal);
        assert!(config.base_dir.is_none());
    }

    #[test]
    fn test_parse_kb_path_valid() {
        let dir = tempfile::tempdir().expect("tempdir");
        let kb = dir.path().join("kb");
        std::fs::create_dir_all(&kb).expect("mkdir");
        let cfg = dir.path().join("workspaces.toml");
        let registry = WorkspaceRegistry::load(&cfg).expect("load");
        registry
            .upsert(pdf_core::management::WorkspaceEntry {
                id: "t".into(),
                name: "T".into(),
                path: kb.clone(),
                active: true,
            })
            .expect("upsert");
        let args = serde_json::json!({ "kb_id": "t" });
        let result = parse_kb_path(&registry, &args);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dispatch_unknown_tool() {
        let ctx = create_test_context();
        let args = serde_json::json!({});
        let result = dispatch_tool(&ctx, "unknown_tool_name", &args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown tool"));
    }
}
