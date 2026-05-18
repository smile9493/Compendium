//! Phase 3 platform tools: workspaces, extraction plugins, sync, collaboration.

use pdf_core::knowledge::{apply_patch_proposal, submit_patch_proposal, WikiPatchRequest};
use pdf_core::management::{
    sync_pull, sync_push, sync_status, FileSyncRemote, WorkspaceEntry, WorkspaceRegistry,
};

use crate::protocol::Content;
use crate::tools::json::{json_content, parse_args};
use crate::tools::mcp_extraction::envelope_from_router;
use crate::tools::{parse_kb_path, ToolContext};
use pdf_core::quality_probe::{ExtractionMethod, QualityProbe};
use pdf_mcp_contracts::{
    ApplyPatchProposalOutput, ListExtractionPluginsOutput, ListWorkspacesOutput,
    ProbeExtractionInput, ProbeExtractionOutput, RegisterWorkspaceOutput, SetActiveWorkspaceOutput,
    SubmitPatchProposalOutput, SyncPullOutput, SyncPushOutput, SyncStatusOutput,
};


pub async fn handle_list_workspaces(registry: &WorkspaceRegistry) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let workspaces = registry.list()?;
    let active = registry.active_id()?;
    let body = serde_json::json!({ "workspaces": workspaces, "active_kb_id": active });
    Ok(vec![Content::text(serde_json::to_string_pretty(&body)?)])
}

pub async fn handle_set_active_workspace(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb_id = args["kb_id"].as_str().ok_or_else(|| anyhow::anyhow!("kb_id required"))?;
    registry.set_active(kb_id)?;
    Ok(vec![Content::text(format!("Active workspace set to {kb_id}"))])
}

pub async fn handle_register_workspace(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let entry = WorkspaceEntry {
        id: args["kb_id"].as_str().ok_or_else(|| anyhow::anyhow!("kb_id required"))?.to_string(),
        name: args["name"].as_str().ok_or_else(|| anyhow::anyhow!("name required"))?.to_string(),
        path: args["path"].as_str().ok_or_else(|| anyhow::anyhow!("path required"))?.into(),
        active: args["active"].as_bool().unwrap_or(false),
    };
    registry.upsert(entry)?;
    Ok(vec![Content::text("Workspace registered".to_string())])
}

pub async fn handle_list_extraction_plugins(ctx: &ToolContext) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let ids = ctx.pipeline.extraction_router().backend_ids();
    Ok(vec![Content::text(serde_json::to_string_pretty(&serde_json::json!({ "backends": ids }))?)])
}

pub async fn handle_probe_extraction(
    ctx: &ToolContext,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let input: ProbeExtractionInput = parse_args(args)?;
    let path = std::path::Path::new(&input.file_path);
    let (backend_id, method) = ctx.pipeline.extraction_router().select_backend_id(path)?;
    let quality = std::fs::read(path).ok().and_then(|data| QualityProbe::analyze(&data).ok());
    let quality_score = quality.as_ref().map(|q| q.confidence);
    let needs_vlm = quality.as_ref().is_some_and(|q| q.needs_vlm);
    let extraction = envelope_from_router(ctx, path, false)?;
    let method_label = match method {
        ExtractionMethod::Pdfium => "pdfium",
        ExtractionMethod::Vlm => "vlm",
        ExtractionMethod::Hybrid => "hybrid",
    };
    json_content(&ProbeExtractionOutput {
        backend_id: backend_id.clone(),
        extraction_method: method_label.to_string(),
        quality_score,
        needs_vlm,
        extraction,
    })
}

fn remote_from_args(args: &serde_json::Value) -> anyhow::Result<FileSyncRemote> {
    let url = args["remote_url"].as_str().ok_or_else(|| anyhow::anyhow!("remote_url required"))?;
    FileSyncRemote::from_url(url).map_err(|e| anyhow::anyhow!("{e}"))
}

pub async fn handle_sync_status(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb = parse_kb_path(registry, args)?;
    let remote = remote_from_args(args)?;
    let status = sync_status(&kb, &remote).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(vec![Content::text(serde_json::to_string_pretty(&status)?)])
}

pub async fn handle_sync_push(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb = parse_kb_path(registry, args)?;
    let remote = remote_from_args(args)?;
    let report = sync_push(&kb, &remote).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(vec![Content::text(serde_json::to_string_pretty(&report)?)])
}

pub async fn handle_sync_pull(
    registry: &WorkspaceRegistry,
    args: &serde_json::Value,
) -> anyhow::Result<Vec<crate::protocol::Content>> {
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
) -> anyhow::Result<Vec<crate::protocol::Content>> {
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
) -> anyhow::Result<Vec<crate::protocol::Content>> {
    let kb = parse_kb_path(registry, args)?;
    let proposal_id =
        args["proposal_id"].as_str().ok_or_else(|| anyhow::anyhow!("proposal_id required"))?;
    let result = apply_patch_proposal(&kb, proposal_id, args["actor"].as_str().map(str::to_string))
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(vec![Content::text(serde_json::to_string_pretty(&result)?)])
}
