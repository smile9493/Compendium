mod code_mode;
mod compile_sampling;
mod extract;
mod index;
mod json;
mod knowledge;
mod management;
pub mod mcp_extraction;
mod meta_tools;
mod platform;
pub mod post_compile;
mod resources;

pub use extract::*;
pub use index::*;
pub use knowledge::*;
pub use management::*;
pub use meta_tools::*;
pub use platform::*;
pub use resources::*;

use crate::protocol::{Content, ToolDefinition};
use crate::sampling::SamplingClient;
use crate::upload::UploadStore;
use pdf_core::knowledge::IndexCache;
use pdf_core::management::WorkspaceRegistry;
use pdf_core::{McpPdfPipeline, PathValidationConfig};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::LazyLock;
use tokio_util::sync::CancellationToken;

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
    /// Cancellation token for graceful shutdown and tool call cancellation.
    pub cancel: CancellationToken,
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
            cancel: CancellationToken::new(),
        }
    }

    pub fn with_sampling(mut self, sampling: Arc<SamplingClient>) -> Self {
        self.sampling = Some(sampling);
        self
    }

    pub fn with_upload_store(mut self, upload_store: Option<Arc<UploadStore>>) -> Self {
        self.upload_store = upload_store;
        self
    }

    /// Set the cancellation token for this context.
    pub fn with_cancel(mut self, cancel: CancellationToken) -> Self {
        self.cancel = cancel;
        self
    }

    pub fn new_with_upload_store(
        pipeline: Arc<McpPdfPipeline>,
        upload_store: Option<Arc<UploadStore>>,
        workspace_registry: Arc<WorkspaceRegistry>,
        index_cache: Arc<IndexCache>,
    ) -> Self {
        Self { upload_store, ..Self::new(pipeline, workspace_registry, index_cache) }
    }
}

// ── Tool Registry (P1-3) ──

/// Boxed future returned by async tool handlers.
type BoxFut<'a> = Pin<Box<dyn Future<Output = anyhow::Result<Vec<Content>>> + Send + 'a>>;

/// Handler function type for async MCP tool calls.
type ToolHandler = for<'a> fn(&'a ToolContext, &'a serde_json::Value) -> BoxFut<'a>;

/// Declarative registry of MCP tool handlers, replacing the match-based dispatch.
pub struct ToolRegistry {
    handlers: HashMap<&'static str, ToolHandler>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { handlers: HashMap::new() }
    }

    /// Register a tool handler.
    pub fn register(&mut self, name: &'static str, handler: ToolHandler) {
        self.handlers.insert(name, handler);
    }

    /// Dispatch a tool call to the registered handler.
    pub async fn dispatch(
        &self,
        ctx: &ToolContext,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> anyhow::Result<Vec<Content>> {
        match self.handlers.get(tool_name) {
            Some(handler) => handler(ctx, args).await,
            None => anyhow::bail!("Unknown tool: {}", tool_name),
        }
    }

    /// Check if a tool is registered.
    pub fn contains(&self, tool_name: &str) -> bool {
        self.handlers.contains_key(tool_name)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Wrapper functions for handlers with non-standard signatures ──

fn wrap_init_knowledge_base<'a>(
    ctx: &'a ToolContext,
    args: &'a serde_json::Value,
) -> BoxFut<'a> {
    Box::pin(knowledge::handle_init_knowledge_base(&ctx.workspace_registry, args))
}

fn wrap_lint_wiki<'a>(ctx: &'a ToolContext, args: &'a serde_json::Value) -> BoxFut<'a> {
    Box::pin(knowledge::handle_lint_wiki(&ctx.workspace_registry, args))
}

fn wrap_detect_stale_entries<'a>(
    ctx: &'a ToolContext,
    args: &'a serde_json::Value,
) -> BoxFut<'a> {
    Box::pin(knowledge::handle_detect_stale_entries(&ctx.workspace_registry, args))
}

fn wrap_load_tools<'a>(ctx: &'a ToolContext, args: &'a serde_json::Value) -> BoxFut<'a> {
    let _ = ctx;
    Box::pin(meta_tools::handle_load_tools(args))
}

fn wrap_archive_answer<'a>(ctx: &'a ToolContext, args: &'a serde_json::Value) -> BoxFut<'a> {
    Box::pin(knowledge::handle_archive_answer(&ctx.workspace_registry, args))
}

fn wrap_get_entry_context<'a>(ctx: &'a ToolContext, args: &'a serde_json::Value) -> BoxFut<'a> {
    Box::pin(index::handle_get_entry_context(&ctx.workspace_registry, args))
}

