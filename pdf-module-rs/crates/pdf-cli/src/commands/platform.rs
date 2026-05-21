//! Phase 3 CLI: workspaces and sync.

use std::path::PathBuf;

use clap::Subcommand;
use pdf_core::management::{
    FileSyncRemote, SyncConflictResolution, WorkspaceEntry, WorkspaceRegistry, sync_pull,
    sync_push, sync_status,
};

use super::{CmdResult, Mode};
use crate::config::CliConfig;
use crate::output::OutputFormat;

#[derive(Clone, Subcommand)]
pub enum WorkspaceAction {
    /// List registered workspaces
    List,
    /// Set active workspace
    SetActive {
        #[arg(long)]
        kb_id: String,
    },
    /// Register workspace
    Add {
        #[arg(long)]
        kb_id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        path: PathBuf,
    },
}

#[derive(Clone, Subcommand)]
pub enum SyncAction {
    Status {
        #[arg(long)]
        remote_url: String,
        #[arg(long)]
        knowledge_base: Option<PathBuf>,
        #[arg(long)]
        kb_id: Option<String>,
    },
    Push {
        #[arg(long)]
        remote_url: String,
        #[arg(long)]
        knowledge_base: Option<PathBuf>,
        #[arg(long)]
        kb_id: Option<String>,
    },
    Pull {
        #[arg(long)]
        remote_url: String,
        #[arg(long)]
        knowledge_base: Option<PathBuf>,
        #[arg(long)]
        kb_id: Option<String>,
        #[arg(long, default_value_t = true)]
        rebuild_index: bool,
    },
}

fn resolve_kb(
    config: &CliConfig,
    kb_id: Option<&str>,
    knowledge_base: Option<&PathBuf>,
) -> anyhow::Result<PathBuf> {
    let registry = WorkspaceRegistry::load_default()?;
    let kb_path = knowledge_base.map(|p| p.as_path()).or(config.knowledge_base.as_deref());
    registry.resolve_kb(kb_id, kb_path.and_then(|p| p.to_str())).map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn run_workspace(
    _config: &CliConfig,
    action: WorkspaceAction,
    _format: OutputFormat,
) -> anyhow::Result<CmdResult> {
    let registry = WorkspaceRegistry::load_default()?;
    let result = match action {
        WorkspaceAction::List => {
            let workspaces = registry.list()?;
            let active = registry.active_id()?;
            serde_json::json!({ "workspaces": workspaces, "active_kb_id": active })
        }
        WorkspaceAction::SetActive { kb_id } => {
            registry.set_active(&kb_id)?;
            serde_json::json!({ "active_kb_id": kb_id })
        }
        WorkspaceAction::Add { kb_id, name, path } => {
            registry.upsert(WorkspaceEntry { id: kb_id.clone(), name, path, active: false })?;
            serde_json::json!({ "registered": kb_id })
        }
    };
    Ok(CmdResult::new("Workspace", result))
}

pub fn run_sync(
    config: &CliConfig,
    mode: Mode,
    action: SyncAction,
    _format: OutputFormat,
) -> anyhow::Result<CmdResult> {
    if mode == Mode::Remote {
        anyhow::bail!("sync commands require local mode");
    }
    let result = match action {
        SyncAction::Status { remote_url, knowledge_base, kb_id } => {
            let kb = resolve_kb(config, kb_id.as_deref(), knowledge_base.as_ref())?;
            let remote = FileSyncRemote::from_url(&remote_url)?;
            let status = sync_status(&kb, &remote)?;
            serde_json::to_value(status)?
        }
        SyncAction::Push { remote_url, knowledge_base, kb_id } => {
            let kb = resolve_kb(config, kb_id.as_deref(), knowledge_base.as_ref())?;
            let remote = FileSyncRemote::from_url(&remote_url)?;
            let report = sync_push(&kb, &remote, SyncConflictResolution::Abort)?;
            serde_json::to_value(report)?
        }
        SyncAction::Pull { remote_url, knowledge_base, kb_id, rebuild_index } => {
            let kb = resolve_kb(config, kb_id.as_deref(), knowledge_base.as_ref())?;
            let remote = FileSyncRemote::from_url(&remote_url)?;
            let report = sync_pull(&kb, &remote, SyncConflictResolution::Abort)?;
            if rebuild_index {
                pdf_core::knowledge::rebuild_all(&kb)?;
            }
            serde_json::to_value(report)?
        }
    };
    Ok(CmdResult::new("Sync", result))
}
