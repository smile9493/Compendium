//! Shared hooks after a successful compile run.

use std::path::Path;

use pdf_core::knowledge::rebuild_all;
use pdf_core::management::refresh_quality_snapshot;
use tracing::warn;

/// Rebuild indexes and refresh the quality snapshot after compile.
pub fn post_compile_success(knowledge_base: &Path) {
    if let Err(e) = rebuild_all(knowledge_base) {
        warn!(error = %e, "Post-compile index rebuild failed");
    }
    if let Err(e) = refresh_quality_snapshot(knowledge_base) {
        warn!(error = %e, "Post-compile quality snapshot failed");
    }
}
