use crate::tools::json::json_content;
use crate::tools::{ToolContext, attach_compile_sampling, parse_kb_path};
use pdf_core::KnowledgeEngine;
use pdf_core::knowledge::run_incremental_extract;
use pdf_core::management::WorkspaceRegistry;
use pdf_core::management::{
    CompileJobStore, ConfigManager, HealthReporter, build_compile_status_json,
};
use pdf_mcp_contracts::*;
use std::sync::Arc;
use tracing::instrument;

use crate::protocol::Content;
use crate::tools::mcp_extraction::extraction_health_from_ctx;

#[instrument(skip(args))]
pub async fn handle_get_config(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(registry, args)?;
    let mut cm = ConfigManager::new(&kb_path);
    cm.load().map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

    let data: std::collections::HashMap<String, String> = cm.all().clone();
    let result = serde_json::json!({
        "config": data,
        "total_keys": data.len(),
        "config_path": kb_path.join(".rsut_index").join("config.json").to_string_lossy(),
    });
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(args))]
pub async fn handle_set_config(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(registry, args)?;
    let key = args["key"].as_str().ok_or_else(|| anyhow::anyhow!("Missing key"))?;
    let value = args["value"].as_str().ok_or_else(|| anyhow::anyhow!("Missing value"))?;

    let mut cm = ConfigManager::new(&kb_path);
    cm.load().map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;
    cm.set(key, value).map_err(|e| anyhow::anyhow!("Failed to set config: {}", e))?;

    let result = serde_json::json!({
        "status": "success",
        "key": key,
        "value": value,
        "message": format!("Configuration '{}' updated successfully.", key),
    });
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(ctx, args))]
pub async fn handle_get_health_report(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(&ctx.workspace_registry, args)?;
    let reporter = HealthReporter::new(&kb_path);
    let report =
        reporter.report().map_err(|e| anyhow::anyhow!("Failed to generate report: {}", e))?;

    let quality_snapshot =
        pdf_core::management::QualitySnapshotStore::new(&kb_path).read().unwrap_or_default();
    let extraction = extraction_health_from_ctx(ctx);

    json_content(&GetHealthReportOutput {
        total_entries: report.total_entries,
        orphan_count: report.orphan_count,
        contradiction_count: report.contradiction_count,
        broken_link_count: report.broken_link_count,
        index_size_mb: report.index_size_bytes / 1024 / 1024,
        graph_nodes: report.graph_node_count,
        graph_edges: report.graph_edge_count,
        avg_quality_score: format!("{:.1}%", report.avg_quality_score * 100.0),
        domains: report.domains.clone(),
        last_compile: report.last_compile.map(|t| t.to_rfc3339()),
        generated_at: report.generated_at.to_rfc3339(),
        report_text: report.to_string(),
        quality_snapshot: serde_json::to_value(&quality_snapshot)?,
        extraction,
    })
}

#[instrument(skip(ctx, args))]
pub async fn handle_trigger_incremental_compile(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let registry = &ctx.workspace_registry;
    let kb_path = parse_kb_path(registry, args)?;
    let engine = KnowledgeEngine::new(Arc::clone(&ctx.pipeline), &kb_path)?;
    let job_store = CompileJobStore::new(&kb_path);
    let (job_id, result) = run_incremental_extract(&engine, &job_store).await?;

    let mut payload = serde_json::json!({
        "job_id": job_id,
        "pipeline_status": "awaiting_agent",
        "incremental_result": result,
    });
    attach_compile_sampling(ctx, &kb_path, &job_id, &mut payload).await;
    json_content(&TriggerIncrementalCompileOutput { result: payload })
}

#[instrument(skip(args))]
pub async fn handle_get_compile_status(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(registry, args)?;
    let value = build_compile_status_json(&kb_path)
        .map_err(|e| anyhow::anyhow!("Failed to read compile status: {}", e))?;
    Ok(vec![Content::text(serde_json::to_string_pretty(&value)?)])
}

#[instrument(skip(args))]
pub async fn handle_list_quality_issues(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(registry, args)?;
    let wiki_dir = kb_path.join("wiki");
    let severity = args["severity"].as_str();
    let limit = args["limit"].as_u64().unwrap_or(50) as usize;
    let issues = pdf_core::knowledge::list_quality_issues(&wiki_dir, severity, limit)?;
    Ok(vec![Content::text(serde_json::to_string_pretty(&serde_json::json!({
        "issues": issues,
        "count": issues.len(),
    }))?)])
}

#[instrument(skip(args))]
pub async fn handle_fix_suggest(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(registry, args)?;
    let issue_id = args["issue_id"].as_str().ok_or_else(|| anyhow::anyhow!("Missing issue_id"))?;
    let wiki_dir = kb_path.join("wiki");
    let kb_str = kb_path.to_string_lossy();
    let result = pdf_core::knowledge::fix_suggest(&wiki_dir, &kb_str, issue_id)?;
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(args))]
pub async fn handle_apply_quality_gate(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(registry, args)?;
    let gate = pdf_core::knowledge::apply_publish_gate(&kb_path)?;
    let _ = pdf_core::management::refresh_quality_snapshot(&kb_path);
    if let Some(job_id) = args["job_id"].as_str() {
        let store = CompileJobStore::new(&kb_path);
        if let Ok(mut job) = store.load_job(job_id) {
            job.stats.entries_blocked = gate.blocked_count;
            let _ = store.write_job(&job);
        }
    }
    Ok(vec![Content::text(serde_json::to_string_pretty(&gate)?)])
}

