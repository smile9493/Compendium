use crate::protocol::{Content, ToolDefinition};
use crate::tools::{parse_kb_path, ToolContext};
use pdf_core::management::{ConfigManager, HealthReporter};
use pdf_core::KnowledgeEngine;
use std::sync::Arc;
use tracing::instrument;

pub fn management_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "get_config".to_string(),
            description: "Get current runtime configuration for a knowledge base. Returns all key-value pairs from the managed config file.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": {
                        "type": "string",
                        "description": "Knowledge base path (default: /app/kb or KNOWLEDGE_BASE_PATH env)"
                    }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "set_config".to_string(),
            description: "Set a runtime configuration value for a knowledge base. Persists atomically via write-tmp + rename.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": {
                        "type": "string",
                        "description": "Knowledge base path (default: /app/kb or KNOWLEDGE_BASE_PATH env)"
                    },
                    "key": {
                        "type": "string",
                        "description": "Configuration key (e.g. 'vlm_api_key', 'extract_mode')"
                    },
                    "value": {
                        "type": "string",
                        "description": "Configuration value"
                    }
                },
                "required": ["key", "value"]
            }),
        },
        ToolDefinition {
            name: "get_health_report".to_string(),
            description: "Get a comprehensive health report for the knowledge base: entry count, orphan count, contradiction count, index size, graph topology, quality score, and last compile time.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": {
                        "type": "string",
                        "description": "Knowledge base path (default: /app/kb or KNOWLEDGE_BASE_PATH env)"
                    }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "trigger_incremental_compile".to_string(),
            description: "Manually trigger an incremental compilation of the knowledge base. Scans raw/ for changed PDFs and recompiles only those that need it.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": {
                        "type": "string",
                        "description": "Knowledge base path (default: /app/kb or KNOWLEDGE_BASE_PATH env)"
                    }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "get_compile_status".to_string(),
            description: "Get the current compile status: whether a compile is running, last start/finish times, duration, outcome, and recent compile history.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": {
                        "type": "string",
                        "description": "Knowledge base path (default: /app/kb or KNOWLEDGE_BASE_PATH env)"
                    }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "show_wiki_browser".to_string(),
            description: "Open the interactive wiki browser as an MCP App. Returns a resource reference to ui://wiki/browser which the client renders as an iframe. The browser provides tree navigation, full-text search, concept maps, and backlinks for the knowledge base.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
    ]
}

#[instrument(skip(args))]
pub async fn handle_get_config(args: &serde_json::Value) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(args)?;
    let mut cm = ConfigManager::new(&kb_path);
    cm.load()
        .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

    let data: std::collections::HashMap<String, String> = cm.all().clone();
    let result = serde_json::json!({
        "config": data,
        "total_keys": data.len(),
        "config_path": kb_path.join(".rsut_index").join("config.json").to_string_lossy(),
    });
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(args))]
pub async fn handle_set_config(args: &serde_json::Value) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(args)?;
    let key = args["key"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing key"))?;
    let value = args["value"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing value"))?;

    let mut cm = ConfigManager::new(&kb_path);
    cm.load()
        .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;
    cm.set(key, value)
        .map_err(|e| anyhow::anyhow!("Failed to set config: {}", e))?;

    let result = serde_json::json!({
        "status": "success",
        "key": key,
        "value": value,
        "message": format!("Configuration '{}' updated successfully.", key),
    });
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(args))]
pub async fn handle_get_health_report(args: &serde_json::Value) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(args)?;
    let reporter = HealthReporter::new(&kb_path);
    let report = reporter
        .report()
        .map_err(|e| anyhow::anyhow!("Failed to generate report: {}", e))?;

    let result = serde_json::json!({
        "total_entries": report.total_entries,
        "orphan_count": report.orphan_count,
        "contradiction_count": report.contradiction_count,
        "broken_link_count": report.broken_link_count,
        "index_size_mb": report.index_size_bytes / 1024 / 1024,
        "graph_nodes": report.graph_node_count,
        "graph_edges": report.graph_edge_count,
        "avg_quality_score": format!("{:.1}%", report.avg_quality_score * 100.0),
        "domains": report.domains,
        "last_compile": report.last_compile.map(|t| t.to_rfc3339()),
        "generated_at": report.generated_at.to_rfc3339(),
        "report_text": report.to_string(),
    });
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(ctx, args))]
pub async fn handle_trigger_incremental_compile(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(args)?;
    let engine = KnowledgeEngine::new(Arc::clone(&ctx.pipeline), &kb_path)?;
    let raw_dir = engine.raw_dir();
    let result = engine.incremental_compile(&raw_dir).await?;

    let status_path = kb_path.join(".rsut_index").join("compile_status.json");
    if let Some(parent) = status_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let status = serde_json::json!({
        "running": false,
        "last_finished": chrono::Utc::now().to_rfc3339(),
        "last_outcome": "success",
        "last_duration_ms": 0,
        "entries_compiled": result.compiled,
        "entries_skipped": result.skipped,
        "message": format!("Incremental compile: {} compiled, {} skipped", result.compiled, result.skipped),
    });
    let _ = tokio::fs::write(
        &status_path,
        serde_json::to_string_pretty(&status).unwrap_or_default(),
    )
    .await;

    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(args))]
