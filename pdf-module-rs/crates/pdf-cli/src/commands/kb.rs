//! Knowledge base initialization (`kb init`).

use anyhow::{Context, Result};
use clap::Args;
use pdf_core::init_knowledge_base;
use std::path::PathBuf;

#[derive(Args, Clone)]
pub struct KbInitArgs {
    /// Target directory for the new knowledge base
    pub path: PathBuf,
}

pub fn run_init(args: KbInitArgs) -> Result<serde_json::Value> {
    let result = init_knowledge_base(&args.path).context("init knowledge base")?;
    Ok(serde_json::json!({
        "knowledge_base": result.knowledge_base,
        "created_files": result.created_files,
        "skipped_files": result.skipped_files,
    }))
}
