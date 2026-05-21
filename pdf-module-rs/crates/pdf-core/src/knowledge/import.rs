//! Knowledge base import — restore a KB from a `.tar.gz` archive.
//!
//! Extracts a previously exported knowledge base archive into a target directory.
//! Can optionally rebuild the search indexes after extraction.

use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use tracing::{debug, info, warn};

use crate::error::{PdfModuleError, PdfResult};

/// Controls the import behavior.
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// Allow overwriting an existing non-empty knowledge base.
    pub overwrite: bool,
    /// Rebuild Tantivy, graph, and vector indexes after extraction.
    pub rebuild_indexes: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self { overwrite: false, rebuild_indexes: true }
    }
}

/// Result of a successful import operation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportResult {
    /// Absolute path to the restored knowledge base directory.
    pub knowledge_base: PathBuf,
    /// Number of files extracted from the archive.
    pub extracted_files: usize,
    /// Total uncompressed bytes extracted.
    pub total_bytes: u64,
    /// Sections found in the archive.
    pub sections: Vec<String>,
    /// Rebuild statistics (if `rebuild_indexes` was enabled and wiki/ was present).
    pub rebuild_stats: Option<RebuildMeta>,
}

/// Lightweight rebuild statistics for the import result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RebuildMeta {
    pub fulltext_count: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub vector_count: usize,
}

/// Restore a knowledge base from a portable `.tar.gz` archive.
///
/// # Behaviour
///
/// - Rejects import if target directory exists and is non-empty (unless `overwrite` is true).
///   When `overwrite` is true, existing files are overwritten; directories are merged.
/// - After extraction, optionally rebuilds all search indexes (`rebuild_indexes`).
/// - Archives produced by [`export_knowledge_base`] are guaranteed round-trip compatible.
pub fn import_knowledge_base(
    archive_path: &Path,
    target: &Path,
    options: ImportOptions,
) -> PdfResult<ImportResult> {
    if !archive_path.exists() {
        return Err(PdfModuleError::FileNotFound(archive_path.to_string_lossy().to_string()));
    }

    // Validate archive extension
    let has_valid_ext = archive_path.extension().map(|e| e == "gz").unwrap_or(false)
        || archive_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(".tar.gz"))
            .unwrap_or(false);

    if !has_valid_ext {
        return Err(PdfModuleError::Storage(format!(
            "Archive must be a .tar.gz file: {}",
            archive_path.display()
        )));
    }

    // Check target directory
    if target.exists() {
        if !target.is_dir() {
            return Err(PdfModuleError::Storage(format!(
                "Target path exists and is not a directory: {}",
                target.display()
            )));
        }

        let is_non_empty = target
            .read_dir()
            .map_err(|e| PdfModuleError::Storage(format!("Failed to read target dir: {e}")))?
            .filter_map(|e| e.ok())
            .any(|e| {
                let n = e.file_name();
                let s = n.to_string_lossy();
                s != "." && s != ".."
            });

        if is_non_empty && !options.overwrite {
            return Err(PdfModuleError::Storage(format!(
                "Target directory is not empty: {}. Use overwrite flag to proceed.",
                target.display()
            )));
        }
    }

    fs::create_dir_all(target).map_err(|e| {
        PdfModuleError::Storage(format!("Failed to create target dir {}: {e}", target.display()))
    })?;

    // Open and decompress the archive
    let file = fs::File::open(archive_path).map_err(|e| {
        PdfModuleError::Storage(format!("Failed to open archive {}: {e}", archive_path.display()))
    })?;
    let reader = BufReader::new(file);
    let decoder = GzDecoder::new(reader);
    let mut archive = tar::Archive::new(decoder);

    let mut extracted_files: usize = 0;
    let mut total_bytes: u64 = 0;
    let mut sections: Vec<String> = Vec::new();

    // Extract all entries
    for entry_result in archive
        .entries()
        .map_err(|e| PdfModuleError::Storage(format!("Failed to read archive entries: {e}")))?
    {
        let mut entry = entry_result
            .map_err(|e| PdfModuleError::Storage(format!("Failed to read archive entry: {e}")))?;

        let path = entry
            .path()
            .map_err(|e| PdfModuleError::Storage(format!("Failed to read entry path: {e}")))?;

        let path_str = path.to_string_lossy().to_string();

        // Track top-level sections
        if let Some(section) = path.components().next().and_then(|c| c.as_os_str().to_str())
            && !sections.contains(&section.to_string())
            && !section.starts_with('.')
            && (section == "schema"
                || section == "raw"
                || section == "wiki"
                || section == ".rsut_index")
        {
            sections.push(section.to_string());
        }

        let size = entry.size();

        debug!(path = %path_str, "Extracting");

        entry
            .unpack_in(target)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to extract {}: {e}", path_str)))?;

        extracted_files += 1;
        total_bytes += size;
    }

    info!(
        archive = %archive_path.display(),
        target = %target.display(),
        files = extracted_files,
        bytes = total_bytes,
        sections = ?sections,
        "Knowledge base imported successfully",
    );

    // Optionally rebuild indexes
    let rebuild_stats = if options.rebuild_indexes && sections.contains(&"wiki".to_string()) {
        Some(rebuild_indexes_after_import(target)?)
    } else if options.rebuild_indexes {
        warn!("No wiki/ found in archive, skipping index rebuild");
        None
    } else {
        None
    };

    let kb_path = target.canonicalize().unwrap_or_else(|_| target.to_path_buf());

    Ok(ImportResult {
        knowledge_base: kb_path,
        extracted_files,
        total_bytes,
        sections,
        rebuild_stats,
    })
}