#[instrument]
pub async fn handle_show_wiki_browser() -> anyhow::Result<Vec<crate::protocol::Content>> {
    Ok(vec![Content::text(serde_json::json!({
        "type": "resource",
        "uri": "ui://wiki/browser",
        "message": "Wiki browser opened. The client should render ui://wiki/browser as an MCP App iframe."
    }).to_string())])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolContext;

    use tempfile::TempDir;

    fn create_test_context() -> ToolContext {
        crate::tools::create_test_tool_context()
    }

    #[test]
    fn test_management_tool_names_in_manifest() {
        let names: std::collections::HashSet<_> =
            pdf_mcp_contracts::all_tool_specs().into_iter().map(|s| s.name).collect();
        for name in ["get_health_report", "get_compile_status", "show_wiki_browser"] {
            assert!(names.contains(name));
        }
    }

    #[tokio::test]
    async fn test_get_config_default_kb() {
        let args = serde_json::json!({});

        let registry = create_test_context().workspace_registry;
        let result = handle_get_config(&registry, &args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_config_default_kb() {
        let args = serde_json::json!({
            "key": "test_key",
            "value": "test_value"
        });

        let registry = create_test_context().workspace_registry;
        let result = handle_set_config(&registry, &args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_config_missing_key() {
        let args = serde_json::json!({
            "knowledge_base": "/tmp/test_kb",
            "value": "test_value"
        });

        let registry = create_test_context().workspace_registry;
        let result = handle_set_config(&registry, &args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing key"));
    }

    #[tokio::test]
    async fn test_set_config_missing_value() {
        let args = serde_json::json!({
            "knowledge_base": "/tmp/test_kb",
            "key": "test_key"
        });

        let registry = create_test_context().workspace_registry;
        let result = handle_set_config(&registry, &args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing value"));
    }

    #[tokio::test]
    async fn test_get_health_report_default_kb() {
        let args = serde_json::json!({});

        let ctx = create_test_context();
        let result = handle_get_health_report(&ctx, &args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_trigger_incremental_compile_default_kb() {
        let ctx = create_test_context();
        let args = serde_json::json!({});

        let result = handle_trigger_incremental_compile(&ctx, &args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_compile_status_default_kb() {
        let args = serde_json::json!({});

        let registry = create_test_context().workspace_registry;
        let result = handle_get_compile_status(&registry, &args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_wiki_browser() {
        let result = handle_show_wiki_browser().await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.len(), 1);

        let parsed: serde_json::Value =
            serde_json::from_str(&content[0].text).expect("Should be valid JSON");
        assert_eq!(parsed["type"], "resource");
        assert_eq!(parsed["uri"], "ui://wiki/browser");
    }

    #[tokio::test]
    async fn test_get_config_with_valid_kb() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let kb_path = temp_dir.path();

        let index_dir = kb_path.join(".rsut_index");
        tokio::fs::create_dir_all(&index_dir).await.expect("Failed to create index dir");

        let args = serde_json::json!({
            "knowledge_base": kb_path.to_str().unwrap()
        });

        let registry = create_test_context().workspace_registry;
        let result = handle_get_config(&registry, &args).await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.len(), 1);

        let parsed: serde_json::Value =
            serde_json::from_str(&content[0].text).expect("Should be valid JSON");
        assert!(parsed.get("config").is_some());
    }

    #[tokio::test]
    async fn test_set_config_with_valid_kb() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let kb_path = temp_dir.path();

        let index_dir = kb_path.join(".rsut_index");
        tokio::fs::create_dir_all(&index_dir).await.expect("Failed to create index dir");

        let args = serde_json::json!({
            "knowledge_base": kb_path.to_str().unwrap(),
            "key": "test_key",
            "value": "test_value"
        });

        let registry = create_test_context().workspace_registry;
        let result = handle_set_config(&registry, &args).await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.len(), 1);

        let parsed: serde_json::Value =
            serde_json::from_str(&content[0].text).expect("Should be valid JSON");
        assert_eq!(parsed["status"], "success");
        assert_eq!(parsed["key"], "test_key");
        assert_eq!(parsed["value"], "test_value");
    }

    #[tokio::test]
    async fn test_get_compile_status_no_prior_compile() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let kb_path = temp_dir.path();

        let args = serde_json::json!({
            "knowledge_base": kb_path.to_str().unwrap()
        });

        let registry = create_test_context().workspace_registry;
        let result = handle_get_compile_status(&registry, &args).await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.len(), 1);

        let parsed: serde_json::Value =
            serde_json::from_str(&content[0].text).expect("Should be valid JSON");
        let status = parsed.get("status").unwrap_or(&parsed);
        assert_eq!(status["running"], false);
        assert_eq!(parsed["last_started"], serde_json::Value::Null);
        assert!(parsed.get("history").and_then(|h| h.as_array()).is_some());
    }
}
