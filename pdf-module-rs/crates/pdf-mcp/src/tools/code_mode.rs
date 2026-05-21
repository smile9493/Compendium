//! Code Mode MCP tools — search + execute batch over the full tool dispatch table.

use crate::protocol::Content;
use crate::tools::{ToolContext, dispatch_api_tool_inner};
use pdf_mcp_contracts::{
    CompendiumCallResult, CompendiumMcpMode, ExecuteCompendiumInput, ExecuteCompendiumOutput,
    SearchCompendiumApiInput, SearchCompendiumApiOutput, allowed_tool_names, code_mode_tool_specs,
    search_api,
};
use tracing::instrument;

/// Embedded TypeScript SDK (regenerate via `cargo run -p pdf-mcp-contracts --bin generate-sdk`).
pub const TYPESCRIPT_SDK: &str = include_str!("../../../../templates/sdk/compendium.d.ts");

pub fn is_code_mode() -> bool {
    CompendiumMcpMode::from_env() == CompendiumMcpMode::Code
}

pub fn tool_definitions() -> Vec<crate::protocol::ToolDefinition> {
    code_mode_tool_specs().into_iter().map(crate::protocol::ToolDefinition::from).collect()
}

#[instrument(skip(args))]
pub async fn handle_search_compendium_api(
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let input: SearchCompendiumApiInput = crate::tools::json::parse_args(args)?;
    let limit = input.limit.clamp(1, 50) as usize;
    let hits = search_api(&input.query, limit);
    let out = SearchCompendiumApiOutput { hits, total_catalog: pdf_mcp_contracts::tool_count() };
    Ok(vec![Content::text(serde_json::to_string_pretty(&out)?)])
}

fn content_to_json(content: &[Content]) -> serde_json::Value {
    let texts: Vec<&str> = content.iter().map(|c| c.text.as_str()).collect();
    if texts.len() == 1 {
        if let Ok(v) = serde_json::from_str(texts[0]) {
            return v;
        }
        return serde_json::Value::String(texts[0].to_string());
    }
    serde_json::json!({ "content": texts })
}

fn truncate_result_value(
    mut value: serde_json::Value,
    max_chars: usize,
) -> (serde_json::Value, bool) {
    let serialized = serde_json::to_string(&value).unwrap_or_default();
    if serialized.chars().count() <= max_chars {
        return (value, false);
    }
    let truncated_str = format!(
        "{}… [truncated, {} chars total — read wiki files on disk for full content]",
        serialized.chars().take(max_chars).collect::<String>(),
        serialized.chars().count()
    );
    value = serde_json::json!({
        "truncated": true,
        "preview": truncated_str,
        "hint": "Use get_wiki_entry for full Markdown; get_agent_context for token-budget excerpts."
    });
    (value, true)
}

#[instrument(skip(ctx, args))]
pub async fn handle_execute_compendium(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let input: ExecuteCompendiumInput = crate::tools::json::parse_args(args)?;
    let whitelist = allowed_tool_names();
    let max_calls = input.max_calls.clamp(1, 20) as usize;
    let max_result_chars = input.max_result_chars.clamp(256, 64_000) as usize;

    if input.calls.is_empty() {
        anyhow::bail!("calls must not be empty");
    }
    if input.calls.len() > max_calls {
        anyhow::bail!("too many calls: {} (max_calls={})", input.calls.len(), max_calls);
    }

    let mut results = Vec::with_capacity(input.calls.len());
    let mut stop = false;

    for call in &input.calls {
        if stop {
            results.push(CompendiumCallResult {
                method: call.method.clone(),
                ok: false,
                data: None,
                error: Some("skipped: stop_on_error".to_string()),
                truncated: None,
            });
            continue;
        }

        if !whitelist.contains(&call.method) {
            results.push(CompendiumCallResult {
                method: call.method.clone(),
                ok: false,
                data: None,
                error: Some(format!("unknown method: {}", call.method)),
                truncated: None,
            });
            if input.stop_on_error {
                stop = true;
            }
            continue;
        }

        match dispatch_api_tool_inner(ctx, &call.method, &call.args).await {
            Ok(content) => {
                let data = content_to_json(&content);
                let (data, was_truncated) = truncate_result_value(data, max_result_chars);
                results.push(CompendiumCallResult {
                    method: call.method.clone(),
                    ok: true,
                    data: Some(data),
                    error: None,
                    truncated: was_truncated.then_some(true),
                });
            }
            Err(e) => {
                results.push(CompendiumCallResult {
                    method: call.method.clone(),
                    ok: false,
                    data: None,
                    error: Some(e.to_string()),
                    truncated: None,
                });
                if input.stop_on_error {
                    stop = true;
                }
            }
        }
    }

    let executed =
        results.iter().filter(|r| r.error.as_deref() != Some("skipped: stop_on_error")).count();
    let out = ExecuteCompendiumOutput { results, executed };
    Ok(vec![Content::text(serde_json::to_string_pretty(&out)?)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::create_test_tool_context;

    #[test]
    fn typescript_sdk_embedded() {
        assert!(TYPESCRIPT_SDK.contains("searchKnowledge"));
    }

    #[tokio::test]
    async fn execute_unknown_method_returns_error_shape() {
        let ctx = create_test_tool_context();
        let args = serde_json::json!({
            "calls": [{ "method": "not_a_real_tool", "args": {} }]
        });
        let result = handle_execute_compendium(&ctx, &args).await.expect("execute");
        let out: serde_json::Value = serde_json::from_str(&result[0].text).expect("json");
        assert_eq!(out["results"][0]["ok"], false);
    }

    #[tokio::test]
    async fn dispatch_api_list_workspaces() {
        let ctx = create_test_tool_context();
        let content = dispatch_api_tool_inner(&ctx, "list_workspaces", &serde_json::json!({}))
            .await
            .expect("list_workspaces");
        assert!(!content.is_empty());
    }
}
