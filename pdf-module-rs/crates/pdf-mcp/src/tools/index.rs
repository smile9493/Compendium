use crate::protocol::{Content, ToolDefinition};
use crate::tools::parse_kb_path;
use pdf_core::knowledge::{graph, rebuild_all, search};
use tracing::instrument;

pub fn index_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "search_knowledge".to_string(),
            description: "Full-text search across all wiki entries using Tantivy. Returns ranked results with snippets.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": {
                        "type": "string",
                        "description": "Knowledge base path (default: /app/kb or KNOWLEDGE_BASE_PATH env)"
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query (supports keywords, phrases, boolean)"
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum number of results (default: 10)"
                    }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "rebuild_index".to_string(),
            description: "Rebuild all indexes (Tantivy fulltext + petgraph link graph) from wiki Markdown files. Use after bulk changes or for recovery.".to_string(),
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
            name: "get_entry_context".to_string(),
            description: "Get N-hop neighbors of a knowledge entry (by link relationships, tag co-occurrence). Returns connected entries for context expansion.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": {
                        "type": "string",
                        "description": "Knowledge base path (default: /app/kb or KNOWLEDGE_BASE_PATH env)"
                    },
                    "entry_path": {
                        "type": "string",
                        "description": "Relative path of the entry within wiki/ (e.g. 'it/http2_multiplex.md')"
                    },
                    "hops": {
                        "type": "number",
                        "description": "Maximum number of hops to traverse (default: 2)"
                    }
                },
                "required": ["entry_path"]
            }),
        },
        ToolDefinition {
            name: "find_orphans".to_string(),
            description: "Find knowledge entries with no incoming or outgoing related/contradiction links. These are candidates for integration.".to_string(),
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
            name: "suggest_links".to_string(),
            description: "Suggest potential links for a knowledge entry based on tag similarity (Jaccard index). Helps discover hidden connections.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": {
                        "type": "string",
                        "description": "Knowledge base path (default: /app/kb or KNOWLEDGE_BASE_PATH env)"
                    },
                    "entry_path": {
                        "type": "string",
                        "description": "Relative path of the entry within wiki/"
                    },
                    "top_k": {
                        "type": "number",
                        "description": "Maximum number of suggestions (default: 10)"
                    }
                },
                "required": ["entry_path"]
            }),
        },
        ToolDefinition {
            name: "export_concept_map".to_string(),
            description: "Export a local concept map around an entry as Mermaid.js text. Shows relationships within N hops for visualization.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": {
                        "type": "string",
                        "description": "Knowledge base path (default: /app/kb or KNOWLEDGE_BASE_PATH env)"
                    },
                    "entry_path": {
                        "type": "string",
                        "description": "Relative path of the center entry within wiki/"
                    },
                    "depth": {
                        "type": "number",
                        "description": "Number of hops to include (default: 2)"
                    }
                },
                "required": ["entry_path"]
            }),
        },
        ToolDefinition {
            name: "check_quality".to_string(),
            description: "Analyze wiki quality: detect missing tags, orphan entries, broken links, style issues. Returns a comprehensive report.".to_string(),
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
    ]
}

#[instrument(skip(args))]
pub async fn handle_search_knowledge(args: &serde_json::Value) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(args)?;
    let query = args["query"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing query"))?;
    let limit = args["limit"].as_u64().unwrap_or(10) as usize;

    let hits = search(&kb_path, query, limit)?;
    Ok(vec![Content::text(serde_json::to_string_pretty(&hits)?)])
}

