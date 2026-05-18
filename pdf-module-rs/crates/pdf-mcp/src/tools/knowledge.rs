use crate::tools::json::json_content;
use crate::tools::{attach_compile_sampling, parse_kb_path, ToolContext};
use pdf_mcp_contracts::{
    AggregateEntriesOutput, CompileToWikiOutput, CompileUploadedPdfOutput,
    CompleteCompileJobOutput, GenerateCompilePlanOutput, GetCompilePlanOutput,
    HypothesisTestOutput, IncrementalCompileOutput, MarkPlanTaskDoneOutput, MicroCompileOutput,
    RecompileEntryOutput, SaveWikiEntryOutput,
};
use pdf_core::dto::ExtractOptions;
use pdf_core::knowledge::entry::{CompileStatus, KnowledgeEntry};
use pdf_core::knowledge::{run_incremental_extract, run_single_pdf_extract, CompilePlanStore};
use pdf_core::management::CompileJobStore;
use pdf_core::KnowledgeEngine;
use std::sync::Arc;
use tracing::instrument;

#[instrument(skip(ctx, args))]
pub async fn handle_compile_to_wiki(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let pdf_path_str =
        args["pdf_path"].as_str().ok_or_else(|| anyhow::anyhow!("Missing pdf_path"))?;
    let pdf_path = std::path::Path::new(pdf_path_str);
    let kb_path = parse_kb_path(&ctx.workspace_registry, args)?;
    let domain = args["domain"].as_str();

    pdf_core::FileValidator::validate_path_safety(pdf_path, &ctx.path_config)
        .map_err(|e| anyhow::anyhow!("Path validation failed: {}", e))?;

    let engine = KnowledgeEngine::new(Arc::clone(&ctx.pipeline), &kb_path)?;
    let job_store = CompileJobStore::new(&kb_path);
    let (job_id, result) = run_single_pdf_extract(&engine, &job_store, pdf_path, domain).await?;

    let mut payload = serde_json::json!({
        "job_id": job_id,
        "pipeline_status": "awaiting_agent",
        "compile_result": result,
        "next_step": "save_wiki_entry then complete_compile_job"
    });
    attach_compile_sampling(ctx, &kb_path, &job_id, &mut payload).await;
    json_content(&CompileToWikiOutput { result: payload })
}

#[instrument(skip(ctx, args))]
pub async fn handle_incremental_compile(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(&ctx.workspace_registry, args)?;
    let engine = KnowledgeEngine::new(Arc::clone(&ctx.pipeline), &kb_path)?;
    let job_store = CompileJobStore::new(&kb_path);
    let (job_id, result) = run_incremental_extract(&engine, &job_store).await?;

    let mut payload = serde_json::json!({
        "job_id": job_id,
        "pipeline_status": "awaiting_agent",
        "incremental_result": result,
        "next_step": "save_wiki_entry then complete_compile_job"
    });
    attach_compile_sampling(ctx, &kb_path, &job_id, &mut payload).await;
    json_content(&IncrementalCompileOutput { result: payload })
}

#[instrument(skip(ctx, args))]
pub async fn handle_micro_compile(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let pdf_path_str =
        args["pdf_path"].as_str().ok_or_else(|| anyhow::anyhow!("Missing pdf_path"))?;
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

    let source_name = pdf_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");

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
        if let Some(r) = page_range { format!("\n- 提取范围: {}", r) } else { String::new() },
        text
    );

    json_content(&MicroCompileOutput {
        result: serde_json::json!({ "markdown": output }),
    })
}