fn wrap_get_wiki_entry<'a>(ctx: &'a ToolContext, args: &'a serde_json::Value) -> BoxFut<'a> {
    Box::pin(index::handle_get_wiki_entry(&ctx.workspace_registry, args))
}

fn wrap_get_agent_context<'a>(ctx: &'a ToolContext, args: &'a serde_json::Value) -> BoxFut<'a> {
    Box::pin(index::handle_get_agent_context(&ctx.workspace_registry, args))
}

fn wrap_preview_wiki_patch<'a>(
    ctx: &'a ToolContext,
    args: &'a serde_json::Value,
) -> BoxFut<'a> {
    Box::pin(index::handle_preview_wiki_patch(&ctx.workspace_registry, args))
}

fn wrap_patch_wiki_entry<'a>(ctx: &'a ToolContext, args: &'a serde_json::Value) -> BoxFut<'a> {
    Box::pin(index::handle_patch_wiki_entry(&ctx.workspace_registry, args))
}

fn wrap_get_compilation_context<'a>(
    ctx: &'a ToolContext,
    args: &'a serde_json::Value,
) -> BoxFut<'a> {
    Box::pin(index::handle_get_compilation_context(&ctx.workspace_registry, args))
}

fn wrap_find_orphans<'a>(ctx: &'a ToolContext, args: &'a serde_json::Value) -> BoxFut<'a> {
    Box::pin(index::handle_find_orphans(&ctx.workspace_registry, args))
}

fn wrap_suggest_links<'a>(ctx: &'a ToolContext, args: &'a serde_json::Value) -> BoxFut<'a> {
    Box::pin(index::handle_suggest_links(&ctx.workspace_registry, args))
}

fn wrap_export_concept_map<'a>(
    ctx: &'a ToolContext,
    args: &'a serde_json::Value,
) -> BoxFut<'a> {
    Box::pin(index::handle_export_concept_map(&ctx.workspace_registry, args))
}

fn wrap_check_quality<'a>(ctx: &'a ToolContext, args: &'a serde_json::Value) -> BoxFut<'a> {
    Box::pin(index::handle_check_quality(&ctx.workspace_registry, args))
}

fn wrap_get_config<'a>(ctx: &'a ToolContext, args: &'a serde_json::Value) -> BoxFut<'a> {
    Box::pin(management::handle_get_config(&ctx.workspace_registry, args))
}

fn wrap_set_config<'a>(ctx: &'a ToolContext, args: &'a serde_json::Value) -> BoxFut<'a> {
    Box::pin(management::handle_set_config(&ctx.workspace_registry, args))
}

fn wrap_get_compile_status<'a>(
    ctx: &'a ToolContext,
    args: &'a serde_json::Value,
) -> BoxFut<'a> {
    Box::pin(management::handle_get_compile_status(&ctx.workspace_registry, args))
}

fn wrap_list_quality_issues<'a>(
    ctx: &'a ToolContext,
    args: &'a serde_json::Value,
) -> BoxFut<'a> {
    Box::pin(management::handle_list_quality_issues(&ctx.workspace_registry, args))
}

fn wrap_fix_suggest<'a>(ctx: &'a ToolContext, args: &'a serde_json::Value) -> BoxFut<'a> {
    Box::pin(management::handle_fix_suggest(&ctx.workspace_registry, args))
}

fn wrap_apply_quality_gate<'a>(
    ctx: &'a ToolContext,
    args: &'a serde_json::Value,
) -> BoxFut<'a> {
    Box::pin(management::handle_apply_quality_gate(&ctx.workspace_registry, args))
}

fn wrap_show_wiki_browser<'a>(
    ctx: &'a ToolContext,
    args: &'a serde_json::Value,
) -> BoxFut<'a> {
    let _ = (ctx, args);
    Box::pin(management::handle_show_wiki_browser())
}

fn wrap_list_workspaces<'a>(ctx: &'a ToolContext, args: &'a serde_json::Value) -> BoxFut<'a> {
    let _ = args;
    Box::pin(platform::handle_list_workspaces(&ctx.workspace_registry))
}

fn wrap_set_active_workspace<'a>(
    ctx: &'a ToolContext,
    args: &'a serde_json::Value,
) -> BoxFut<'a> {
    Box::pin(platform::handle_set_active_workspace(&ctx.workspace_registry, args))
}

fn wrap_register_workspace<'a>(
    ctx: &'a ToolContext,
    args: &'a serde_json::Value,
) -> BoxFut<'a> {
    Box::pin(platform::handle_register_workspace(&ctx.workspace_registry, args))
}

fn wrap_list_extraction_plugins<'a>(
    ctx: &'a ToolContext,
    args: &'a serde_json::Value,
) -> BoxFut<'a> {
    let _ = args;
    Box::pin(platform::handle_list_extraction_plugins(ctx))
}

