use std::fs;

use crate::protocol::{Content, ToolDefinition};
use crate::tools::parse_kb_path;
use pdf_core::knowledge::patch::{apply_patch, preview_patch, WikiPatchRequest};
use pdf_core::knowledge::quality::build_next_actions;
use pdf_core::knowledge::{
    graph, rebuild_all, reindex_entry, search_with_mode, wiki_dir, KnowledgeEntry, SearchMode,
};
use pdf_core::management::WorkspaceRegistry;
use tracing::instrument;

pub fn index_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "search_knowledge".to_string(),
            description: "Search wiki entries. Default mode `hybrid` fuses Tantivy CJK (jieba) full-text with TF-IDF vector similarity via RRF — not ONNX.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": {
                        "type": "string",
                        "description": "Knowledge base path (default: /app/kb or KNOWLEDGE_BASE_PATH env)"
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum number of results (default: 10)"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["keyword", "semantic", "hybrid"],
                        "description": "keyword=Tantivy only, semantic=TF-IDF vectors, hybrid=RRF merge (default)"
                    }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "rebuild_index".to_string(),
            description: "Rebuild all indexes (Tantivy + petgraph + TF-IDF vectors) from wiki Markdown files.".to_string(),
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
        ToolDefinition {
            name: "get_agent_context".to_string(),
            description: "Token-efficient context bundle for an entry: center body, graph neighbors, and hybrid-related snippets.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": { "type": "string" },
                    "entry_path": { "type": "string", "description": "Relative path within wiki/ (e.g. IT/concept.md)" },
                    "hops": { "type": "number", "description": "Graph neighbor hops (default: 2)" },
                    "max_body_chars": { "type": "number", "description": "Max chars for center body (default: 4000)" },
                    "related_limit": { "type": "number", "description": "Hybrid search hits for related snippets (default: 3)" }
                },
                "required": ["entry_path"]
            }),
        },
        ToolDefinition {
            name: "preview_wiki_patch".to_string(),
            description: "Preview a structured patch on a wiki entry (unified diff, no write).".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": { "type": "string" },
                    "entry_path": { "type": "string" },
                    "operations": {
                        "type": "array",
                        "description": "replace_section | replace_front_matter | search_replace ops"
                    }
                },
                "required": ["entry_path", "operations"]
            }),
        },
        ToolDefinition {
            name: "patch_wiki_entry".to_string(),
            description: "Apply a structured patch to a wiki entry, then reindex. Prefer over save_wiki_entry for small edits.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": { "type": "string" },
                    "entry_path": { "type": "string" },
                    "operations": { "type": "array" }
                },
                "required": ["entry_path", "operations"]
            }),
        },
    ]
}

#[instrument(skip(args))]
pub async fn handle_search_knowledge(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(registry, args)?;
    let query = args["query"].as_str().ok_or_else(|| anyhow::anyhow!("Missing query"))?;
    let limit = args["limit"].as_u64().unwrap_or(10) as usize;
    let mode = args["mode"].as_str().map(SearchMode::parse).unwrap_or(SearchMode::Hybrid);

    let hits = search_with_mode(&kb_path, query, limit, mode)?;
    Ok(vec![Content::text(serde_json::to_string_pretty(&serde_json::json!({
        "mode": format!("{:?}", mode).to_lowercase(),
        "results": hits,
        "total": hits.len()
    }))?)])
}

