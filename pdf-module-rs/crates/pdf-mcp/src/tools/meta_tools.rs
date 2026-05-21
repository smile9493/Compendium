//! Karpathy meta tools: `ingest`, `query`, `lint` — orchestrate atomic MCP APIs.

use crate::tools::json::json_content;
use crate::tools::{
    ToolContext, handle_check_quality, handle_compile_to_wiki, handle_find_orphans,
    handle_get_agent_context, handle_incremental_compile, handle_search_knowledge,
};
use pdf_core::knowledge::{DEFAULT_STALE_DAYS, detect_stale_entries, graph, lint_wiki, wiki_dir};
use pdf_mcp_contracts::{
    LoadToolsOutput, MetaIngestOutput, MetaLintOutput, MetaQueryOutput, ToolExposureTier,
    progressive_tool_index, tool_exposure_tier, tools_in_tier,
};
use std::fs;
use tracing::instrument;

use crate::tools::parse_kb_path;

#[instrument(skip(ctx, args))]
pub async fn handle_meta_ingest(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let pipeline_blocks = if let Some(pdf_path) = args.get("pdf_path").and_then(|v| v.as_str()) {
        let mut compile_args = args.clone();
        compile_args["pdf_path"] = serde_json::Value::String(pdf_path.to_string());
        handle_compile_to_wiki(ctx, &compile_args).await?
    } else {
        handle_incremental_compile(ctx, args).await?
    };

    let kb_path = parse_kb_path(&ctx.workspace_registry, args)?;
    let pipeline = parse_tool_json(&pipeline_blocks);

    let payload = serde_json::json!({
        "command": "ingest",
        "pipeline": pipeline,
        "wiki_index_excerpt": read_index_excerpt(&kb_path, 1200),
        "next_steps": [
            "get_compilation_context",
            "save_wiki_entry (per concept)",
            "complete_compile_job"
        ],
        "schema_hint": "Read schema/AGENTS.md for ingest workflow"
    });
    json_content(&MetaIngestOutput { result: payload })
}

#[instrument(skip(ctx, args))]
pub async fn handle_meta_query(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(&ctx.workspace_registry, args)?;

    let retrieval = if let Some(query) = args.get("query").and_then(|v| v.as_str()) {
        let search_blocks = handle_search_knowledge(ctx, args).await?;
        let search = parse_tool_json(&search_blocks);
        let top_path = search
            .get("results")
            .and_then(|r| r.as_array())
            .and_then(|a| a.first())
            .and_then(|h| h.get("path").or_else(|| h.get("entry_path")))
            .and_then(|p| p.as_str());

        let mut out = serde_json::json!({ "query": query, "search": search });
        if let Some(entry_path) = top_path {
            let mut ctx_args = args.clone();
            ctx_args["entry_path"] = serde_json::Value::String(entry_path.to_string());
            ctx_args["knowledge_base"] =
                serde_json::Value::String(kb_path.to_string_lossy().into_owned());
            let ctx_blocks = handle_get_agent_context(&ctx.workspace_registry, &ctx_args).await?;
            out["top_entry_context"] = parse_tool_json(&ctx_blocks);
        }
        out
    } else if let Some(entry_path) = args.get("entry_path").and_then(|v| v.as_str()) {
        let mut ctx_args = args.clone();
        ctx_args["entry_path"] = serde_json::Value::String(entry_path.to_string());
        ctx_args["knowledge_base"] =
            serde_json::Value::String(kb_path.to_string_lossy().into_owned());
        let ctx_blocks = handle_get_agent_context(&ctx.workspace_registry, &ctx_args).await?;
        parse_tool_json(&ctx_blocks)
    } else {
        anyhow::bail!("meta query requires `query` or `entry_path`");
    };

    let payload = serde_json::json!({
        "command": "query",
        "wiki_index_excerpt": read_index_excerpt(&kb_path, 2000),
        "retrieval": retrieval,
        "hint": "Synthesize from wiki; use archive_answer for durable answers",
        "load_tools_hint": "Deferred tools are callable by name without load_tools unlock"
    });
    json_content(&MetaQueryOutput { result: payload })
}