fn wrap_sync_status<'a>(ctx: &'a ToolContext, args: &'a serde_json::Value) -> BoxFut<'a> {
    Box::pin(platform::handle_sync_status(&ctx.workspace_registry, args))
}

fn wrap_sync_push<'a>(ctx: &'a ToolContext, args: &'a serde_json::Value) -> BoxFut<'a> {
    Box::pin(platform::handle_sync_push(&ctx.workspace_registry, args))
}

fn wrap_sync_pull<'a>(ctx: &'a ToolContext, args: &'a serde_json::Value) -> BoxFut<'a> {
    Box::pin(platform::handle_sync_pull(&ctx.workspace_registry, args))
}

fn wrap_submit_patch_proposal<'a>(
    ctx: &'a ToolContext,
    args: &'a serde_json::Value,
) -> BoxFut<'a> {
    Box::pin(platform::handle_submit_patch_proposal(&ctx.workspace_registry, args))
}

fn wrap_apply_patch_proposal<'a>(
    ctx: &'a ToolContext,
    args: &'a serde_json::Value,
) -> BoxFut<'a> {
    Box::pin(platform::handle_apply_patch_proposal(&ctx.workspace_registry, args))
}

fn wrap_list_patch_proposals<'a>(
    ctx: &'a ToolContext,
    args: &'a serde_json::Value,
) -> BoxFut<'a> {
    Box::pin(platform::handle_list_patch_proposals(&ctx.workspace_registry, args))
}

/// Build the tool registry by registering all tool handlers.
fn build_tool_registry() -> ToolRegistry {
    let mut r = ToolRegistry::new();

    // ── Extract tools ──
    r.register("extract_text", |ctx, args| Box::pin(extract::handle_extract_text(ctx, args)));
    r.register("extract_structured", |ctx, args| {
        Box::pin(extract::handle_extract_structured(ctx, args))
    });
    r.register("get_page_count", |ctx, args| {
        Box::pin(extract::handle_get_page_count(ctx, args))
    });
    r.register("search_keywords", |ctx, args| {
        Box::pin(extract::handle_search_keywords(ctx, args))
    });
    r.register("extrude_to_server_wiki", |ctx, args| {
        Box::pin(extract::handle_extrude_to_server_wiki(ctx, args))
    });
    r.register("extrude_to_agent_payload", |ctx, args| {
        Box::pin(extract::handle_extrude_to_agent_payload(ctx, args))
    });

    // ── Knowledge tools ──
    r.register("init_knowledge_base", wrap_init_knowledge_base);
    r.register("lint_wiki", wrap_lint_wiki);
    r.register("detect_stale_entries", wrap_detect_stale_entries);
    r.register("archive_answer", wrap_archive_answer);
    r.register("compile_to_wiki", |ctx, args| {
        Box::pin(knowledge::handle_compile_to_wiki(ctx, args))
    });
    r.register("compile_image", |ctx, args| {
        Box::pin(knowledge::handle_compile_image(ctx, args))
    });
    r.register("compile_uploaded_pdf", |ctx, args| {
        Box::pin(knowledge::handle_compile_uploaded_pdf(ctx, args))
    });
    r.register("incremental_compile", |ctx, args| {
        Box::pin(knowledge::handle_incremental_compile(ctx, args))
    });
    r.register("micro_compile", |ctx, args| {
        Box::pin(knowledge::handle_micro_compile(ctx, args))
    });
    r.register("aggregate_entries", |ctx, args| {
        Box::pin(knowledge::handle_aggregate_entries(ctx, args))
    });
    r.register("hypothesis_test", |ctx, args| {
        Box::pin(knowledge::handle_hypothesis_test(ctx, args))
    });
    r.register("recompile_entry", |ctx, args| {
        Box::pin(knowledge::handle_recompile_entry(ctx, args))
    });
    r.register("save_wiki_entry", |ctx, args| {
        Box::pin(knowledge::handle_save_wiki_entry(ctx, args))
    });
    r.register("complete_compile_job", |ctx, args| {
        Box::pin(knowledge::handle_complete_compile_job(ctx, args))
    });
    r.register("generate_compile_plan", |ctx, args| {
        Box::pin(knowledge::handle_generate_compile_plan(ctx, args))
    });
    r.register("get_compile_plan", |ctx, args| {
        Box::pin(knowledge::handle_get_compile_plan(ctx, args))
    });
    r.register("mark_plan_task_done", |ctx, args| {
        Box::pin(knowledge::handle_mark_plan_task_done(ctx, args))
    });

    // ── Index tools ──
    r.register("search_knowledge", |ctx, args| {
        Box::pin(index::handle_search_knowledge(ctx, args))
    });
    r.register("rebuild_index", |ctx, args| {
        Box::pin(index::handle_rebuild_index(ctx, args))
    });
    r.register("get_entry_context", wrap_get_entry_context);
    r.register("get_wiki_entry", wrap_get_wiki_entry);
    r.register("get_agent_context", wrap_get_agent_context);
    r.register("preview_wiki_patch", wrap_preview_wiki_patch);
    r.register("patch_wiki_entry", wrap_patch_wiki_entry);
    r.register("apply_wiki_patch", wrap_patch_wiki_entry);
    r.register("get_compilation_context", wrap_get_compilation_context);
    r.register("find_orphans", wrap_find_orphans);
    r.register("suggest_links", wrap_suggest_links);
    r.register("export_concept_map", wrap_export_concept_map);
    r.register("check_quality", wrap_check_quality);

    // ── Management tools ──
    r.register("get_config", wrap_get_config);
    r.register("set_config", wrap_set_config);
    r.register("get_health_report", |ctx, args| {
        Box::pin(management::handle_get_health_report(ctx, args))
    });
    r.register("trigger_incremental_compile", |ctx, args| {
        Box::pin(management::handle_trigger_incremental_compile(ctx, args))
    });
    r.register("get_compile_status", wrap_get_compile_status);
    r.register("list_quality_issues", wrap_list_quality_issues);
    r.register("fix_suggest", wrap_fix_suggest);
    r.register("apply_quality_gate", wrap_apply_quality_gate);
    r.register("show_wiki_browser", wrap_show_wiki_browser);

    // ── Meta tools ──
    r.register("ingest", |ctx, args| Box::pin(meta_tools::handle_meta_ingest(ctx, args)));
    r.register("query", |ctx, args| Box::pin(meta_tools::handle_meta_query(ctx, args)));
    r.register("lint", |ctx, args| Box::pin(meta_tools::handle_meta_lint(ctx, args)));
    r.register("load_tools", wrap_load_tools);

    // ── Platform tools ──
    r.register("list_workspaces", wrap_list_workspaces);
    r.register("set_active_workspace", wrap_set_active_workspace);
    r.register("register_workspace", wrap_register_workspace);
    r.register("list_extraction_plugins", wrap_list_extraction_plugins);
    r.register("probe_extraction", |ctx, args| {
        Box::pin(platform::handle_probe_extraction(ctx, args))
    });
    r.register("sync_status", wrap_sync_status);
    r.register("sync_push", wrap_sync_push);
    r.register("sync_pull", wrap_sync_pull);
    r.register("submit_patch_proposal", wrap_submit_patch_proposal);
    r.register("apply_patch_proposal", wrap_apply_patch_proposal);
    r.register("list_patch_proposals", wrap_list_patch_proposals);

    r
}

