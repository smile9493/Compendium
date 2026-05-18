use std::fs;

use crate::tools::json::json_content;
use crate::tools::parse_kb_path;
use pdf_core::knowledge::patch::{apply_patch, preview_patch, WikiPatchRequest};
use pdf_core::knowledge::quality::build_next_actions;
use pdf_core::knowledge::{
    graph, rebuild_all, reindex_entry, search_with_options, wiki_dir, KnowledgeEntry, SearchMode,
    SearchOptions,
};
use pdf_core::management::WorkspaceRegistry;
use pdf_core::management::{build_compile_status_json, CompileJobStore, QualitySnapshotStore};
use pdf_mcp_contracts::{
    AgentCenterOut, CheckQualityOutput, ExportConceptMapOutput, FindOrphansOutput,
    GetAgentContextOutput, GetCompilationContextInput, GetCompilationContextOutput,
    GetEntryContextOutput, PatchWikiEntryOutput, PromptExcerptOut, RebuildIndexOutput,
    RelatedSnippetOut, SearchHitOut, SearchKnowledgeOutput, SearchMetaOut, SuggestLinksOutput,
};
use tracing::instrument;

use crate::protocol::Content;

#[instrument(skip(ctx, args))]
pub async fn handle_search_knowledge(
    ctx: &crate::tools::ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(&ctx.workspace_registry, args)?;
    let query = args["query"].as_str().ok_or_else(|| anyhow::anyhow!("Missing query"))?;
    let limit = args["limit"].as_u64().unwrap_or(10) as usize;
    let mode = args["mode"].as_str().map(SearchMode::parse).unwrap_or(SearchMode::Hybrid);

    let mut opts = SearchOptions::for_api();
    if let Some(d) = args["domain"].as_str() {
        opts.domain = Some(d.to_string());
    }
    let response = ctx.index_cache.search(&kb_path, query, limit, mode, opts)?;
    Ok(vec![Content::text(serde_json::to_string_pretty(&serde_json::json!({
        "mode": response.meta.mode,
        "meta": response.meta,
        "results": response.hits,
        "total": response.hits.len()
    }))?)])
}

#[instrument(skip(ctx, args))]
pub async fn handle_rebuild_index(
    ctx: &crate::tools::ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(&ctx.workspace_registry, args)?;
    let stats = rebuild_all(&kb_path)?;
    ctx.index_cache.invalidate(&kb_path);

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
) -> anyhow::Result<Vec<crate::protocol::Content>> {
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
) -> anyhow::Result<Vec<crate::protocol::Content>> {
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
) -> anyhow::Result<Vec<crate::protocol::Content>> {
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
) -> anyhow::Result<Vec<crate::protocol::Content>> {
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
) -> anyhow::Result<Vec<crate::protocol::Content>> {
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
) -> anyhow::Result<Vec<crate::protocol::Content>> {
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
    let related_resp = search_with_options(
        &kb_path,
        &related_query,
        related_limit,
        SearchMode::Hybrid,
        SearchOptions::for_api(),
    )?;
    let related_hits = related_resp
        .hits
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
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(registry, args)?;
    let request = parse_patch_request(args)?;
    let result = preview_patch(&kb_path, &request)?;
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(args))]
pub async fn handle_patch_wiki_entry(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(registry, args)?;
    let request = parse_patch_request(args)?;
    let result = apply_patch(&kb_path, &request)?;
    reindex_entry(&kb_path, &request.entry_path)?;
    // Note: cache invalidation happens on full rebuild; single-entry reindex reloads on next search.
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(registry, args))]
pub async fn handle_get_compilation_context(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let input: GetCompilationContextInput = crate::tools::json::parse_args(args)?;
    let kb_path = parse_kb_path(registry, args)?;
    let store = CompileJobStore::new(&kb_path);

    let job = if let Some(ref id) = input.job_id {
        Some(store.load_job(id)?)
    } else {
        store.active_job()?
    };

    let view = store.build_view()?;
    let quality_snapshot = QualitySnapshotStore::new(&kb_path).read().unwrap_or_default();

    let mut prompt_excerpts = Vec::new();
    if input.include_prompt_excerpts {
        if let Some(ref j) = job {
            let max = input.max_chars as usize;
            for path in &j.artifacts.prompt_paths {
                if let Ok(text) = fs::read_to_string(path) {
                    let excerpt = truncate_chars(&text, max);
                    prompt_excerpts.push(PromptExcerptOut { path: path.clone(), excerpt });
                }
            }
        }
    }

    let suggested = match view.pipeline_status.as_deref() {
        Some("awaiting_agent") => vec![
            "save_wiki_entry".to_string(),
            "get_agent_context".to_string(),
            "complete_compile_job".to_string(),
        ],
        Some("running") => vec!["get_compile_status".to_string()],
        _ => vec!["compile_to_wiki".to_string(), "incremental_compile".to_string()],
    };

    let job_json = job.as_ref().map(serde_json::to_value).transpose()?;
    let stages =
        job_json.as_ref().and_then(|j| j.get("stages")).cloned().unwrap_or(serde_json::json!([]));
    let artifacts = job_json
        .as_ref()
        .and_then(|j| j.get("artifacts"))
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let stats =
        job_json.as_ref().and_then(|j| j.get("stats")).cloned().unwrap_or(serde_json::json!({}));

    json_content(&GetCompilationContextOutput {
        active_job_id: view.active_job_id.clone(),
        pipeline_status: view.pipeline_status.clone(),
        job: job_json,
        stages,
        artifacts,
        stats,
        quality_snapshot: serde_json::to_value(&quality_snapshot)?,
        suggested_next_tools: suggested,
        prompt_excerpts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_tool_names_in_manifest() {
        let names: std::collections::HashSet<_> =
            pdf_mcp_contracts::all_tool_specs().into_iter().map(|s| s.name).collect();
        for name in ["search_knowledge", "get_compilation_context", "apply_wiki_patch"] {
            assert!(names.contains(name), "missing {name}");
        }
    }

    fn test_ctx() -> crate::tools::ToolContext {
        use pdf_core::knowledge::IndexCache;
        use pdf_core::{McpPdfPipeline, ServerConfig};
        use std::sync::Arc;

        let pipeline = Arc::new(McpPdfPipeline::new(&ServerConfig::default()).unwrap());
        let registry = Arc::new(
            pdf_core::management::WorkspaceRegistry::load(
                &std::env::temp_dir().join("rsut_index_test_workspaces.toml"),
            )
            .expect("registry"),
        );
        crate::tools::ToolContext::new(pipeline, registry, Arc::new(IndexCache::new()))
    }

    #[tokio::test]
    async fn test_search_knowledge_missing_query() {
        let args = serde_json::json!({});
        let ctx = test_ctx();

        let result = handle_search_knowledge(&ctx, &args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing query"));
    }

    #[tokio::test]
    async fn test_get_entry_context_missing_entry_path() {
        let args = serde_json::json!({});

        let registry = test_ctx().workspace_registry;
        let result = handle_get_entry_context(&registry, &args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing entry_path"));
    }

    #[tokio::test]
    async fn test_suggest_links_missing_entry_path() {
        let args = serde_json::json!({});

        let registry = test_ctx().workspace_registry;
        let result = handle_suggest_links(&registry, &args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing entry_path"));
    }

    #[tokio::test]
    async fn test_export_concept_map_missing_entry_path() {
        let args = serde_json::json!({});

        let registry = test_ctx().workspace_registry;
        let result = handle_export_concept_map(&registry, &args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing entry_path"));
    }
}
