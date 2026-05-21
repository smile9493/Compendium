use crate::tools::ToolContext;
use crate::tools::json::{json_content, parse_args};
use crate::tools::mcp_extraction::envelope_from_router;
use pdf_core::dto::ExtractOptions;
use pdf_core::wiki::{AgentPayload, WikiStorage};
use pdf_mcp_contracts::{
    ExtractStructuredInput, ExtractStructuredOutput, ExtractTextInput, ExtractTextOutput,
    ExtrudeToAgentPayloadInput, ExtrudeToAgentPayloadOutput, ExtrudeToServerWikiInput,
    ExtrudeToServerWikiOutput, GetPageCountInput, GetPageCountOutput, KeywordMatch,
    SearchKeywordsInput, SearchKeywordsOutput,
};
use tracing::instrument;

#[instrument(skip(ctx, args))]
pub async fn handle_extract_text(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let input: ExtractTextInput = parse_args(args)?;
    let file_path = std::path::Path::new(&input.file_path);

    pdf_core::FileValidator::validate_path_safety(file_path, &ctx.path_config)
        .map_err(|e| anyhow::anyhow!("Path validation failed: {}", e))?;

    let extraction = envelope_from_router(ctx, file_path, false)?;
    let result = ctx.pipeline.extract_text(file_path).await?;
    json_content(&ExtractTextOutput { text: result.extracted_text, extraction })
}

#[instrument(skip(ctx, args))]
pub async fn handle_extract_structured(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let input: ExtractStructuredInput = parse_args(args)?;
    let file_path = std::path::Path::new(&input.file_path);

    pdf_core::FileValidator::validate_path_safety(file_path, &ctx.path_config)
        .map_err(|e| anyhow::anyhow!("Path validation failed: {}", e))?;

    let extraction = envelope_from_router(ctx, file_path, false)?;
    let result = ctx.pipeline.extract_structured(file_path, &ExtractOptions::default()).await?;
    let structured = serde_json::to_value(&result)?;
    json_content(&ExtractStructuredOutput { structured, extraction })
}

#[instrument(skip(ctx, args))]
pub async fn handle_get_page_count(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let input: GetPageCountInput = parse_args(args)?;
    let file_path = std::path::Path::new(&input.file_path);

    pdf_core::FileValidator::validate_path_safety(file_path, &ctx.path_config)
        .map_err(|e| anyhow::anyhow!("Path validation failed: {}", e))?;

    let extraction = envelope_from_router(ctx, file_path, false)?;
    let count = ctx.pipeline.get_page_count(file_path).await?;
    json_content(&GetPageCountOutput { page_count: count, extraction })
}

#[instrument(skip(ctx, args))]
pub async fn handle_search_keywords(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let input: SearchKeywordsInput = parse_args(args)?;
    let file_path = std::path::Path::new(&input.file_path);

    pdf_core::FileValidator::validate_path_safety(file_path, &ctx.path_config)
        .map_err(|e| anyhow::anyhow!("Path validation failed: {}", e))?;

    if input.keywords.is_empty() {
        return Err(anyhow::anyhow!("Keywords array is empty"));
    }

    let keywords = &input.keywords;
    let case_sensitive = input.case_sensitive;
    let context_length = input.context_length as usize;
    let extraction = envelope_from_router(ctx, file_path, false)?;

    let result = ctx.pipeline.extract_structured(file_path, &ExtractOptions::default()).await?;
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

    let parsed_matches: Vec<KeywordMatch> = matches
        .into_iter()
        .filter_map(|m| {
            Some(KeywordMatch {
                keyword: m["keyword"].as_str()?.to_string(),
                page: m["page"].as_u64()? as u32,
                position: m["position"].as_u64()? as usize,
                context: m["context"].as_str()?.to_string(),
            })
        })
        .collect();

    json_content(&SearchKeywordsOutput {
        total_matches: parsed_matches.len(),
        pages_with_matches: pages_with_matches.len(),
        matches: parsed_matches,
        extraction,
    })
}

