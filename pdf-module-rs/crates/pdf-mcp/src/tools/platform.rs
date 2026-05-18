//! Phase 3 platform tools: workspaces, extraction plugins, sync, collaboration.

use pdf_core::knowledge::{apply_patch_proposal, submit_patch_proposal, WikiPatchRequest};
use pdf_core::management::{
    sync_pull, sync_push, sync_status, FileSyncRemote, WorkspaceEntry, WorkspaceRegistry,
};

use crate::protocol::{Content, ToolDefinition};
use crate::tools::{parse_kb_path, ToolContext};

pub fn platform_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "list_workspaces".to_string(),
            description: "List registered knowledge base workspaces.".to_string(),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
        },
        ToolDefinition {
            name: "set_active_workspace".to_string(),
            description: "Set the active workspace by kb_id.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": { "kb_id": { "type": "string" } },
                "required": ["kb_id"]
            }),
        },
        ToolDefinition {
            name: "register_workspace".to_string(),
            description: "Register or update a knowledge base workspace.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "kb_id": { "type": "string" },
                    "name": { "type": "string" },
                    "path": { "type": "string" },
                    "active": { "type": "boolean" }
                },
                "required": ["kb_id", "name", "path"]
            }),
        },
        ToolDefinition {
            name: "list_extraction_plugins".to_string(),
            description: "List extraction backends in the current router.".to_string(),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
        },
        ToolDefinition {
            name: "probe_extraction".to_string(),
            description: "Probe which extraction backend would be selected for a PDF.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string" }
                },
                "required": ["file_path"]
            }),
        },
        ToolDefinition {
            name: "sync_status".to_string(),
            description: "Compare local KB manifest with remote sync store.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "kb_id": { "type": "string" },
                    "knowledge_base": { "type": "string" },
                    "remote_url": { "type": "string", "description": "file:///path or path" }
                },
                "required": ["remote_url"]
            }),
        },
        ToolDefinition {
            name: "sync_push".to_string(),
            description: "Push local KB objects to remote sync store.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "kb_id": { "type": "string" },
                    "knowledge_base": { "type": "string" },
                    "remote_url": { "type": "string" }
                },
                "required": ["remote_url"]
            }),
        },
        ToolDefinition {
            name: "sync_pull".to_string(),
            description: "Pull KB objects from remote sync store.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "kb_id": { "type": "string" },
                    "knowledge_base": { "type": "string" },
                    "remote_url": { "type": "string" },
                    "rebuild_index": { "type": "boolean" }
                },
                "required": ["remote_url"]
            }),
        },
        ToolDefinition {
            name: "submit_patch_proposal".to_string(),
            description: "Submit wiki patch proposal without applying.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "kb_id": { "type": "string" },
                    "knowledge_base": { "type": "string" },
                    "entry_path": { "type": "string" },
                    "operations": { "type": "array" }
                },
                "required": ["entry_path", "operations"]
            }),
        },
        ToolDefinition {
            name: "apply_patch_proposal".to_string(),
            description: "Apply a pending patch proposal by id.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "kb_id": { "type": "string" },
                    "knowledge_base": { "type": "string" },
                    "proposal_id": { "type": "string" }
                },
                "required": ["proposal_id"]
            }),
        },
    ]
}

pub async fn handle_list_workspaces(
    registry: &WorkspaceRegistry,
) -> anyhow::Result<Vec<Content>> {
    let workspaces = registry.list()?;
    let active = registry.active_id()?;
    let body = serde_json::json!({ "workspaces": workspaces, "active_kb_id": active });
    Ok(vec![Content::text(serde_json::to_string_pretty(&body)?)])
}

pub async fn handle_set_active_workspace(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb_id = args["kb_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("kb_id required"))?;
    registry.set_active(kb_id)?;
    Ok(vec![Content::text(format!("Active workspace set to {kb_id}"))])
}

pub async fn handle_register_workspace(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let entry = WorkspaceEntry {
        id: args["kb_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("kb_id required"))?
            .to_string(),
        name: args["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("name required"))?
            .to_string(),
        path: args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("path required"))?
            .into(),
        active: args["active"].as_bool().unwrap_or(false),
    };
    registry.upsert(entry)?;
    Ok(vec![Content::text("Workspace registered".to_string())])
}

pub async fn handle_list_extraction_plugins(
    ctx: &ToolContext,
) -> anyhow::Result<Vec<Content>> {
    let ids = ctx.pipeline.extraction_router().backend_ids();
    Ok(vec![Content::text(serde_json::to_string_pretty(
        &serde_json::json!({ "backends": ids }),
    )?)])
}

pub async fn handle_probe_extraction(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let file_path = args["file_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("file_path required"))?;
    let (backend_id, method) = ctx
        .pipeline
        .extraction_router()
        .select_backend_id(std::path::Path::new(file_path))?;
    let body = serde_json::json!({
        "backend_id": backend_id,
        "extraction_method": format!("{:?}", method),
    });
    Ok(vec![Content::text(serde_json::to_string_pretty(&body)?)])
}

fn remote_from_args(args: &serde_json::Value) -> anyhow::Result<FileSyncRemote> {
    let url = args["remote_url"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("remote_url required"))?;
    FileSyncRemote::from_url(url).map_err(|e| anyhow::anyhow!("{e}"))
}

pub async fn handle_sync_status(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb = parse_kb_path(registry, args)?;
    let remote = remote_from_args(args)?;
    let status = sync_status(&kb, &remote).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(vec![Content::text(serde_json::to_string_pretty(&status)?)])
}

pub async fn handle_sync_push(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb = parse_kb_path(registry, args)?;
    let remote = remote_from_args(args)?;
    let report = sync_push(&kb, &remote).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(vec![Content::text(serde_json::to_string_pretty(&report)?)])
}

pub async fn handle_sync_pull(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb = parse_kb_path(registry, args)?;
    let remote = remote_from_args(args)?;
    let report = sync_pull(&kb, &remote).map_err(|e| anyhow::anyhow!("{e}"))?;
    if args["rebuild_index"].as_bool().unwrap_or(true) && report.rebuilt_index_recommended {
        pdf_core::knowledge::rebuild_all(&kb).map_err(|e| anyhow::anyhow!("rebuild: {e}"))?;
    }
    Ok(vec![Content::text(serde_json::to_string_pretty(&report)?)])
}

pub async fn handle_submit_patch_proposal(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb = parse_kb_path(registry, args)?;
    let request: WikiPatchRequest = serde_json::from_value(serde_json::json!({
        "entry_path": args["entry_path"],
        "operations": args["operations"],
    }))?;
    let proposal = submit_patch_proposal(&kb, request, args["actor"].as_str().map(str::to_string))
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(vec![Content::text(serde_json::to_string_pretty(&proposal)?)])
}

pub async fn handle_apply_patch_proposal(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<Content>> {
    let kb = parse_kb_path(registry, args)?;
    let proposal_id = args["proposal_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("proposal_id required"))?;
    let result = apply_patch_proposal(
        &kb,
        proposal_id,
        args["actor"].as_str().map(str::to_string),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}
