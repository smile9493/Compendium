//! # Compile Commands
//!
//! compile, micro-compile, recompile, incremental

use crate::commands::{CmdResult, Mode, build_remote_client, resolve_kb_path, resolve_remote_kb};
use crate::config::CliConfig;
use crate::local;
use crate::output::OutputFormat;
use anyhow::Result;
use clap::Args;
use serde_json::Value;
use std::path::PathBuf;

/// Compile a PDF into the knowledge base.
#[derive(Debug, Args)]
pub struct CompileArgs {
    /// Path to the PDF file
    pub input: PathBuf,

    /// Domain classification (e.g. "IT", "Math")
    #[arg(short, long)]
    pub domain: Option<String>,

    /// Knowledge base path (local) or remote path (remote)
    #[arg(long)]
    pub knowledge_base: Option<String>,

    /// Output format
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

pub async fn run_compile(config: &CliConfig, mode: Mode, args: &CompileArgs) -> Result<CmdResult> {
    match mode {
        Mode::Local => {
            let kb =
                resolve_kb_path(config, args.knowledge_base.as_ref().map(PathBuf::from).as_deref());
            let result = local::compile_to_wiki(&kb, &args.input, args.domain.as_deref()).await?;
            Ok(CmdResult::new("Compile Result (Local)", result))
        }
        Mode::Remote => {
            let client = build_remote_client(config)?;
            let kb = resolve_remote_kb(config, args.knowledge_base.as_deref());
            crate::commands::remote_compile_uploaded(
                &client,
                &args.input,
                args.domain.as_deref(),
                &kb,
            )
            .await
        }
    }
}

/// Micro-compile: extract text for current session, not saved to wiki.
#[derive(Debug, Args)]
pub struct MicroCompileArgs {
    /// Path to the PDF file
    pub input: PathBuf,

    /// Page range (e.g. "1-5", "3,7,12")
    #[arg(short, long)]
    pub range: Option<String>,

    /// Knowledge base path (local mode only)
    #[arg(long)]
    pub knowledge_base: Option<String>,

    /// Output format
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

pub async fn run_micro_compile(
    config: &CliConfig,
    mode: Mode,
    args: &MicroCompileArgs,
) -> Result<CmdResult> {
    match mode {
        Mode::Local => {
            let kb =
                resolve_kb_path(config, args.knowledge_base.as_ref().map(PathBuf::from).as_deref());
            let result = local::micro_compile(&kb, &args.input, args.range.as_deref()).await?;
            Ok(CmdResult::new("Micro-Compile Result (Local)", result))
        }
        Mode::Remote => {
            let client = build_remote_client(config)?;
            let mut mcp_args = serde_json::json!({
                "pdf_path": args.input.to_string_lossy(),
            });
            if let Some(ref range) = args.range {
                mcp_args["page_range"] = Value::String(range.clone());
            }
            let result = client.call_tool("micro_compile", mcp_args).await?;
            Ok(CmdResult::new("Micro-Compile Result (Remote)", result))
        }
    }
}

/// Recompile an existing wiki entry.
#[derive(Debug, Args)]
pub struct RecompileArgs {
    /// Relative path of the entry within wiki/ (e.g. "it/concept.md")
    pub entry_path: String,

    /// Knowledge base path
    #[arg(long)]
    pub knowledge_base: Option<String>,

    /// Output format
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

pub async fn run_recompile(
    config: &CliConfig,
    mode: Mode,
    args: &RecompileArgs,
) -> Result<CmdResult> {
    match mode {
        Mode::Local => {
            let kb =
                resolve_kb_path(config, args.knowledge_base.as_ref().map(PathBuf::from).as_deref());
            let result = local::recompile_entry(&kb, std::path::Path::new(&args.entry_path))?;
            Ok(CmdResult::new("Recompile Result (Local)", result))
        }
        Mode::Remote => {
            let client = build_remote_client(config)?;
            let kb = resolve_remote_kb(config, args.knowledge_base.as_deref());
            let mcp_args = serde_json::json!({
                "knowledge_base": kb,
                "entry_path": args.entry_path,
            });
            let result = client.call_tool("recompile_entry", mcp_args).await?;
            Ok(CmdResult::new("Recompile Result (Remote)", result))
        }
    }
}

/// Incremental compile of the entire raw/ directory.
#[derive(Debug, Args)]
pub struct IncrementalArgs {
    /// Knowledge base path
    #[arg(long)]
    pub knowledge_base: Option<String>,

    /// Output format
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

pub async fn run_incremental(
    config: &CliConfig,
    mode: Mode,
    args: &IncrementalArgs,
) -> Result<CmdResult> {
    match mode {
        Mode::Local => {
            let kb =
                resolve_kb_path(config, args.knowledge_base.as_ref().map(PathBuf::from).as_deref());
            let result = local::incremental_compile(&kb).await?;
            Ok(CmdResult::new("Incremental Compile (Local)", result))
        }
        Mode::Remote => {
            let client = build_remote_client(config)?;
            let kb = resolve_remote_kb(config, args.knowledge_base.as_deref());
            let mcp_args = serde_json::json!({
                "knowledge_base": kb,
            });
            let result = client.call_tool("trigger_incremental_compile", mcp_args).await?;
            Ok(CmdResult::new("Incremental Compile (Remote)", result))
        }
    }
}
