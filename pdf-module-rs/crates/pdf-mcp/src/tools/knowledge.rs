use crate::protocol::{Content, ToolDefinition};
use crate::tools::{parse_kb_path, ToolContext};
use pdf_core::dto::ExtractOptions;
use pdf_core::KnowledgeEngine;
use std::sync::Arc;
use tracing::instrument;

pub fn knowledge_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "compile_to_wiki".to_string(),
            description: "Compile a PDF into the knowledge base: extract text, save to raw/, generate compilation prompt for AI. This is the primary entry point for the Karpathy compiler pattern.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pdf_path": {
                        "type": "string",
                        "description": "Absolute path to the PDF file"
                    },
                    "knowledge_base": {
                        "type": "string",
                        "description": "Absolute path to the knowledge base directory"
                    },
                    "domain": {
                        "type": "string",
                        "description": "Domain classification (e.g. 'IT', 'Math'). Default: '未分类'"
                    }
                },
                "required": ["pdf_path", "knowledge_base"]
            }),
        },
        ToolDefinition {
            name: "incremental_compile".to_string(),
            description: "Scan raw/ directory for new or changed PDFs and compile only those that need it. Uses SHA-256 hash comparison for change detection.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": {
                        "type": "string",
                        "description": "Absolute path to the knowledge base directory"
                    }
                },
                "required": ["knowledge_base"]
            }),
        },
        ToolDefinition {
            name: "micro_compile".to_string(),
            description: "On-demand extraction from a PDF for the current conversation context. Results are NOT saved to wiki — they are injected directly into the AI session for immediate use.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pdf_path": {
                        "type": "string",
                        "description": "Absolute path to the PDF file"
                    },
                    "page_range": {
                        "type": "string",
                        "description": "Page range to extract (e.g. '1-5', '3,7,12'). Default: all pages"
                    }
                },
                "required": ["pdf_path"]
            }),
        },
        ToolDefinition {
            name: "aggregate_entries".to_string(),
            description: "Identify clusters of related L1 wiki entries that can be aggregated into L2 summary entries. Returns clusters with shared tags for AI to synthesize. (Phase 3: Hierarchical compilation)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": {
                        "type": "string",
                        "description": "Absolute path to the knowledge base directory"
                    }
                },
                "required": ["knowledge_base"]
            }),
        },
        ToolDefinition {
            name: "hypothesis_test".to_string(),
            description: "Find pairs of entries that explicitly contradict each other, and generate a debate framework for AI to resolve the contradictions. Returns contradiction pairs with entry context for AI-driven analysis. (Phase 4: Dynamic reasoning)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": {
                        "type": "string",
                        "description": "Absolute path to the knowledge base directory"
                    }
                },
                "required": ["knowledge_base"]
            }),
        },
        ToolDefinition {
            name: "recompile_entry".to_string(),
            description: "Recompile a single wiki entry: bumps version, creates backup, checks if source PDF changed, and generates a recompile prompt for AI. Use for quality drift correction.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": {
                        "type": "string",
                        "description": "Absolute path to the knowledge base directory"
                    },
                    "entry_path": {
                        "type": "string",
                        "description": "Relative path of the entry within wiki/ (e.g. 'it/concept.md')"
                    }
                },
                "required": ["knowledge_base", "entry_path"]
            }),
        },
    ]
}

#[instrument(skip(ctx, args))]
pub async fn handle_compile_to_wiki(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let pdf_path_str = args["pdf_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing pdf_path"))?;
    let pdf_path = std::path::Path::new(pdf_path_str);
    let kb_path = parse_kb_path(args)?;
    let domain = args["domain"].as_str();

    pdf_core::FileValidator::validate_path_safety(pdf_path, &ctx.path_config)
        .map_err(|e| anyhow::anyhow!("Path validation failed: {}", e))?;

    let engine = KnowledgeEngine::new(Arc::clone(&ctx.pipeline), &kb_path)?;
    let result = engine.compile_to_wiki(pdf_path, domain).await?;

    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(ctx, args))]
pub async fn handle_incremental_compile(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(args)?;
    let engine = KnowledgeEngine::new(Arc::clone(&ctx.pipeline), &kb_path)?;
    let raw_dir = engine.raw_dir();
    let result = engine.incremental_compile(&raw_dir).await?;

    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(ctx, args))]
