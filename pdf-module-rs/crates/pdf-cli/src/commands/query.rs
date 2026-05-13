//! # Query Commands
//!
//! search, context, concept-map, orphans, stats

use crate::commands::{
    build_remote_client, resolve_kb_path, resolve_remote_kb, CmdResult, Mode,
};
use crate::config::CliConfig;
use crate::local;
use crate::output::OutputFormat;
use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

/// Full-text search across wiki entries.
#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Search query
    pub query: String,

    /// Maximum results
    #[arg(short, long, default_value = "10")]
    pub limit: usize,

    /// Knowledge base path
    #[arg(long)]
    pub knowledge_base: Option<String>,

    /// Output format
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

pub async fn run_search(
    config: &CliConfig,
    mode: Mode,
    args: &SearchArgs,
) -> Result<CmdResult> {
    match mode {
        Mode::Local => {
            let kb = resolve_kb_path(config, args.knowledge_base.as_ref().map(PathBuf::from).as_deref());
            let result = local::search(&kb, &args.query, args.limit)?;
            Ok(CmdResult::new(format!("Search: \"{}\"", args.query), result))
        }
        Mode::Remote => {
            let client = build_remote_client(config)?;
            let result = client.search_wiki(&args.query, args.limit).await?;
            Ok(CmdResult::new(format!("Search: \"{}\"", args.query), result))
        }
    }
}

/// Get N-hop context for an entry.
#[derive(Debug, Args)]
pub struct ContextArgs {
    /// Relative path of the entry within wiki/ (e.g. "it/concept.md")
    pub entry_path: String,

    /// Number of hops to traverse
    #[arg(short, long, default_value = "2")]
    pub hops: u32,

    /// Knowledge base path
    #[arg(long)]
    pub knowledge_base: Option<String>,

    /// Output format
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

pub async fn run_context(
    config: &CliConfig,
    mode: Mode,
    args: &ContextArgs,
) -> Result<CmdResult> {
    match mode {
        Mode::Local => {
            let kb = resolve_kb_path(config, args.knowledge_base.as_ref().map(PathBuf::from).as_deref());
            let result = local::get_entry_context(&kb, &args.entry_path, args.hops)?;
            Ok(CmdResult::new(format!("Context: {}", args.entry_path), result))
        }
        Mode::Remote => {
            let client = build_remote_client(config)?;
            let kb = resolve_remote_kb(config, args.knowledge_base.as_deref());
            let mcp_args = serde_json::json!({
                "knowledge_base": kb,
                "entry_path": args.entry_path,
                "hops": args.hops,
            });
            let result = client.call_tool("get_entry_context", mcp_args).await?;
            Ok(CmdResult::new(format!("Context: {}", args.entry_path), result))
        }
    }
}

/// Export concept map as Mermaid.js.
#[derive(Debug, Args)]
pub struct ConceptMapArgs {
    /// Relative path of the center entry within wiki/
    pub entry_path: String,

    /// Depth (hops) to include
    #[arg(short, long, default_value = "2")]
    pub depth: u32,

    /// Knowledge base path
    #[arg(long)]
    pub knowledge_base: Option<String>,

    /// Output format
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

pub async fn run_concept_map(
    config: &CliConfig,
    mode: Mode,
    args: &ConceptMapArgs,
) -> Result<CmdResult> {
    match mode {
        Mode::Local => {
            let kb = resolve_kb_path(config, args.knowledge_base.as_ref().map(PathBuf::from).as_deref());
            let result = local::export_concept_map(&kb, &args.entry_path, args.depth)?;
            Ok(CmdResult::new(format!("Concept Map: {}", args.entry_path), result))
        }
        Mode::Remote => {
            let client = build_remote_client(config)?;
            let kb = resolve_remote_kb(config, args.knowledge_base.as_deref());
            let mcp_args = serde_json::json!({
                "knowledge_base": kb,
                "entry_path": args.entry_path,
                "depth": args.depth,
            });
            let result = client.call_tool("export_concept_map", mcp_args).await?;
            Ok(CmdResult::new(format!("Concept Map: {}", args.entry_path), result))
        }
    }
}

/// List orphan entries.
#[derive(Debug, Args)]
pub struct OrphansArgs {
    /// Knowledge base path
    #[arg(long)]
    pub knowledge_base: Option<String>,

    /// Output format
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

pub async fn run_orphans(
    config: &CliConfig,
    mode: Mode,
    args: &OrphansArgs,
) -> Result<CmdResult> {
    match mode {
        Mode::Local => {
            let kb = resolve_kb_path(config, args.knowledge_base.as_ref().map(PathBuf::from).as_deref());
            let result = local::find_orphans(&kb)?;
            Ok(CmdResult::new("Orphan Entries", result))
        }
        Mode::Remote => {
            let client = build_remote_client(config)?;
            let kb = resolve_remote_kb(config, args.knowledge_base.as_deref());
            let mcp_args = serde_json::json!({
                "knowledge_base": kb,
            });
            let result = client.call_tool("find_orphans", mcp_args).await?;
            Ok(CmdResult::new("Orphan Entries", result))
        }
    }
}

/// Knowledge base statistics.
#[derive(Debug, Args)]
pub struct StatsArgs {
    /// Knowledge base path
    #[arg(long)]
    pub knowledge_base: Option<String>,

    /// Output format
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

pub async fn run_stats(
    config: &CliConfig,
    mode: Mode,
    args: &StatsArgs,
) -> Result<CmdResult> {
    match mode {
        Mode::Local => {
            let kb = resolve_kb_path(config, args.knowledge_base.as_ref().map(PathBuf::from).as_deref());
            let result = local::stats(&kb)?;
            Ok(CmdResult::new("Knowledge Base Stats", result))
        }
        Mode::Remote => {
            let client = build_remote_client(config)?;
            let result = client.wiki_stats().await?;
            Ok(CmdResult::new("Knowledge Base Stats", result))
        }
    }
}