#[instrument(skip(ctx, args))]
pub async fn handle_extrude_to_server_wiki(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let input: ExtrudeToServerWikiInput = parse_args(args)?;
    let file_path = std::path::Path::new(&input.file_path);

    pdf_core::FileValidator::validate_path_safety(file_path, &ctx.path_config)
        .map_err(|e| anyhow::anyhow!("Path validation failed: {}", e))?;

    let extraction = envelope_from_router(ctx, file_path, false)?;
    let wiki_base_path = input
        .wiki_base_path
        .as_deref()
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

    json_content(&ExtrudeToServerWikiOutput {
        status: "success".to_string(),
        raw_path: wiki_result.raw_path.to_string_lossy().into_owned(),
        index_path: wiki_result.index_path.to_string_lossy().into_owned(),
        log_path: wiki_result.log_path.to_string_lossy().into_owned(),
        page_count: wiki_result.page_count,
        message: "PDF extracted to raw/. AI Agent should process and create wiki entries."
            .to_string(),
        next_step: "Use extrude_to_agent_payload to get the prompt for AI Agent, or manually process raw/ content.".to_string(),
        extraction,
    })
}

#[instrument(skip(ctx, args))]
pub async fn handle_extrude_to_agent_payload(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let input: ExtrudeToAgentPayloadInput = parse_args(args)?;
    let file_path = std::path::Path::new(&input.file_path);

    pdf_core::FileValidator::validate_path_safety(file_path, &ctx.path_config)
        .map_err(|e| anyhow::anyhow!("Path validation failed: {}", e))?;

    let extraction = envelope_from_router(ctx, file_path, false)?;
    let result = ctx
        .pipeline
        .extract_structured(file_path, &ExtractOptions::default())
        .await
        .map_err(|e| anyhow::anyhow!("Extraction failed: {}", e))?;

    let payload = AgentPayload::from_extraction(&result, file_path, 0.85);
    let markdown = payload.to_markdown();

    json_content(&ExtrudeToAgentPayloadOutput { payload: markdown, extraction })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolContext;

    use tempfile::TempDir;

    fn get_test_pdf_path() -> std::path::PathBuf {
        std::path::PathBuf::from("/opt/pdf-module/深入理解Nginx.PDF")
    }

    fn create_test_context() -> ToolContext {
        crate::tools::create_test_tool_context()
    }

    #[tokio::test]
    async fn test_extract_text_missing_file_path() {
        let ctx = create_test_context();
        let args = serde_json::json!({});
        let result = handle_extract_text(&ctx, &args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid tool params"));
    }

    #[tokio::test]
    async fn test_extract_text_file_not_found() {
        let ctx = create_test_context();
        let args = serde_json::json!({
            "file_path": "/nonexistent/path/file.pdf"
        });
        let result = handle_extract_text(&ctx, &args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extract_text_real_pdf() {
        let pdf_path = get_test_pdf_path();
        if !pdf_path.exists() {
            eprintln!("Skipping test: PDF file not found at {:?}", pdf_path);
            return;
        }

        let ctx = create_test_context();
        let args = serde_json::json!({
            "file_path": pdf_path.to_str().unwrap()
        });

        let result = handle_extract_text(&ctx, &args).await;
        match result {
            Ok(content) => {
                assert_eq!(content.len(), 1);
                let parsed: serde_json::Value =
                    serde_json::from_str(&content[0].text).expect("JSON output");
                let text = parsed["text"].as_str().unwrap_or("");
                assert!(!text.is_empty());
                assert!(text.contains("Nginx") || text.contains("nginx"));
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("Failed to load pdfium") || err_msg.contains("PDFIUM_LIB_PATH")
                {
                    eprintln!("Skipping test: pdfium not available - {:?}", err_msg);
                    return;
                }
                panic!("extract_text failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_get_page_count_real_pdf() {
        let pdf_path = get_test_pdf_path();
        if !pdf_path.exists() {
            eprintln!("Skipping test: PDF file not found at {:?}", pdf_path);
            return;
        }

        let ctx = create_test_context();
        let args = serde_json::json!({
            "file_path": pdf_path.to_str().unwrap()
        });

        let result = handle_get_page_count(&ctx, &args).await;
        match result {
            Ok(content) => {
                assert_eq!(content.len(), 1);
                let page_count: u32 = content[0].text.parse().expect("Should be a number");
                assert!(page_count > 0);
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("Failed to load pdfium") || err_msg.contains("PDFIUM_LIB_PATH")
                {
                    eprintln!("Skipping test: pdfium not available - {:?}", err_msg);
                    return;
                }
                panic!("get_page_count failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_extract_structured_real_pdf() {
        let pdf_path = get_test_pdf_path();
        if !pdf_path.exists() {
            eprintln!("Skipping test: PDF file not found at {:?}", pdf_path);
            return;
        }

        let ctx = create_test_context();
        let args = serde_json::json!({
            "file_path": pdf_path.to_str().unwrap()
        });

        let result = handle_extract_structured(&ctx, &args).await;
        match result {
            Ok(content) => {
                assert_eq!(content.len(), 1);

                let parsed: serde_json::Value =
                    serde_json::from_str(&content[0].text).expect("Should be valid JSON");
                assert!(parsed.get("pages").is_some());
                assert!(parsed.get("extracted_text").is_some());
                assert!(parsed.get("page_count").is_some());
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("Failed to load pdfium") || err_msg.contains("PDFIUM_LIB_PATH")
                {
                    eprintln!("Skipping test: pdfium not available - {:?}", err_msg);
                    return;
                }
                panic!("extract_structured failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_search_keywords_real_pdf() {
        let pdf_path = get_test_pdf_path();
        if !pdf_path.exists() {
            eprintln!("Skipping test: PDF file not found at {:?}", pdf_path);
            return;
        }

        let ctx = create_test_context();
        let args = serde_json::json!({
            "file_path": pdf_path.to_str().unwrap(),
            "keywords": ["nginx", "HTTP"],
            "case_sensitive": false,
            "context_length": 30
        });

        let result = handle_search_keywords(&ctx, &args).await;
        match result {
            Ok(content) => {
                assert_eq!(content.len(), 1);

                let parsed: serde_json::Value =
                    serde_json::from_str(&content[0].text).expect("Should be valid JSON");
                assert!(parsed.get("total_matches").is_some());
                assert!(parsed.get("matches").is_some());
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("Failed to load pdfium") || err_msg.contains("PDFIUM_LIB_PATH")
                {
                    eprintln!("Skipping test: pdfium not available - {:?}", err_msg);
                    return;
                }
                panic!("search_keywords failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_search_keywords_empty_array() {
        let pdf_path = get_test_pdf_path();
        if !pdf_path.exists() {
            eprintln!("Skipping test: PDF file not found at {:?}", pdf_path);
            return;
        }

        let ctx = create_test_context();
        let args = serde_json::json!({
            "file_path": pdf_path.to_str().unwrap(),
            "keywords": []
        });

        let result = handle_search_keywords(&ctx, &args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Keywords array is empty"));
    }

    #[tokio::test]
    async fn test_extrude_to_agent_payload_real_pdf() {
        let pdf_path = get_test_pdf_path();
        if !pdf_path.exists() {
            eprintln!("Skipping test: PDF file not found at {:?}", pdf_path);
            return;
        }

        let ctx = create_test_context();
        let args = serde_json::json!({
            "file_path": pdf_path.to_str().unwrap()
        });

        let result = handle_extrude_to_agent_payload(&ctx, &args).await;
        match result {
            Ok(content) => {
                assert_eq!(content.len(), 1);
                let text = &content[0].text;
                assert!(text.starts_with("# "), "Output should start with '# '");
                assert!(
                    text.contains("Nginx") || text.len() > 100,
                    "Output should contain content"
                );
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("Failed to load pdfium") || err_msg.contains("PDFIUM_LIB_PATH")
                {
                    eprintln!("Skipping test: pdfium not available - {:?}", err_msg);
                    return;
                }
                panic!("extrude_to_agent_payload failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_extrude_to_server_wiki_real_pdf() {
        let pdf_path = get_test_pdf_path();
        if !pdf_path.exists() {
            eprintln!("Skipping test: PDF file not found at {:?}", pdf_path);
            return;
        }

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let wiki_path = temp_dir.path().to_str().unwrap();

        let ctx = create_test_context();
        let args = serde_json::json!({
            "file_path": pdf_path.to_str().unwrap(),
            "wiki_base_path": wiki_path
        });

        let result = handle_extrude_to_server_wiki(&ctx, &args).await;
        match result {
            Ok(content) => {
                assert_eq!(content.len(), 1);

                let parsed: serde_json::Value =
                    serde_json::from_str(&content[0].text).expect("Should be valid JSON");
                assert_eq!(parsed["status"], "success");
                assert!(parsed.get("raw_path").is_some());
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("Failed to load pdfium") || err_msg.contains("PDFIUM_LIB_PATH")
                {
                    eprintln!("Skipping test: pdfium not available - {:?}", err_msg);
                    return;
                }
                panic!("extrude_to_server_wiki failed: {:?}", e);
            }
        }
    }
}
