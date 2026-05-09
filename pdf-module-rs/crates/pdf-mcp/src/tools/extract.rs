use crate::protocol::{Content, ToolDefinition};
use crate::tools::ToolContext;
use pdf_core::dto::ExtractOptions;
use pdf_core::wiki::{AgentPayload, WikiStorage};
use tracing::instrument;

pub fn extract_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "extract_text".to_string(),
            description: "Extract plain text from a PDF file using pdfium engine".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the PDF file"
                    }
                },
                "required": ["file_path"]
            }),
        },
        ToolDefinition {
            name: "extract_structured".to_string(),
            description: "Extract structured data (per-page text + bbox) from PDF".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the PDF file"
                    }
                },
                "required": ["file_path"]
            }),
        },
        ToolDefinition {
            name: "get_page_count".to_string(),
            description: "Get the number of pages in a PDF file".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the PDF file"
                    }
                },
                "required": ["file_path"]
            }),
        },
        ToolDefinition {
            name: "search_keywords".to_string(),
            description: "Search for keywords in a PDF file and return matches with page numbers and context".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the PDF file"
                    },
                    "keywords": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Keywords to search for"
                    },
                    "case_sensitive": {
                        "type": "boolean",
                        "description": "Case sensitive search (default: false)"
                    },
                    "context_length": {
                        "type": "number",
                        "description": "Characters of context around match (default: 50)"
                    }
                },
                "required": ["file_path", "keywords"]
            }),
        },
        ToolDefinition {
            name: "extrude_to_server_wiki".to_string(),
            description: "Extract PDF to server-side wiki (Karpathy paradigm). Rust engine only saves to raw/, AI Agent should read and create atomic wiki entries.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the PDF file"
                    },
                    "wiki_base_path": {
                        "type": "string",
                        "description": "Base directory for wiki storage (default: ./wiki)"
                    }
                },
                "required": ["file_path"]
            }),
        },
        ToolDefinition {
            name: "extrude_to_agent_payload".to_string(),
            description: "Extract PDF and return markdown payload with knowledge compilation instructions for AI Agent to create local wiki entries".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the PDF file"
                    }
                },
                "required": ["file_path"]
            }),
        },
    ]
}

#[instrument(skip(ctx, args))]
pub async fn handle_extract_text(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let file_path_str = args["file_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing file_path"))?;
    let file_path = std::path::Path::new(file_path_str);

    pdf_core::FileValidator::validate_path_safety(file_path, &ctx.path_config)
        .map_err(|e| anyhow::anyhow!("Path validation failed: {}", e))?;

    let result = ctx.pipeline.extract_text(file_path).await?;
    Ok(vec![Content::text(result.extracted_text)])
}

#[instrument(skip(ctx, args))]
pub async fn handle_extract_structured(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let file_path_str = args["file_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing file_path"))?;
    let file_path = std::path::Path::new(file_path_str);

    pdf_core::FileValidator::validate_path_safety(file_path, &ctx.path_config)
        .map_err(|e| anyhow::anyhow!("Path validation failed: {}", e))?;

    let result = ctx
        .pipeline
        .extract_structured(file_path, &ExtractOptions::default())
        .await?;
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(ctx, args))]
pub async fn handle_get_page_count(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let file_path_str = args["file_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing file_path"))?;
    let file_path = std::path::Path::new(file_path_str);

    pdf_core::FileValidator::validate_path_safety(file_path, &ctx.path_config)
        .map_err(|e| anyhow::anyhow!("Path validation failed: {}", e))?;

    let count = ctx.pipeline.get_page_count(file_path).await?;
    Ok(vec![Content::text(format!("{}", count))])
}

