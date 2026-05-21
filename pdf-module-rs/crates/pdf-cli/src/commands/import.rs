//! Knowledge base import command (`kb import`).
//!
//! Restores a knowledge base from a `.tar.gz` archive created by `kb export`.

use anyhow::{Context, Result};
use clap::Args;
use pdf_core::import_knowledge_base;
use pdf_core::knowledge::ImportOptions;
use std::path::PathBuf;

#[derive(Args, Clone)]
pub struct KbImportArgs {
    /// Source archive file (*.tar.gz)
    pub archive: PathBuf,

    /// Target directory for the restored knowledge base
    #[arg(short = 'o', long)]
    pub output: PathBuf,

    /// Allow overwriting an existing non-empty target directory
    #[arg(long)]
    pub overwrite: bool,

    /// Skip rebuilding search indexes after extraction
    #[arg(long)]
    pub no_rebuild_indexes: bool,
}

pub fn run_import(args: KbImportArgs) -> Result<serde_json::Value> {
    let options =
        ImportOptions { overwrite: args.overwrite, rebuild_indexes: !args.no_rebuild_indexes };

    let result = import_knowledge_base(&args.archive, &args.output, options)
        .context("Failed to import knowledge base")?;

    Ok(serde_json::json!({
        "knowledge_base": result.knowledge_base.to_string_lossy(),
        "extracted_files": result.extracted_files,
        "total_bytes": result.total_bytes,
        "sections": result.sections,
        "rebuild_stats": result.rebuild_stats.map(|s| serde_json::json!({
            "fulltext_count": s.fulltext_count,
            "graph_nodes": s.graph_nodes,
            "graph_edges": s.graph_edges,
            "vector_count": s.vector_count,
        })),
    }))
}
