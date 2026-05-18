use crate::protocol::{Content, ToolDefinition};
use crate::tools::{parse_kb_path, ToolContext};
use pdf_core::dto::ExtractOptions;
use pdf_core::management::{CompileFinishStats, CompileStatusStore};
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
                        "description": "Knowledge base path (default: /app/kb or KNOWLEDGE_BASE_PATH env)"
                    },
                    "domain": {
                        "type": "string",
                        "description": "Domain classification (e.g. 'IT', 'Math'). Default: '未分类'"
                    }
                },
                "required": ["pdf_path"]
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
                        "description": "Knowledge base path (default: /app/kb or KNOWLEDGE_BASE_PATH env)"
                    }
                },
                "required": []
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
                        "description": "Knowledge base path (default: /app/kb or KNOWLEDGE_BASE_PATH env)"
                    }
                },
                "required": []
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
                        "description": "Knowledge base path (default: /app/kb or KNOWLEDGE_BASE_PATH env)"
                    }
                },
                "required": []
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
                        "description": "Knowledge base path (default: /app/kb or KNOWLEDGE_BASE_PATH env)"
                    },
                    "entry_path": {
                        "type": "string",
                        "description": "Relative path of the entry within wiki/ (e.g. 'it/concept.md')"
                    }
                },
                "required": ["entry_path"]
            }),
        },
        ToolDefinition {
            name: "save_wiki_entry".to_string(),
            description: "Create or update a wiki entry in the knowledge base. This is the primary write tool for the AI Agent to persist compiled knowledge entries. Entry content MUST follow the YAML front matter format with required fields (domain, level, tags, status, created, updated). Use after compile_to_wiki to save the AI-generated wiki content back to the server, completing the compilation loop.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_base": {
                        "type": "string",
                        "description": "Knowledge base path (default: /app/kb or KNOWLEDGE_BASE_PATH env)"
                    },
                    "entry_path": {
                        "type": "string",
                        "description": "Relative path within wiki/ directory, e.g. 'IT/concept.md'. Must end with .md and must not contain '..' path traversal"
                    },
                    "content": {
                        "type": "string",
                        "description": "Full markdown content with YAML front matter header. Required format: ---\ndomain: XX\nlevel: L1\ntags: [tag1, tag2]\nstatus: draft\ncreated: YYYY-MM-DD\nupdated: YYYY-MM-DD\n---\n\n# Title\nContent..."
                    }
                },
                "required": ["entry_path", "content"]
            }),
        },
        ToolDefinition {
            name: "compile_uploaded_pdf".to_string(),
            description: "Compile an uploaded PDF identified by file_id into the knowledge base. Use after uploading a file via POST /api/upload. This enables cross-network PDF compilation where the client cannot share a filesystem with the server.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_id": {
                        "type": "string",
                        "description": "File ID returned from POST /api/upload"
                    },
                    "knowledge_base": {
                        "type": "string",
                        "description": "Knowledge base path (default: /app/kb or KNOWLEDGE_BASE_PATH env)"
                    },
                    "domain": {
                        "type": "string",
                        "description": "Domain classification (e.g. 'IT', 'Math'). Default: '未分类'"
                    }
                },
                "required": ["file_id"]
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

    let store = CompileStatusStore::new(&kb_path);
    let guard = store
        .begin_compile()
        .map_err(|e| anyhow::anyhow!("Failed to begin compile status: {}", e))?;

    let engine = KnowledgeEngine::new(Arc::clone(&ctx.pipeline), &kb_path)?;
    let result = match engine.compile_to_wiki(pdf_path, domain).await {
        Ok(r) => r,
        Err(e) => {
            let _ = guard.finish_error(e.to_string());
            return Err(e.into());
        }
    };

    guard
        .finish_success(CompileFinishStats {
            entries_compiled: result.entries.len(),
            entries_skipped: 0,
        })
        .map_err(|e| anyhow::anyhow!("Failed to record compile status: {}", e))?;

    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