#[instrument(skip(ctx, args))]
pub async fn handle_meta_lint(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(&ctx.workspace_registry, args)?;
    let max_age =
        args.get("max_age_days").and_then(|v| v.as_u64()).unwrap_or(DEFAULT_STALE_DAYS as u64)
            as u32;

    let lint_report = lint_wiki(&kb_path)?;
    let summary = lint_report.summary.clone();
    let stale = detect_stale_entries(&kb_path, max_age)?;
    let graph_ref = graph(&kb_path)?;
    let hub = pdf_core::knowledge::hub_threshold_for_kb(&kb_path, &graph_ref)?;
    let load_bearing = graph_ref.load_bearing_entries(hub);

    let quality_json = parse_tool_json(&handle_check_quality(&ctx.workspace_registry, args).await?);
    let orphans_json = parse_tool_json(&handle_find_orphans(&ctx.workspace_registry, args).await?);
    let diversity = pdf_core::knowledge::analyze_cognitive_diversity(&kb_path, &graph_ref, hub)?;
    let propagation = pdf_core::knowledge::compute_propagation(&kb_path, &graph_ref, 2)?;

    let payload = serde_json::json!({
        "command": "lint",
        "lint_wiki": lint_report,
        "check_quality": quality_json,
        "cognitive_diversity": diversity,
        "confidence_propagation": propagation,
        "find_orphans": orphans_json,
        "detect_stale_entries": {
            "max_age_days": max_age,
            "count": stale.len(),
            "entries": stale
        },
        "load_bearing": load_bearing,
        "summary": summary
    });
    json_content(&MetaLintOutput { result: payload })
}

#[instrument(skip(args))]
pub async fn handle_load_tools(
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let tier_name = args.get("tier").and_then(|v| v.as_str()).unwrap_or("deferred");
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(30).clamp(1, 60) as usize;
    let tier = match tier_name {
        "core" => ToolExposureTier::Core,
        "deferred" => ToolExposureTier::Deferred,
        "code_only" | "code-only" => ToolExposureTier::CodeOnly,
        "all" => {
            let payload = LoadToolsOutput {
                tier: "all".to_string(),
                tools: Vec::new(),
                progressive_index: progressive_tool_index(),
            };
            return json_content(&payload);
        }
        _ => anyhow::bail!("unknown tier: {tier_name}"),
    };
    let names = tools_in_tier(tier);
    let specs: std::collections::HashMap<_, _> =
        pdf_mcp_contracts::all_tool_specs().into_iter().map(|s| (s.name.clone(), s)).collect();
    let tools: Vec<serde_json::Value> = names
        .into_iter()
        .take(limit)
        .filter_map(|name| {
            specs.get(&name).map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "description": s.description,
                    "tier": format!("{:?}", tool_exposure_tier(&s.name)).to_lowercase(),
                    "category": pdf_mcp_contracts::tool_category(&s.name),
                })
            })
        })
        .collect();
    json_content(&LoadToolsOutput {
        tier: tier_name.to_string(),
        tools,
        progressive_index: progressive_tool_index(),
    })
}

fn read_index_excerpt(kb_path: &std::path::Path, max_chars: usize) -> String {
    let index_path = wiki_dir(kb_path).join("index.md");
    let Ok(content) = fs::read_to_string(&index_path) else {
        return String::new();
    };
    if content.len() <= max_chars {
        content
    } else {
        format!("{}…", &content[..content.floor_char_boundary(max_chars)])
    }
}

fn parse_tool_json(blocks: &[crate::protocol::Content]) -> serde_json::Value {
    serde_json::from_str(blocks.first().map(|c| c.text.as_str()).unwrap_or("{}"))
        .unwrap_or_default()
}