/// Global tool registry, initialized once on first access.
static TOOL_REGISTRY: LazyLock<ToolRegistry> = LazyLock::new(build_tool_registry);

pub fn all_tool_definitions() -> Vec<ToolDefinition> {
    if code_mode::is_code_mode() {
        return code_mode::tool_definitions();
    }
    pdf_mcp_contracts::all_tool_specs()
        .into_iter()
        .filter(|s| pdf_mcp_contracts::listed_in_default_manifest(&s.name))
        .map(ToolDefinition::from)
        .collect()
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
    if !pdf_mcp_contracts::direct_call_allowed(tool_name) {
        anyhow::bail!(
            "tool '{tool_name}' is CodeOnly — use COMPENDIUM_MCP_MODE=code with execute_compendium, or set COMPENDIUM_UNLOCK_CODE_TOOLS=1"
        );
    }
    dispatch_api_tool_inner(ctx, tool_name, args).await
}

/// Dispatch without tier guard (Code Mode `execute_compendium` batches).
pub async fn dispatch_api_tool_inner(
    ctx: &ToolContext,
    tool_name: &str,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    TOOL_REGISTRY.dispatch(ctx, tool_name, args).await
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

    #[test]
    fn test_tool_registry_contains_expected_tools() {
        assert!(TOOL_REGISTRY.contains("extract_text"));
        assert!(TOOL_REGISTRY.contains("compile_to_wiki"));
        assert!(TOOL_REGISTRY.contains("search_knowledge"));
        assert!(TOOL_REGISTRY.contains("rebuild_index"));
        assert!(TOOL_REGISTRY.contains("list_workspaces"));
        assert!(TOOL_REGISTRY.contains("patch_wiki_entry"));
        assert!(TOOL_REGISTRY.contains("apply_wiki_patch"));
        assert!(!TOOL_REGISTRY.contains("nonexistent_tool"));
    }
}