/// Rebuild all search indexes after import.
fn rebuild_indexes_after_import(kb_path: &Path) -> PdfResult<RebuildMeta> {
    // We use the facade rebuild to avoid pulling in WAL-heavy dependencies here.
    let stats = crate::knowledge::index::facade::rebuild_all(kb_path)?;
    Ok(RebuildMeta {
        fulltext_count: stats.fulltext_entries_indexed,
        graph_nodes: stats.graph_nodes,
        graph_edges: stats.graph_edges,
        vector_count: stats.vector_entries_indexed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::export::{ExportOptions, export_knowledge_base};
    use std::fs;
    use tempfile::TempDir;

    fn create_minimal_kb(dir: &Path) {
        fs::create_dir_all(dir.join("schema")).unwrap();
        fs::create_dir_all(dir.join("raw")).unwrap();
        fs::create_dir_all(dir.join("wiki/IT")).unwrap();
        fs::write(dir.join("schema/AGENTS.md"), "# Rules\n").unwrap();
        fs::write(dir.join("wiki/index.md"), "# Index\n").unwrap();
        fs::write(dir.join("wiki/log.md"), "# Log\n").unwrap();
        fs::write(
            dir.join("wiki/IT/nginx.md"),
            "---\ntitle: Nginx\ndomain: IT\n---\n\n# Content\n",
        )
        .unwrap();
    }

    #[test]
    fn test_export_import_roundtrip() {
        let dir = TempDir::new().unwrap();
        let kb = dir.path().join("kb");
        create_minimal_kb(&kb);
        fs::write(kb.join("raw/paper.md"), "# Paper\n").unwrap();

        let archive = dir.path().join("export.tar.gz");

        // Export
        export_knowledge_base(&kb, &archive, ExportOptions::default()).unwrap();

        // Import to new location
        let restored = dir.path().join("restored");
        let result = import_knowledge_base(
            &archive,
            &restored,
            ImportOptions { overwrite: false, rebuild_indexes: false },
        )
        .unwrap();

        assert!(restored.join("schema/AGENTS.md").exists());
        assert!(restored.join("wiki/index.md").exists());
        assert!(restored.join("wiki/IT/nginx.md").exists());
        assert!(restored.join("raw/paper.md").exists());
        assert!(result.sections.contains(&"schema".to_string()));
        assert!(result.sections.contains(&"wiki".to_string()));
        assert!(result.extracted_files >= 4);
    }

    #[test]
    fn test_import_rejects_existing_nonempty_dir() {
        let dir = TempDir::new().unwrap();
        let kb = dir.path().join("kb");
        create_minimal_kb(&kb);

        let archive = dir.path().join("export.tar.gz");
        export_knowledge_base(&kb, &archive, ExportOptions::default()).unwrap();

        // Create target with existing file
        let target = dir.path().join("target");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("existing.txt"), "data").unwrap();

        let err = import_knowledge_base(
            &archive,
            &target,
            ImportOptions { overwrite: false, rebuild_indexes: false },
        );
        assert!(err.is_err());
    }

    #[test]
    fn test_import_overwrite_merges() {
        let dir = TempDir::new().unwrap();
        let kb = dir.path().join("kb");
        create_minimal_kb(&kb);

        let archive = dir.path().join("export.tar.gz");
        export_knowledge_base(&kb, &archive, ExportOptions::default()).unwrap();

        let target = dir.path().join("target");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("existing.txt"), "data").unwrap();

        let result = import_knowledge_base(
            &archive,
            &target,
            ImportOptions { overwrite: true, rebuild_indexes: false },
        )
        .unwrap();

        assert!(target.join("schema/AGENTS.md").exists());
        assert!(target.join("existing.txt").exists());
        assert!(result.extracted_files > 0);
    }

    #[test]
    fn test_import_invalid_archive() {
        let dir = TempDir::new().unwrap();
        let bad = dir.path().join("bad.txt");
        fs::write(&bad, "not an archive").unwrap();

        let target = dir.path().join("target");
        let err = import_knowledge_base(&bad, &target, ImportOptions::default());
        assert!(err.is_err());
    }

    #[test]
    fn test_import_nonexistent_archive() {
        let dir = TempDir::new().unwrap();
        let err = import_knowledge_base(
            &dir.path().join("nonexistent.tar.gz"),
            &dir.path().join("target"),
            ImportOptions::default(),
        );
        assert!(err.is_err());
    }
}