#[instrument(skip(args))]
pub async fn handle_rebuild_index(args: &serde_json::Value) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(args)?;
    let stats = rebuild_all(&kb_path)?;

    let result = serde_json::json!({
        "status": "success",
        "fulltext_entries_indexed": stats.fulltext_entries_indexed,
        "graph_nodes": stats.graph_nodes,
        "graph_edges": stats.graph_edges,
        "message": "All indexes rebuilt from wiki/ files."
    });
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(args))]
pub async fn handle_get_entry_context(args: &serde_json::Value) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(args)?;
    let entry_path = args["entry_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing entry_path"))?;
    let hops = args["hops"].as_u64().unwrap_or(2) as u32;

    let graph = graph(&kb_path)?;
    let neighbors = graph.get_neighbors(entry_path, hops);

    let result = serde_json::json!({
        "entry": entry_path,
        "hops": hops,
        "neighbors": neighbors,
        "total": neighbors.len()
    });
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(args))]
pub async fn handle_find_orphans(args: &serde_json::Value) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(args)?;

    let graph = graph(&kb_path)?;
    let orphans = graph.find_orphans();

    let result = serde_json::json!({
        "orphan_count": orphans.len(),
        "entries": orphans,
        "message": if orphans.is_empty() {
            "No orphan entries found. All entries have at least one link.".to_string()
        } else {
            format!("{} entries have no links. Consider integrating them.", orphans.len())
        }
    });
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(args))]
pub async fn handle_suggest_links(args: &serde_json::Value) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(args)?;
    let entry_path = args["entry_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing entry_path"))?;
    let top_k = args["top_k"].as_u64().unwrap_or(10) as usize;

    let graph = graph(&kb_path)?;
    let suggestions = graph.suggest_links(entry_path, top_k);

    let result = serde_json::json!({
        "entry": entry_path,
        "suggestions": suggestions,
        "total": suggestions.len()
    });
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(args))]
pub async fn handle_export_concept_map(args: &serde_json::Value) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(args)?;
    let entry_path = args["entry_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing entry_path"))?;
    let depth = args["depth"].as_u64().unwrap_or(2) as u32;

    let graph = graph(&kb_path)?;
    let mermaid = graph.export_concept_map(entry_path, depth);

    let result = serde_json::json!({
        "entry": entry_path,
        "depth": depth,
        "mermaid": mermaid,
        "usage": "Paste the mermaid field into any Mermaid.js renderer (e.g. Obsidian, GitHub, mermaid.live)"
    });
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(args))]
pub async fn handle_check_quality(args: &serde_json::Value) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(args)?;
    let wiki_dir = kb_path.join("wiki");

    let report = pdf_core::knowledge::quality::analyze_wiki(&wiki_dir)?;

    let result = serde_json::json!({
        "total_entries": report.total_entries,
        "avg_quality_score": format!("{:.1}%", report.avg_quality_score * 100.0),
        "domains": report.domains.iter().collect::<Vec<_>>(),
        "issues_count": report.issues.len(),
        "orphan_count": report.orphan_entries.len(),
        "broken_links_count": report.broken_links.len(),
        "report_markdown": report.to_markdown(),
        "has_errors": report.has_errors(),
        "has_warnings": report.has_warnings()
    });
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_tool_definitions() {
        let defs = index_tool_definitions();
        assert_eq!(defs.len(), 7);
        
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"search_knowledge"));
        assert!(names.contains(&"rebuild_index"));
        assert!(names.contains(&"get_entry_context"));
        assert!(names.contains(&"find_orphans"));
        assert!(names.contains(&"suggest_links"));
        assert!(names.contains(&"export_concept_map"));
        assert!(names.contains(&"check_quality"));
    }

    #[tokio::test]
    async fn test_search_knowledge_missing_query() {
        let args = serde_json::json!({});
        
        let result = handle_search_knowledge(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing query"));
    }

    #[tokio::test]
    async fn test_get_entry_context_missing_entry_path() {
        let args = serde_json::json!({});
        
        let result = handle_get_entry_context(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing entry_path"));
    }

    #[tokio::test]
    async fn test_suggest_links_missing_entry_path() {
        let args = serde_json::json!({});
        
        let result = handle_suggest_links(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing entry_path"));
    }

    #[tokio::test]
    async fn test_export_concept_map_missing_entry_path() {
        let args = serde_json::json!({});
        
        let result = handle_export_concept_map(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing entry_path"));
    }
}