fn parse_page_range(range: &str, max_page: u32) -> Vec<u32> {
    let mut pages = Vec::new();
    for part in range.split(',') {
        let part = part.trim();
        if let Some(dash_pos) = part.find('-') {
            if let (Ok(start), Ok(end)) =
                (part[..dash_pos].trim().parse::<u32>(), part[dash_pos + 1..].trim().parse::<u32>())
            {
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
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(&ctx.workspace_registry, args)?;

    let engine = pdf_core::KnowledgeEngine::new(Arc::clone(&ctx.pipeline), &kb_path)?;

    let plan = engine.generate_compile_plan()?;
    let l2_tasks: Vec<_> = plan
        .tasks
        .iter()
        .filter(|t| matches!(t.kind, pdf_core::knowledge::PlanTaskKind::AggregateL2))
        .collect();

    let result = serde_json::json!({
        "plan_path": ".rsut_index/compile_plan.json",
        "l2_tasks": l2_tasks,
        "total_l2_tasks": l2_tasks.len(),
        "total_tasks": plan.tasks.len(),
        "instructions": "Execute tasks from get_compile_plan; use save_wiki_entry(plan_task_id=...) then complete_compile_job."
    });
    json_content(&AggregateEntriesOutput { result })
}

#[instrument(skip(ctx, args))]
pub async fn handle_hypothesis_test(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(&ctx.workspace_registry, args)?;

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

    let resolution_template = serde_json::json!({
        "contradictions": ["wiki/other/entry.md"],
        "note": "Resolved: <summary> | Open question: <question>"
    });

    let quality_issues: Vec<_> = enriched
        .iter()
        .map(|p| {
            pdf_core::knowledge::quality_issues::issues_from_contradictions(&[(
                p.entry_a.clone(),
                p.entry_b.clone(),
            )])
        })
        .flatten()
        .collect();

    let result = serde_json::json!({
        "contradiction_pairs": enriched,
        "total": enriched.len(),
        "quality_issues": quality_issues,
        "resolution_template": resolution_template,
        "next_actions": [{
            "tool": "patch_wiki_entry",
            "args": {
                "knowledge_base": kb_path.to_string_lossy(),
                "entry_path": "<entry_a>",
                "operations": [{
                    "type": "replace_front_matter",
                    "contradictions": []
                }]
            },
            "reason": "Update front matter after resolving a contradiction pair"
        }],
        "instructions": if enriched.is_empty() {
            "No explicit contradictions found. Use 'suggest_links' to discover implicit tensions between entries.".to_string()
        } else {
            "For each pair, read both entries and conduct a structured debate: 1) State the core claim of each entry, 2) Identify the precise point of disagreement, 3) Evaluate supporting evidence, 4) Propose a resolution or mark as 'open question'. Use patch_wiki_entry with resolution_template fields.".to_string()
        }
    });
    json_content(&HypothesisTestOutput { result })
}

#[instrument(skip(ctx, args))]
pub async fn handle_recompile_entry(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(&ctx.workspace_registry, args)?;
    let entry_path =
        args["entry_path"].as_str().ok_or_else(|| anyhow::anyhow!("Missing entry_path"))?;

    let engine = pdf_core::KnowledgeEngine::new(Arc::clone(&ctx.pipeline), &kb_path)?;

    let result = engine.recompile_entry(std::path::Path::new(entry_path))?;

    json_content(&RecompileEntryOutput {
        result: serde_json::to_value(&result)?,
    })
}

#[instrument(skip(ctx, args))]
pub async fn handle_compile_uploaded_pdf(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let file_id = args["file_id"].as_str().ok_or_else(|| anyhow::anyhow!("Missing file_id"))?;

    let upload_store = ctx
        .upload_store
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Upload store not available on this server"))?;

    let uploaded = upload_store
        .get(file_id)
        .ok_or_else(|| anyhow::anyhow!("File not found or expired: {}", file_id))?;

    let kb_path = parse_kb_path(&ctx.workspace_registry, args)?;
    let domain = args["domain"].as_str();

    let engine = KnowledgeEngine::new(Arc::clone(&ctx.pipeline), &kb_path)?;
    let job_store = CompileJobStore::new(&kb_path);
    let (job_id, result) =
        run_single_pdf_extract(&engine, &job_store, &uploaded.temp_path, domain).await?;

    upload_store.remove(file_id);

    let mut payload = serde_json::json!({
        "job_id": job_id,
        "pipeline_status": "awaiting_agent",
        "compile_result": result,
    });
    attach_compile_sampling(ctx, &kb_path, &job_id, &mut payload).await;
    json_content(&CompileUploadedPdfOutput { result: payload })
}

pub async fn handle_save_wiki_entry(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let entry_path =
        args["entry_path"].as_str().ok_or_else(|| anyhow::anyhow!("Missing entry_path"))?;
    let content = args["content"].as_str().ok_or_else(|| anyhow::anyhow!("Missing content"))?;

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

    let kb_path = parse_kb_path(&ctx.workspace_registry, args)?;
    let wiki_dir = kb_path.join("wiki");
    let target_path = wiki_dir.join(entry_path);

    let resolved = target_path.canonicalize().unwrap_or_else(|_| target_path.clone());
    let wiki_canonical = wiki_dir.canonicalize().unwrap_or_else(|_| wiki_dir.clone());
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
    let mark_compiled = args["mark_compiled"].as_bool().unwrap_or(true);
    let mut final_content = content.to_string();
    if mark_compiled {
        if let Some(mut entry) = KnowledgeEntry::from_markdown(content) {
            if entry.status != CompileStatus::Compiled {
                entry.status = CompileStatus::Compiled;
                entry.touch();
                let body = content.split("---").nth(2).unwrap_or("").trim_start();
                final_content = entry.to_markdown(body)?;
            }
        }
    }

    std::fs::write(&target_path, &final_content)?;

    if let Some(job_id) = args["job_id"].as_str() {
        let job_store = CompileJobStore::new(&kb_path);
        job_store.record_entry_saved(job_id, entry_path)?;
    }

    if let Some(task_id) = args["plan_task_id"].as_str() {
        CompilePlanStore::new(&kb_path).mark_task_done(task_id)?;
    }

    let _ = pdf_core::knowledge::reindex_entry(&kb_path, entry_path);

    let relative_path = entry_path.to_string();
    json_content(&SaveWikiEntryOutput {
        result: serde_json::json!({
            "status": "success",
            "path": relative_path,
            "absolute_path": target_path.to_string_lossy(),
            "size_bytes": final_content.len(),
            "job_id": args["job_id"].as_str(),
            "message": format!("Wiki entry '{}' saved successfully", entry_path)
        }),
    })
}

#[instrument(skip(ctx, args))]
pub async fn handle_complete_compile_job(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(&ctx.workspace_registry, args)?;
    let job_id = args["job_id"].as_str().ok_or_else(|| anyhow::anyhow!("Missing job_id"))?;
    let force = args["force"].as_bool().unwrap_or(false);
    let result = pdf_core::knowledge::complete_compile_job(&kb_path, job_id, force)?;
    ctx.index_cache.invalidate(&kb_path);
    json_content(&CompleteCompileJobOutput {
        result: serde_json::to_value(&result)?,
    })
}

#[instrument(skip(ctx, args))]
pub async fn handle_generate_compile_plan(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(&ctx.workspace_registry, args)?;
    let engine = KnowledgeEngine::new(Arc::clone(&ctx.pipeline), &kb_path)?;
    let plan = engine.generate_compile_plan()?;
    json_content(&GenerateCompilePlanOutput {
        result: serde_json::json!({
            "plan_version": plan.plan_version,
            "generated_at": plan.generated_at,
            "task_count": plan.tasks.len(),
            "tasks": plan.tasks,
        }),
    })
}

#[instrument(skip(ctx, args))]
pub async fn handle_get_compile_plan(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(&ctx.workspace_registry, args)?;
    let plan = CompilePlanStore::new(&kb_path)
        .read()?
        .ok_or_else(|| anyhow::anyhow!("No compile plan; call generate_compile_plan first"))?;
    json_content(&GetCompilePlanOutput {
        result: serde_json::to_value(&plan)?,
    })
}

#[instrument(skip(ctx, args))]
pub async fn handle_mark_plan_task_done(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_path = parse_kb_path(&ctx.workspace_registry, args)?;
    let task_id = args["task_id"].as_str().ok_or_else(|| anyhow::anyhow!("Missing task_id"))?;
    let plan = CompilePlanStore::new(&kb_path).mark_task_done(task_id)?;
    json_content(&MarkPlanTaskDoneOutput {
        result: serde_json::json!({
            "status": "ok",
            "task_id": task_id,
            "remaining_pending": plan.tasks.iter().filter(|t| t.status == pdf_core::knowledge::PlanTaskStatus::Pending).count(),
        }),
    })
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
        crate::tools::create_test_tool_context()
    }

    #[test]
    fn test_knowledge_tool_definitions() {
        let defs: Vec<_> = pdf_mcp_contracts::all_tool_specs()
            .into_iter()
            .filter(|t| {
                matches!(
                    t.name.as_str(),
                    "compile_to_wiki"
                        | "incremental_compile"
                        | "micro_compile"
                        | "aggregate_entries"
                        | "hypothesis_test"
                        | "recompile_entry"
                        | "save_wiki_entry"
                        | "complete_compile_job"
                        | "generate_compile_plan"
                        | "get_compile_plan"
                        | "mark_plan_task_done"
                        | "compile_uploaded_pdf"
                )
            })
            .collect();
        assert!(defs.len() >= 8);

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
                if err_msg.contains("Failed to load pdfium") || err_msg.contains("PDFIUM_LIB_PATH")
                {
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
                if err_msg.contains("Failed to load pdfium") || err_msg.contains("PDFIUM_LIB_PATH")
                {
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
