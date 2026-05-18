//! Optional MCP sampling after compile reaches `awaiting_agent`.
//!
//! When `RSUT_COMPILE_SAMPLING=1` and the stdio server has a [`SamplingClient`],
//! requests a short quality-oriented summary of the generated compile prompt(s)
//! via `sampling/createMessage` (requires a capable MCP host).

use std::path::{Path, PathBuf};

use pdf_core::management::compile_job::{CompileJobStore, PipelineStatus};
use tracing::warn;

use crate::sampling::{Role, SamplingClient, SamplingContent, SamplingMessage, SamplingRequest};
use crate::tools::ToolContext;

/// Summary attached to compile tool JSON when sampling succeeds.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompileSamplingSummary {
    pub model: String,
    pub summary: String,
    pub prompt_path: Option<String>,
}

/// True when `RSUT_COMPILE_SAMPLING` is `1`, `true`, or `yes` (case-insensitive).
pub fn compile_sampling_enabled() -> bool {
    compile_sampling_enabled_from(std::env::var("RSUT_COMPILE_SAMPLING").ok().as_deref())
}

fn compile_sampling_enabled_from(value: Option<&str>) -> bool {
    match value {
        Some(v) => {
            let lower = v.to_ascii_lowercase();
            matches!(lower.as_str(), "1" | "true" | "yes")
        }
        None => false,
    }
}

/// After a successful compile extract, optionally call MCP sampling and persist a hint on the job.
pub async fn maybe_run_compile_sampling(
    ctx: &ToolContext,
    knowledge_base: &Path,
    job_id: &str,
) -> Option<CompileSamplingSummary> {
    if !compile_sampling_enabled() {
        return None;
    }
    let client = ctx.sampling.as_ref()?;

    let store = CompileJobStore::new(knowledge_base);
    let job = store.load_job(job_id).ok()?;
    if job.pipeline_status != PipelineStatus::AwaitingAgent {
        return None;
    }

    let (prompt_path, excerpt) = first_prompt_excerpt(&job.artifacts.prompt_paths, 6000)?;
    if excerpt.trim().is_empty() {
        return None;
    }

    let request = SamplingRequest {
        system_prompt: Some(
            "You assist a knowledge-base compile pipeline. Summarize the compile prompt \
             for the agent: key topics, structure expectations, and quality risks. \
             Be concise (under 400 words)."
                .to_string(),
        ),
        messages: vec![SamplingMessage {
            role: Role::User,
            content: SamplingContent::Text {
                text: format!(
                    "Compile job `{job_id}` is awaiting_agent. Prompt file: `{}`.\n\n---\n{excerpt}",
                    prompt_path.display()
                ),
            },
        }],
        max_tokens: Some(512),
        temperature: Some(0.3),
        ..Default::default()
    };

    match client.request_sampling(request).await {
        Ok(response) => {
            let summary = match response.content {
                SamplingContent::Text { text } => text,
                SamplingContent::Image { .. } => {
                    warn!("Compile sampling returned image content; ignoring");
                    return None;
                }
            };
            let out = CompileSamplingSummary {
                model: response.model,
                summary: summary.clone(),
                prompt_path: Some(prompt_path.to_string_lossy().into_owned()),
            };
            if let Ok(mut updated) = store.load_job(job_id) {
                let hint = summary.chars().take(800).collect::<String>();
                updated.message = Some(format!("sampling_summary: {hint}"));
                if let Err(e) = store.write_job(&updated) {
                    warn!(error = %e, "Failed to persist sampling summary on compile job");
                }
            }
            Some(out)
        }
        Err(e) => {
            warn!(error = %e, job_id = %job_id, "Compile sampling failed (non-fatal)");
            None
        }
    }
}

fn first_prompt_excerpt(prompt_paths: &[String], max_chars: usize) -> Option<(PathBuf, String)> {
    for path in prompt_paths {
        let p = PathBuf::from(path);
        if let Ok(text) = std::fs::read_to_string(&p) {
            let excerpt = truncate_chars(&text, max_chars);
            if !excerpt.is_empty() {
                return Some((p, excerpt));
            }
        }
    }
    None
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_sampling_env_parse() {
        assert!(compile_sampling_enabled_from(Some("1")));
        assert!(compile_sampling_enabled_from(Some("true")));
        assert!(compile_sampling_enabled_from(Some("YES")));
        assert!(!compile_sampling_enabled_from(Some("0")));
        assert!(!compile_sampling_enabled_from(None));
    }

    #[test]
    fn truncate_chars_short() {
        assert_eq!(truncate_chars("abc", 10), "abc");
    }

    #[tokio::test]
    async fn maybe_run_sampling_with_mock_client() {
        use std::sync::Arc;

        use pdf_core::management::compile_job::{CompileTrigger, PipelineStatus};
        use tokio::sync::mpsc;

        use crate::sampling::{OutgoingRequest, SamplingClient, SamplingResponse};
        use crate::tools::ToolContext;
        use pdf_core::knowledge::IndexCache;
        use pdf_core::{McpPdfPipeline, ServerConfig};

        std::env::set_var("RSUT_COMPILE_SAMPLING", "1");

        let dir = tempfile::tempdir().expect("tempdir");
        let kb = dir.path().join("kb");
        std::fs::create_dir_all(&kb).expect("kb");
        let prompt = kb.join("prompt.md");
        std::fs::write(&prompt, "# Compile\n\nSummarize this PDF into wiki.").expect("prompt");

        let store = CompileJobStore::new(&kb);
        let mut job = store.begin_job(CompileTrigger::SinglePdf).expect("job");
        job.pipeline_status = PipelineStatus::AwaitingAgent;
        job.artifacts.prompt_paths.push(prompt.to_string_lossy().into_owned());
        store.write_job(&job).expect("write");

        let (tx, mut rx) = mpsc::channel::<OutgoingRequest>(4);
        let client = Arc::new(SamplingClient::with_sender(5, tx));
        let pipeline = Arc::new(McpPdfPipeline::new(&ServerConfig::default()).expect("pipeline"));
        let registry = Arc::new(
            pdf_core::management::WorkspaceRegistry::load(&dir.path().join("ws.toml")).expect("reg"),
        );
        let ctx = ToolContext::new(pipeline, registry, Arc::new(IndexCache::new()))
            .with_sampling(Arc::clone(&client));

        let job_id = job.job_id.clone();
        let handle = tokio::spawn(async move {
            maybe_run_compile_sampling(&ctx, &kb, &job_id).await
        });

        if let Some(outgoing) = rx.recv().await {
            let response = SamplingResponse {
                model: "test".to_string(),
                role: Role::Assistant,
                content: SamplingContent::Text { text: "Quality OK".to_string() },
                stop_reason: None,
            };
            client.handle_response(outgoing.id, Ok(response)).await;
        }

        let summary = handle.await.expect("join").expect("summary");
        assert!(summary.summary.contains("Quality"));
        std::env::remove_var("RSUT_COMPILE_SAMPLING");
    }
}