#[instrument(skip(ctx, args))]
pub async fn handle_incremental_compile(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb_path = parse_kb_path(args)?;
    let store = CompileStatusStore::new(&kb_path);
    let guard = store
        .begin_compile()
        .map_err(|e| anyhow::anyhow!("Failed to begin compile status: {}", e))?;

    let engine = KnowledgeEngine::new(Arc::clone(&ctx.pipeline), &kb_path)?;
    let raw_dir = engine.raw_dir();
    let result = match engine.incremental_compile(&raw_dir).await {
        Ok(r) => r,
        Err(e) => {
            let _ = guard.finish_error(e.to_string());
            return Err(e.into());
        }
    };

    guard
        .finish_success(CompileFinishStats {
            entries_compiled: result.compiled,
            entries_skipped: result.skipped,
        })
        .map_err(|e| anyhow::anyhow!("Failed to record compile status: {}", e))?;

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

#[instrument(skip(ctx, args))]
pub async fn handle_compile_uploaded_pdf(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let file_id = args["file_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing file_id"))?;

    let upload_store = ctx
        .upload_store
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Upload store not available on this server"))?;

    let uploaded = upload_store
        .get(file_id)
        .ok_or_else(|| anyhow::anyhow!("File not found or expired: {}", file_id))?;

    let kb_path = parse_kb_path(args)?;
    let domain = args["domain"].as_str();

    let store = CompileStatusStore::new(&kb_path);
    let guard = store
        .begin_compile()
        .map_err(|e| anyhow::anyhow!("Failed to begin compile status: {}", e))?;

    let engine = KnowledgeEngine::new(Arc::clone(&ctx.pipeline), &kb_path)?;
    let result = match engine.compile_to_wiki(&uploaded.temp_path, domain).await {
        Ok(r) => r,
        Err(e) => {
            let _ = guard.finish_error(e.to_string());
            return Err(e.into());
        }
    };

    guard
        .finish_success(CompileFinishStats {
            entries_compiled: result.entries.len(),
            entries_skipped: 0,
        })
        .map_err(|e| anyhow::anyhow!("Failed to record compile status: {}", e))?;

    // Clean up the uploaded temp file after successful compile
    upload_store.remove(file_id);

    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}