#[instrument(skip(args))]
pub async fn handle_rebuild_index(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(registry, args)?;
    let stats = rebuild_all(&kb_path)?;

    let result = serde_json::json!({
        "status": "success",
        "fulltext_entries_indexed": stats.fulltext_entries_indexed,
        "graph_nodes": stats.graph_nodes,
        "graph_edges": stats.graph_edges,
        "vector_entries_indexed": stats.vector_entries_indexed,
        "message": "All indexes rebuilt from wiki/ files."
    });
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(args))]
pub async fn handle_get_entry_context(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(registry, args)?;
    let entry_path =
        args["entry_path"].as_str().ok_or_else(|| anyhow::anyhow!("Missing entry_path"))?;
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
pub async fn handle_find_orphans(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(registry, args)?;

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
pub async fn handle_suggest_links(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(registry, args)?;
    let entry_path =
        args["entry_path"].as_str().ok_or_else(|| anyhow::anyhow!("Missing entry_path"))?;
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
pub async fn handle_export_concept_map(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(registry, args)?;
    let entry_path =
        args["entry_path"].as_str().ok_or_else(|| anyhow::anyhow!("Missing entry_path"))?;
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
pub async fn handle_check_quality(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(registry, args)?;
    let wiki_dir = kb_path.join("wiki");

    let report = pdf_core::knowledge::quality::analyze_wiki(&wiki_dir)?;
    let kb_str = kb_path.to_string_lossy();
    let next_actions = build_next_actions(&report, &kb_str);
    let issues = pdf_core::knowledge::list_quality_issues(&wiki_dir, None, 50)?;

    let result = serde_json::json!({
        "total_entries": report.total_entries,
        "avg_quality_score": format!("{:.1}%", report.avg_quality_score * 100.0),
        "domains": report.domains.iter().collect::<Vec<_>>(),
        "issues_count": report.issues.len(),
        "issues": issues,
        "orphan_count": report.orphan_entries.len(),
        "broken_links_count": report.broken_links.len(),
        "drift_pairs_count": report.drift_pairs.len(),
        "report_markdown": report.to_markdown(),
        "has_errors": report.has_errors(),
        "has_warnings": report.has_warnings(),
        "next_actions": next_actions
    });
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(args))]
pub async fn handle_get_agent_context(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(registry, args)?;
    let entry_path =
        args["entry_path"].as_str().ok_or_else(|| anyhow::anyhow!("Missing entry_path"))?;
    let hops = args["hops"].as_u64().unwrap_or(2) as u32;
    let max_body_chars = args["max_body_chars"].as_u64().unwrap_or(4000) as usize;
    let related_limit = args["related_limit"].as_u64().unwrap_or(3) as usize;

    let rel = entry_path.trim_start_matches("wiki/").trim_start_matches('/');
    let full_path = wiki_dir(&kb_path).join(rel);
    let content = fs::read_to_string(&full_path)
        .map_err(|e| anyhow::anyhow!("Failed to read entry: {}", e))?;

    let entry = KnowledgeEntry::from_markdown(&content)
        .ok_or_else(|| anyhow::anyhow!("Invalid front matter in {}", rel))?;
    let body = content.split("---").nth(2).unwrap_or("");
    let body_truncated = truncate_chars(body, max_body_chars);

    let graph = graph(&kb_path)?;
    let neighbors = graph.get_neighbors(rel, hops);

    let related_query = format!("{} {}", entry.title, entry.tags.join(" "));
    let related_hits =
        search_with_mode(&kb_path, &related_query, related_limit, SearchMode::Hybrid)?
            .into_iter()
            .filter(|h| h.path != rel)
            .map(|h| {
                serde_json::json!({
                    "path": h.path,
                    "title": h.title,
                    "score": h.score,
                    "snippet": h.snippet
                })
            })
            .collect::<Vec<_>>();

    let char_count =
        body_truncated.chars().count() + neighbors.len() * 200 + related_hits.len() * 150;
    let token_estimate = char_count / 4;

    let result = serde_json::json!({
        "entry_path": rel,
        "center": {
            "title": entry.title,
            "domain": entry.domain,
            "tags": entry.tags,
            "front_matter": {
                "related": entry.related,
                "contradictions": entry.contradictions,
                "quality_score": entry.quality_score
            },
            "body": body_truncated
        },
        "neighbors": neighbors,
        "related_snippets": related_hits,
        "token_estimate": token_estimate
    });
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}

fn parse_patch_request(args: &serde_json::Value) -> anyhow::Result<WikiPatchRequest> {
    let entry_path = args["entry_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing entry_path"))?
        .to_string();
    let ops = args.get("operations").ok_or_else(|| anyhow::anyhow!("Missing operations"))?;
    let operations: Vec<pdf_core::knowledge::patch::PatchOp> = serde_json::from_value(ops.clone())
        .map_err(|e| anyhow::anyhow!("Invalid operations: {}", e))?;
    Ok(WikiPatchRequest { entry_path, operations })
}

#[instrument(skip(args))]
pub async fn handle_preview_wiki_patch(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(registry, args)?;
    let request = parse_patch_request(args)?;
    let result = preview_patch(&kb_path, &request)?;
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(args))]
pub async fn handle_patch_wiki_entry(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(registry, args)?;
    let request = parse_patch_request(args)?;
    let result = apply_patch(&kb_path, &request)?;
    reindex_entry(&kb_path, &request.entry_path)?;
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_tool_definitions() {
        let defs = index_tool_definitions();
        assert_eq!(defs.len(), 10);

        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"search_knowledge"));
        assert!(names.contains(&"rebuild_index"));
        assert!(names.contains(&"get_entry_context"));
        assert!(names.contains(&"find_orphans"));
        assert!(names.contains(&"suggest_links"));
        assert!(names.contains(&"export_concept_map"));
        assert!(names.contains(&"check_quality"));
        assert!(names.contains(&"get_agent_context"));
        assert!(names.contains(&"preview_wiki_patch"));
        assert!(names.contains(&"patch_wiki_entry"));
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