pub async fn handle_get_compile_status(args: &serde_json::Value) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(args)?;
    let status_path = kb_path.join(".rsut_index").join("compile_status.json");

    if !status_path.exists() {
        let result = serde_json::json!({
            "running": false,
            "last_started": null,
            "last_finished": null,
            "last_duration_ms": null,
            "last_outcome": null,
            "message": "No compile has been performed yet.",
            "history": [],
        });
        return Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)]);
    }

    let content = tokio::fs::read_to_string(&status_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read compile status: {}", e))?;
    let status: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse compile status: {}", e))?;

    Ok(vec![Content::text(serde_json::to_string_pretty(&status)?)])
}

#[instrument]
pub async fn handle_show_wiki_browser() -> anyhow::Result<Vec<Content>> {
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
    use pdf_core::{McpPdfPipeline, ServerConfig};
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_context() -> ToolContext {
        let config = ServerConfig::from_env().unwrap_or_default();
        let pipeline = Arc::new(McpPdfPipeline::new(&config).expect("Failed to create pipeline"));
        ToolContext::new(pipeline)
    }

    #[test]
    fn test_management_tool_definitions() {
        let defs = management_tool_definitions();
        assert_eq!(defs.len(), 6);
        
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"get_config"));
        assert!(names.contains(&"set_config"));
        assert!(names.contains(&"get_health_report"));
        assert!(names.contains(&"trigger_incremental_compile"));
        assert!(names.contains(&"get_compile_status"));
        assert!(names.contains(&"show_wiki_browser"));
    }

    #[tokio::test]
    async fn test_get_config_missing_kb() {
        let args = serde_json::json!({});
        
        let result = handle_get_config(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing knowledge_base"));
    }

    #[tokio::test]
    async fn test_set_config_missing_kb() {
        let args = serde_json::json!({
            "key": "test_key",
            "value": "test_value"
        });
        
        let result = handle_set_config(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing knowledge_base"));
    }

    #[tokio::test]
    async fn test_set_config_missing_key() {
        let args = serde_json::json!({
            "knowledge_base": "/tmp/test_kb",
            "value": "test_value"
        });
        
        let result = handle_set_config(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing key"));
    }

    #[tokio::test]
    async fn test_set_config_missing_value() {
        let args = serde_json::json!({
            "knowledge_base": "/tmp/test_kb",
            "key": "test_key"
        });
        
        let result = handle_set_config(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing value"));
    }

    #[tokio::test]
    async fn test_get_health_report_missing_kb() {
        let args = serde_json::json!({});
        
        let result = handle_get_health_report(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing knowledge_base"));
    }

    #[tokio::test]
    async fn test_trigger_incremental_compile_missing_kb() {
        let ctx = create_test_context();
        let args = serde_json::json!({});
        
        let result = handle_trigger_incremental_compile(&ctx, &args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing knowledge_base"));
    }

    #[tokio::test]
    async fn test_get_compile_status_missing_kb() {
        let args = serde_json::json!({});
        
        let result = handle_get_compile_status(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing knowledge_base"));
    }

    #[tokio::test]
    async fn test_show_wiki_browser() {
        let result = handle_show_wiki_browser().await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.len(), 1);
        
        let parsed: serde_json::Value = serde_json::from_str(&content[0].text).expect("Should be valid JSON");
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
        
        let result = handle_get_config(&args).await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.len(), 1);
        
        let parsed: serde_json::Value = serde_json::from_str(&content[0].text).expect("Should be valid JSON");
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
        
        let result = handle_set_config(&args).await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.len(), 1);
        
        let parsed: serde_json::Value = serde_json::from_str(&content[0].text).expect("Should be valid JSON");
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
        
        let result = handle_get_compile_status(&args).await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.len(), 1);
        
        let parsed: serde_json::Value = serde_json::from_str(&content[0].text).expect("Should be valid JSON");
        assert_eq!(parsed["running"], false);
        assert_eq!(parsed["last_started"], serde_json::Value::Null);
    }
}