#[instrument(skip(ctx, args))]
pub async fn handle_search_keywords(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let file_path_str = args["file_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing file_path"))?;
    let file_path = std::path::Path::new(file_path_str);

    pdf_core::FileValidator::validate_path_safety(file_path, &ctx.path_config)
        .map_err(|e| anyhow::anyhow!("Path validation failed: {}", e))?;

    let keywords: Vec<String> = args["keywords"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing keywords array"))?
        .iter()
        .filter_map(|k| k.as_str().map(|s| s.to_string()))
        .collect();

    if keywords.is_empty() {
        return Err(anyhow::anyhow!("Keywords array is empty"));
    }

    let case_sensitive = args["case_sensitive"].as_bool().unwrap_or(false);
    let context_length = args["context_length"].as_u64().unwrap_or(50) as usize;

    let result = ctx
        .pipeline
        .extract_structured(file_path, &ExtractOptions::default())
        .await?;
    let text = &result.extracted_text;

    let mut page_boundaries: Vec<(usize, u32)> = Vec::with_capacity(result.pages.len());
    let mut offset = 0usize;
    for page in &result.pages {
        page_boundaries.push((offset, page.page_number));
        offset += page.text.len();
    }

    let find_page = |pos: usize| -> u32 {
        match page_boundaries.binary_search_by(|(start, _)| start.cmp(&pos)) {
            Ok(idx) => page_boundaries[idx].1,
            Err(idx) => {
                if idx == 0 {
                    1
                } else if idx >= page_boundaries.len() {
                    page_boundaries.last().map(|(_, p)| *p).unwrap_or(1)
                } else {
                    page_boundaries[idx - 1].1
                }
            }
        }
    };

    let patterns: Vec<regex::Regex> = keywords
        .iter()
        .map(|kw| {
            let pattern = regex::escape(kw);
            let flags = if case_sensitive { "" } else { "(?i)" };
            regex::Regex::new(&format!("{}{}", flags, pattern))
                .map_err(|e| anyhow::anyhow!("Invalid regex for keyword '{}': {}", kw, e))
        })
        .collect::<anyhow::Result<_>>()?;

    let mut matches: Vec<serde_json::Value> = Vec::with_capacity(256);
    let mut pages_with_matches: std::collections::HashSet<u32> = std::collections::HashSet::new();

    for (keyword, re) in keywords.iter().zip(patterns.iter()) {
        for m in re.find_iter(text) {
            let start = m.start();
            let end = m.end();

            let page_number = find_page(start);
            pages_with_matches.insert(page_number);

            let ctx_start = text.floor_char_boundary(start.saturating_sub(context_length));
            let ctx_end = text.ceil_char_boundary((end + context_length).min(text.len()));

            matches.push(serde_json::json!({
                "keyword": keyword,
                "page": page_number,
                "position": start,
                "context": &text[ctx_start..ctx_end]
            }));
        }
    }

    let search_result = serde_json::json!({
        "total_matches": matches.len(),
        "pages_with_matches": pages_with_matches.len(),
        "matches": matches
    });

    Ok(vec![Content::text(serde_json::to_string(&search_result)?)])
}

#[instrument(skip(ctx, args))]
pub async fn handle_extrude_to_server_wiki(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let file_path_str = args["file_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing file_path"))?;
    let file_path = std::path::Path::new(file_path_str);

    pdf_core::FileValidator::validate_path_safety(file_path, &ctx.path_config)
        .map_err(|e| anyhow::anyhow!("Path validation failed: {}", e))?;

    let wiki_base_path = args["wiki_base_path"]
        .as_str()
        .map(std::path::Path::new)
        .unwrap_or_else(|| std::path::Path::new("./wiki"));

    let storage = WikiStorage::new(wiki_base_path)
        .map_err(|e| anyhow::anyhow!("Failed to create wiki storage: {}", e))?;

    let result = ctx
        .pipeline
        .extract_structured(file_path, &ExtractOptions::default())
        .await
        .map_err(|e| anyhow::anyhow!("Extraction failed: {}", e))?;

    let wiki_result = storage
        .save_raw(&result, file_path, 0.85)
        .map_err(|e| anyhow::anyhow!("Failed to save: {}", e))?;

    let response = serde_json::json!({
        "status": "success",
        "raw_path": wiki_result.raw_path.to_string_lossy().to_string(),
        "index_path": wiki_result.index_path.to_string_lossy().to_string(),
        "log_path": wiki_result.log_path.to_string_lossy().to_string(),
        "page_count": wiki_result.page_count,
        "message": "PDF extracted to raw/. AI Agent should process and create wiki entries.",
        "next_step": "Use extrude_to_agent_payload to get the prompt for AI Agent, or manually process raw/ content."
    });

    Ok(vec![Content::text(serde_json::to_string_pretty(
        &response,
    )?)])
}

#[instrument(skip(ctx, args))]
pub async fn handle_extrude_to_agent_payload(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let file_path_str = args["file_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing file_path"))?;
    let file_path = std::path::Path::new(file_path_str);

    pdf_core::FileValidator::validate_path_safety(file_path, &ctx.path_config)
        .map_err(|e| anyhow::anyhow!("Path validation failed: {}", e))?;

    let result = ctx
        .pipeline
        .extract_structured(file_path, &ExtractOptions::default())
        .await
        .map_err(|e| anyhow::anyhow!("Extraction failed: {}", e))?;

    let payload = AgentPayload::from_extraction(&result, file_path, 0.85);
    let markdown = payload.to_markdown();

    Ok(vec![Content::text(markdown)])
}
