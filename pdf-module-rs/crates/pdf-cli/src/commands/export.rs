//! Knowledge base export command (`kb export`).
//!
//! Packs a knowledge base directory into a portable `.tar.gz` archive.

use anyhow::{Context, Result};
use clap::Args;
use pdf_core::export_knowledge_base;
use pdf_core::knowledge::ExportOptions;
use std::path::PathBuf;

#[derive(Args, Clone)]
pub struct KbExportArgs {
    /// Source knowledge base directory
    pub path: PathBuf,

    /// Output archive path (*.tar.gz)
    #[arg(short = 'o', long)]
    pub output: PathBuf,

    /// Include machine-generated indexes (.rsut_index/) for faster restore
    #[arg(long)]
    pub include_indexes: bool,

    /// Include hash cache (.hash_cache) for incremental compile state
    #[arg(long)]
    pub include_hash_cache: bool,

    /// Gzip compression level 0-9 (default: 6)
    #[arg(long, default_value = "6")]
    pub compression_level: u32,
}

pub fn run_export(args: KbExportArgs) -> Result<serde_json::Value> {
    let options = ExportOptions {
        include_indexes: args.include_indexes,
        include_hash_cache: args.include_hash_cache,
        compression_level: args.compression_level,
    };

    let result = export_knowledge_base(&args.path, &args.output, options)
        .context("Failed to export knowledge base")?;

    Ok(serde_json::json!({
        "archive": result.archive_path.to_string_lossy(),
        "total_files": result.total_files,
        "total_bytes": result.total_bytes,
        "sections": result.sections,
    }))
}
