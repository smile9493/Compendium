mod extract;
mod index;
mod knowledge;
mod management;
mod post_compile;
mod resources;

pub use extract::*;
pub use index::*;
pub use knowledge::*;
pub use management::*;
pub use resources::*;

use crate::protocol::{Content, ToolDefinition};
use crate::upload::UploadStore;
use pdf_core::{McpPdfPipeline, PathValidationConfig};
use std::sync::Arc;

pub fn default_path_config() -> PathValidationConfig {
    PathValidationConfig {
        require_absolute: true,
        allow_traversal: false,
        base_dir: None,
    }
}

pub struct ToolContext {
    pub pipeline: Arc<McpPdfPipeline>,
    pub path_config: PathValidationConfig,
    pub upload_store: Option<Arc<UploadStore>>,
}

impl ToolContext {
    pub fn new(pipeline: Arc<McpPdfPipeline>) -> Self {
        Self {
            pipeline,
            path_config: default_path_config(),
            upload_store: None,
        }
    }

    pub fn new_with_upload_store(
        pipeline: Arc<McpPdfPipeline>,
        upload_store: Option<Arc<UploadStore>>,
    ) -> Self {
        Self {
            pipeline,
            path_config: default_path_config(),
            upload_store,
        }
    }
}

pub fn all_tool_definitions() -> Vec<ToolDefinition> {
    let mut tools = Vec::with_capacity(30);
    tools.extend(extract_tool_definitions());
    tools.extend(knowledge_tool_definitions());
    tools.extend(index_tool_definitions());
    tools.extend(management_tool_definitions());
    tools
}

#[tracing::instrument(skip(ctx, args), fields(tool = %tool_name))]
pub async fn dispatch_tool(
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
        "compile_to_wiki" => handle_compile_to_wiki(ctx, args).await,
        "compile_uploaded_pdf" => handle_compile_uploaded_pdf(ctx, args).await,
        "incremental_compile" => handle_incremental_compile(ctx, args).await,
        "search_knowledge" => handle_search_knowledge(args).await,
        "rebuild_index" => handle_rebuild_index(args).await,
        "get_entry_context" => handle_get_entry_context(args).await,
        "get_agent_context" => handle_get_agent_context(args).await,
        "preview_wiki_patch" => handle_preview_wiki_patch(args).await,
        "patch_wiki_entry" => handle_patch_wiki_entry(args).await,
        "find_orphans" => handle_find_orphans(args).await,
        "suggest_links" => handle_suggest_links(args).await,
        "export_concept_map" => handle_export_concept_map(args).await,
        "check_quality" => handle_check_quality(args).await,
        "micro_compile" => handle_micro_compile(ctx, args).await,
        "aggregate_entries" => handle_aggregate_entries(ctx, args).await,
        "hypothesis_test" => handle_hypothesis_test(ctx, args).await,
        "recompile_entry" => handle_recompile_entry(ctx, args).await,
        "save_wiki_entry" => handle_save_wiki_entry(ctx, args).await,
        "get_config" => handle_get_config(args).await,
        "set_config" => handle_set_config(args).await,
        "get_health_report" => handle_get_health_report(args).await,
        "trigger_incremental_compile" => handle_trigger_incremental_compile(ctx, args).await,
        "get_compile_status" => handle_get_compile_status(args).await,
        "show_wiki_browser" => handle_show_wiki_browser().await,
        _ => Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
    }
}

const DEFAULT_KNOWLEDGE_BASE: &str = "/app/kb";

fn parse_kb_path(args: &serde_json::Value) -> anyhow::Result<std::path::PathBuf> {
    let kb = args["knowledge_base"]
        .as_str()
        .map(String::from)
        .or_else(|| std::env::var("KNOWLEDGE_BASE_PATH").ok())
        .unwrap_or_else(|| DEFAULT_KNOWLEDGE_BASE.to_string());
    Ok(std::path::PathBuf::from(kb))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdf_core::{McpPdfPipeline, ServerConfig};
    use std::sync::Arc;

    fn create_test_context() -> ToolContext {
        let config = ServerConfig::from_env().unwrap_or_default();
        let pipeline = Arc::new(McpPdfPipeline::new(&config).expect("Failed to create pipeline"));
        ToolContext::new(pipeline)
    }

    #[test]
    fn test_all_tool_definitions_count() {
        let tools = all_tool_definitions();
        assert_eq!(tools.len(), 30, "Expected 30 tools total");
    }

    #[test]
    fn test_all_tool_definitions_unique_names() {
        let tools = all_tool_definitions();
        let names: std::collections::HashSet<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names.len(), 30, "All tool names should be unique");
    }

    #[test]
    fn test_all_tool_definitions_have_required_fields() {
        let tools = all_tool_definitions();
        for tool in &tools {
            assert!(!tool.name.is_empty(), "Tool name should not be empty");
            assert!(!tool.description.is_empty(), "Tool description should not be empty");
            assert!(tool.input_schema.is_object(), "Tool input_schema should be an object");
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
        let args = serde_json::json!({
            "knowledge_base": "/tmp/test_kb"
        });
        let result = parse_kb_path(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_str().unwrap(), "/tmp/test_kb");
    }

    #[test]
    fn test_parse_kb_path_missing() {
        let args = serde_json::json!({});
        let result = parse_kb_path(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_str().unwrap(), "/app/kb");
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