pub async fn handle_micro_compile(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let pdf_path_str = args["pdf_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing pdf_path"))?;
    let pdf_path = std::path::Path::new(pdf_path_str);

    pdf_core::FileValidator::validate_path_safety(pdf_path, &ctx.path_config)
        .map_err(|e| anyhow::anyhow!("Path validation failed: {}", e))?;

    let page_range = args["page_range"].as_str();

    let result = ctx
        .pipeline
        .extract_structured(pdf_path, &ExtractOptions::default())
        .await
        .map_err(|e| anyhow::anyhow!("Extraction failed: {}", e))?;

    let text = if let Some(range) = page_range {
        let pages_to_include = parse_page_range(range, result.page_count);
        let filtered: Vec<String> = result
            .pages
            .iter()
            .filter(|p| pages_to_include.contains(&p.page_number))
            .map(|p| format!("## Page {}\n\n{}", p.page_number, p.text))
            .collect();
        filtered.join("\n\n")
    } else {
        result.extracted_text.clone()
    };

    let source_name = pdf_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    let output = format!(
        r#"# 微编译结果: {}

> 注意: 此内容仅用于当前对话上下文，不会保存到 wiki。
> 如需持久化，请使用 `compile_to_wiki` 工具。

- 页数: {}{}

---

{}
"#,
        source_name,
        result.page_count,
        if let Some(r) = page_range {
            format!("\n- 提取范围: {}", r)
        } else {
            String::new()
        },
        text
    );

    Ok(vec![Content::text(output)])
}

fn parse_page_range(range: &str, max_page: u32) -> Vec<u32> {
    let mut pages = Vec::new();
    for part in range.split(',') {
        let part = part.trim();
        if let Some(dash_pos) = part.find('-') {
            if let (Ok(start), Ok(end)) = (
                part[..dash_pos].trim().parse::<u32>(),
                part[dash_pos + 1..].trim().parse::<u32>(),
            ) {
                for p in start..=end.min(max_page) {
                    pages.push(p);
                }
            }
        } else if let Ok(p) = part.parse::<u32>() {
            if p <= max_page {
                pages.push(p);
            }
        }
    }
    pages.sort();
    pages.dedup();
    pages
}

#[instrument(skip(ctx, args))]
pub async fn handle_aggregate_entries(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(args)?;

    let engine = pdf_core::KnowledgeEngine::new(Arc::clone(&ctx.pipeline), &kb_path)?;

    let candidates = engine.identify_aggregation_candidates()?;

    let result = serde_json::json!({
        "candidates": candidates,
        "total_clusters": candidates.len(),
        "instructions": if candidates.is_empty() {
            "No aggregation candidates found. Entries may not have enough shared tags to form clusters.".to_string()
        } else {
            "For each cluster, create an L2 summary entry that synthesizes the key ideas. Use 'aggregated_from' field in front matter to record source entries.".to_string()
        }
    });
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(ctx, args))]
pub async fn handle_hypothesis_test(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(args)?;

    let engine = pdf_core::KnowledgeEngine::new(Arc::clone(&ctx.pipeline), &kb_path)?;

    let contradictions = engine.find_contradictions()?;

    let wiki_dir = kb_path.join("wiki");
    let mut enriched = Vec::new();
    for mut pair in contradictions {
        let path_b = wiki_dir.join(&pair.entry_b);
        if let Ok(content) = tokio::fs::read_to_string(&path_b).await {
            if let Some(entry) = pdf_core::knowledge::KnowledgeEntry::from_markdown(&content) {
                pair.title_b = entry.title;
            }
        }
        enriched.push(pair);
    }

    let result = serde_json::json!({
        "contradiction_pairs": enriched,
        "total": enriched.len(),
        "instructions": if enriched.is_empty() {
            "No explicit contradictions found. Use 'suggest_links' to discover implicit tensions between entries.".to_string()
        } else {
            "For each pair, read both entries and conduct a structured debate: 1) State the core claim of each entry, 2) Identify the precise point of disagreement, 3) Evaluate supporting evidence, 4) Propose a resolution or mark as 'open question'. Write the resolution into both entries' 'contradictions' field with a note.".to_string()
        }
    });
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(ctx, args))]
pub async fn handle_recompile_entry(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(args)?;
    let entry_path = args["entry_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing entry_path"))?;

    let engine = pdf_core::KnowledgeEngine::new(Arc::clone(&ctx.pipeline), &kb_path)?;

    let result = engine.recompile_entry(std::path::Path::new(entry_path))?;

    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}