pub async fn handle_save_wiki_entry(
    _ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let entry_path = args["entry_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing entry_path"))?;
    let content = args["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing content"))?;

    if content.trim().is_empty() {
        return Err(anyhow::anyhow!("Content must not be empty"));
    }
    if entry_path.contains("..") || entry_path.starts_with('/') {
        return Err(anyhow::anyhow!(
            "entry_path must be a relative path within wiki/ (no '..' or absolute path): {}",
            entry_path
        ));
    }
    if !entry_path.ends_with(".md") {
        return Err(anyhow::anyhow!("entry_path must end with .md, got: {}", entry_path));
    }

    let kb_path = parse_kb_path(args)?;
    let wiki_dir = kb_path.join("wiki");
    let target_path = wiki_dir.join(entry_path);

    let resolved = target_path
        .canonicalize()
        .unwrap_or_else(|_| target_path.clone());
    let wiki_canonical = wiki_dir
        .canonicalize()
        .unwrap_or_else(|_| wiki_dir.clone());
    if !resolved.starts_with(&wiki_canonical) {
        return Err(anyhow::anyhow!(
            "Path traversal detected: resolved path '{}' is outside wiki directory '{}'",
            resolved.display(),
            wiki_canonical.display()
        ));
    }

    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&target_path, content)?;

    let relative_path = entry_path.to_string();
    Ok(vec![Content::text(
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "success",
            "path": relative_path,
            "absolute_path": target_path.to_string_lossy(),
            "size_bytes": content.len(),
            "message": format!("Wiki entry '{}' saved successfully", entry_path)
        }))?,
    )])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolContext;
    use pdf_core::{McpPdfPipeline, ServerConfig};
    use std::sync::Arc;

    fn get_test_pdf_path() -> std::path::PathBuf {
        std::path::PathBuf::from("/opt/pdf-module/深入理解Nginx.PDF")
    }

    fn create_test_context() -> ToolContext {
        let config = ServerConfig::from_env().unwrap_or_default();
        let pipeline = Arc::new(McpPdfPipeline::new(&config).expect("Failed to create pipeline"));
        ToolContext::new(pipeline)
    }

    #[test]
    fn test_knowledge_tool_definitions() {
        let defs = knowledge_tool_definitions();
        assert_eq!(defs.len(), 8);
        
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"compile_to_wiki"));
        assert!(names.contains(&"compile_uploaded_pdf"));
        assert!(names.contains(&"incremental_compile"));
        assert!(names.contains(&"micro_compile"));
        assert!(names.contains(&"aggregate_entries"));
        assert!(names.contains(&"hypothesis_test"));
        assert!(names.contains(&"recompile_entry"));
        assert!(names.contains(&"save_wiki_entry"));
    }

    #[test]
    fn test_parse_page_range_single() {
        let result = parse_page_range("5", 10);
        assert_eq!(result, vec![5]);
    }

    #[test]
    fn test_parse_page_range_range() {
        let result = parse_page_range("1-3", 10);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_parse_page_range_mixed() {
        let result = parse_page_range("1,3,5-7", 10);
        assert_eq!(result, vec![1, 3, 5, 6, 7]);
    }

    #[test]
    fn test_parse_page_range_exceeds_max() {
        let result = parse_page_range("8-15", 10);
        assert_eq!(result, vec![8, 9, 10]);
    }

    #[test]
    fn test_parse_page_range_duplicates() {
        let result = parse_page_range("1,1,2-3,3", 10);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_micro_compile_real_pdf() {
        let pdf_path = get_test_pdf_path();
        if !pdf_path.exists() {
            eprintln!("Skipping test: PDF file not found at {:?}", pdf_path);
            return;
        }

        let ctx = create_test_context();
        let path_str = pdf_path.to_str().expect("Path should be valid UTF-8");
        eprintln!("Testing with path: {:?}", path_str);
        
        let args = serde_json::json!({
            "pdf_path": path_str
        });
        eprintln!("Args: {:?}", args);
        
        let result = handle_micro_compile(&ctx, &args).await;
        match result {
            Ok(content) => {
                assert_eq!(content.len(), 1);
                assert!(content[0].text.contains("# 微编译结果"));
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("Failed to load pdfium") || err_msg.contains("PDFIUM_LIB_PATH") {
                    eprintln!("Skipping test: pdfium not available - {:?}", err_msg);
                    return;
                }
                panic!("micro_compile failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_micro_compile_with_page_range() {
        let pdf_path = get_test_pdf_path();
        if !pdf_path.exists() {
            eprintln!("Skipping test: PDF file not found at {:?}", pdf_path);
            return;
        }

        let ctx = create_test_context();
        let path_str = pdf_path.to_str().expect("Path should be valid UTF-8");
        eprintln!("Testing with path: {:?}", path_str);
        
        let args = serde_json::json!({
            "pdf_path": path_str,
            "page_range": "1-2"
        });
        eprintln!("Args: {:?}", args);
        
        let result = handle_micro_compile(&ctx, &args).await;
        match result {
            Ok(content) => {
                assert_eq!(content.len(), 1);
                assert!(content[0].text.contains("提取范围: 1-2"));
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("Failed to load pdfium") || err_msg.contains("PDFIUM_LIB_PATH") {
                    eprintln!("Skipping test: pdfium not available - {:?}", err_msg);
                    return;
                }
                panic!("micro_compile failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_compile_to_wiki_missing_pdf_path() {
        let ctx = create_test_context();
        let args = serde_json::json!({
            "knowledge_base": "/tmp/test_kb"
        });
        
        let result = handle_compile_to_wiki(&ctx, &args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing pdf_path"));
    }

    #[tokio::test]
    async fn test_compile_to_wiki_missing_kb() {
        let pdf_path = get_test_pdf_path();
        if !pdf_path.exists() {
            eprintln!("Skipping test: PDF file not found at {:?}", pdf_path);
            return;
        }

        let ctx = create_test_context();
        let args = serde_json::json!({
            "pdf_path": pdf_path.to_str().unwrap()
        });
        
        let result = handle_compile_to_wiki(&ctx, &args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_incremental_compile_default_kb() {
        let ctx = create_test_context();
        let args = serde_json::json!({});
        
        let result = handle_incremental_compile(&ctx, &args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_aggregate_entries_default_kb() {
        let ctx = create_test_context();
        let args = serde_json::json!({});
        
        let result = handle_aggregate_entries(&ctx, &args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_hypothesis_test_default_kb() {
        let ctx = create_test_context();
        let args = serde_json::json!({});
        
        let result = handle_hypothesis_test(&ctx, &args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_recompile_entry_missing_kb() {
        let ctx = create_test_context();
        let args = serde_json::json!({
            "entry_path": "test.md"
        });
        
        let result = handle_recompile_entry(&ctx, &args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_recompile_entry_missing_entry_path() {
        let ctx = create_test_context();
        let args = serde_json::json!({
            "knowledge_base": "/tmp/test_kb"
        });
        
        let result = handle_recompile_entry(&ctx, &args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing entry_path"));
    }
}
