//! Knowledge base export — pack a KB directory into a portable `.tar.gz` archive.
//!
//! Produces a compressed archive containing the human-authored knowledge:
//! `schema/`, `raw/`, `wiki/`, and optionally machine-rebuildable indexes.

use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::write::GzEncoder;
use tracing::{debug, info};

use crate::error::{PdfModuleError, PdfResult};

/// Controls what gets included in the export archive.
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Include `.rsut_index/` (Tantivy, vectors, graph). These are rebuildable.
    pub include_indexes: bool,
    /// Include `.hash_cache` (Merkle incremental-compile cache). Rebuildable.
    pub include_hash_cache: bool,
    /// Gzip compression level 0-9 (default: 6).
    pub compression_level: u32,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self { include_indexes: false, include_hash_cache: false, compression_level: 6 }
    }
}

/// Result of a successful export operation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExportResult {
    /// Absolute path to the created archive file.
    pub archive_path: PathBuf,
    /// Number of files included in the archive.
    pub total_files: usize,
    /// Total uncompressed bytes written.
    pub total_bytes: u64,
    /// Which sections were included in the archive.
    pub sections: Vec<String>,
}

/// Core knowledge directories that are always included in exports.
const CORE_DIRS: &[&str] = &["schema", "raw", "wiki"];

/// Pack a knowledge base directory into a portable `.tar.gz` archive.
///
/// - Always includes: `schema/`, `raw/`, `wiki/`
/// - Optionally includes: `.rsut_index/`, `.hash_cache`
/// - Skips `.versions/` directories inside `raw/` and `wiki/` to keep archives lean
pub fn export_knowledge_base(
    kb_path: &Path,
    output: &Path,
    options: ExportOptions,
) -> PdfResult<ExportResult> {
    if !kb_path.exists() {
        return Err(PdfModuleError::FileNotFound(kb_path.to_string_lossy().to_string()));
    }
    if !kb_path.is_dir() {
        return Err(PdfModuleError::Storage(format!(
            "Knowledge base path is not a directory: {}",
            kb_path.display()
        )));
    }

    // Verify at least one core directory exists
    let mut found_core = false;
    for dir in CORE_DIRS {
        if kb_path.join(dir).is_dir() {
            found_core = true;
            break;
        }
    }
    if !found_core {
        return Err(PdfModuleError::Storage(format!(
            "Knowledge base at {} has no schema/, raw/, or wiki/ directories",
            kb_path.display()
        )));
    }

    // Create output file
    let file = fs::File::create(output).map_err(|e| {
        PdfModuleError::Storage(format!("Failed to create archive {}: {e}", output.display()))
    })?;
    let buf = BufWriter::new(file);

    // Build gzip-compressed tar
    let encoder = GzEncoder::new(buf, Compression::new(options.compression_level));
    let mut archive = tar::Builder::new(encoder);
    let mut total_files: usize = 0;
    let mut total_bytes: u64 = 0;
    let mut sections: Vec<String> = Vec::new();

    // Helper: add a directory tree to the archive (recursive, no extra deps)
    fn add_dir_to_archive<W: Write>(
        archive: &mut tar::Builder<W>,
        kb_path: &Path,
        dir_rel: &str,
        skip_dirs: &[&str],
        file_count: &mut usize,
        byte_count: &mut u64,
    ) -> PdfResult<()> {
        let dir_path = kb_path.join(dir_rel);
        if !dir_path.exists() || !dir_path.is_dir() {
            debug!(dir = %dir_rel, "Directory not found, skipping");
            return Ok(());
        }

        add_path_recursive(archive, kb_path, &dir_path, skip_dirs, file_count, byte_count)?;
        Ok(())
    }

    fn add_path_recursive<W: Write>(
        archive: &mut tar::Builder<W>,
        kb_root: &Path,
        current: &Path,
        skip_dirs: &[&str],
        file_count: &mut usize,
        byte_count: &mut u64,
    ) -> PdfResult<()> {
        for entry in fs::read_dir(current).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to read dir {}: {e}", current.display()))
        })? {
            let entry = entry.map_err(|e| {
                PdfModuleError::Storage(format!(
                    "Failed to read entry in {}: {e}",
                    current.display()
                ))
            })?;
            let entry_path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Compute relative path within the KB root
            let rel = entry_path
                .strip_prefix(kb_root)
                .map_err(|e| PdfModuleError::Storage(format!("Path strip error: {e}")))?;

            let meta = entry.metadata().map_err(|e| {
                PdfModuleError::Storage(format!(
                    "Failed to read metadata for {}: {e}",
                    entry_path.display()
                ))
            })?;

            if meta.is_dir() {
                if skip_dirs.iter().any(|s| *s == file_name.as_str()) {
                    debug!(dir = %entry_path.display(), "Skipping excluded directory");
                    continue;
                }
                archive.append_dir(rel, &entry_path).map_err(|e| {
                    PdfModuleError::Storage(format!(
                        "Failed to append dir {}: {e}",
                        entry_path.display()
                    ))
                })?;
                add_path_recursive(
                    archive,
                    kb_root,
                    &entry_path,
                    skip_dirs,
                    file_count,
                    byte_count,
                )?;
            } else if meta.is_file() {
                archive.append_path_with_name(&entry_path, rel).map_err(|e| {
                    PdfModuleError::Storage(format!(
                        "Failed to append file {}: {e}",
                        entry_path.display()
                    ))
                })?;
                *file_count += 1;
                *byte_count += meta.len();
            }
        }
        Ok(())
    }

    // Always include core directories (excluding .versions/)
    let version_dirs = &[".versions"];
    for dir in CORE_DIRS {
        let dir_path = kb_path.join(dir);
        if dir_path.is_dir() {
            debug!(dir = %dir, "Adding core directory to archive");
            add_dir_to_archive(
                &mut archive,
                kb_path,
                dir,
                version_dirs,
                &mut total_files,
                &mut total_bytes,
            )?;
            sections.push(dir.to_string());
        }
    }

    // Optionally include .rsut_index/
    if options.include_indexes {
        let index_dir = kb_path.join(".rsut_index");
        if index_dir.is_dir() {
            debug!("Adding .rsut_index to archive");
            add_dir_to_archive(
                &mut archive,
                kb_path,
                ".rsut_index",
                &[], // no subdirs to skip
                &mut total_files,
                &mut total_bytes,
            )?;
            sections.push(".rsut_index".to_string());
        } else {
            info!(".rsut_index not found, skipping");
        }
    }

    // Optionally include .hash_cache
    if options.include_hash_cache {
        let hash_path = kb_path.join(".hash_cache");
        if hash_path.is_file() {
            let metadata = fs::metadata(&hash_path).map_err(|e| {
                PdfModuleError::Storage(format!("Failed to read .hash_cache metadata: {e}"))
            })?;
            let rel = Path::new(".hash_cache");
            archive.append_path_with_name(&hash_path, rel).map_err(|e| {
                PdfModuleError::Storage(format!("Failed to append .hash_cache: {e}"))
            })?;
            total_files += 1;
            total_bytes += metadata.len();
            sections.push(".hash_cache".to_string());
        } else {
            info!(".hash_cache not found, skipping");
        }
    }

    // Finalize the archive
    let encoder = archive
        .into_inner()
        .map_err(|e| PdfModuleError::Storage(format!("Failed to finalize tar: {e}")))?;
    let _buf = encoder
        .finish()
        .map_err(|e| PdfModuleError::Storage(format!("Failed to finish gzip: {e}")))?;

    let archive_path = output.canonicalize().unwrap_or_else(|_| output.to_path_buf());

    info!(
        path = %archive_path.display(),
        files = total_files,
        bytes = total_bytes,
        sections = ?sections,
        "Knowledge base exported successfully",
    );

    Ok(ExportResult { archive_path, total_files, total_bytes, sections })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_minimal_kb(dir: &Path) {
        fs::create_dir_all(dir.join("schema")).unwrap();
        fs::create_dir_all(dir.join("raw")).unwrap();
        fs::create_dir_all(dir.join("wiki/IT")).unwrap();
        fs::write(dir.join("schema/AGENTS.md"), "# Rules\n\nBe helpful.\n").unwrap();
        fs::write(dir.join("wiki/index.md"), "# Index\n\n- [IT] test\n").unwrap();
        fs::write(dir.join("wiki/log.md"), "# Log\n- 2026-01-01 extract\n").unwrap();
        fs::write(
            dir.join("wiki/IT/nginx.md"),
            "---\ntitle: Nginx\ndomain: IT\n---\n\n# Content\n",
        )
        .unwrap();
        fs::create_dir_all(dir.join(".rsut_index/tantivy")).unwrap();
        fs::write(dir.join(".rsut_index/config.json"), r#"{"key":"val"}"#).unwrap();
        fs::write(dir.join(".hash_cache"), r#"{"merkle_root":"","leaf_paths":[],"entries":{}}"#)
            .unwrap();
    }

    #[test]
    fn test_export_minimal() {
        let dir = TempDir::new().unwrap();
        let kb = dir.path().join("kb");
        create_minimal_kb(&kb);
        let output = dir.path().join("export.tar.gz");

        let result = export_knowledge_base(&kb, &output, ExportOptions::default()).unwrap();

        assert!(output.exists());
        assert!(result.total_files >= 4);
        assert!(result.sections.contains(&"schema".to_string()));
        assert!(result.sections.contains(&"raw".to_string()));
        assert!(result.sections.contains(&"wiki".to_string()));
        assert!(!result.sections.contains(&".rsut_index".to_string()));
    }

    #[test]
    fn test_export_with_indexes() {
        let dir = TempDir::new().unwrap();
        let kb = dir.path().join("kb");
        create_minimal_kb(&kb);
        let output = dir.path().join("export_with_indexes.tar.gz");

        let result = export_knowledge_base(
            &kb,
            &output,
            ExportOptions { include_indexes: true, include_hash_cache: true, ..Default::default() },
        )
        .unwrap();

        assert!(output.exists());
        assert!(result.sections.contains(&".rsut_index".to_string()));
        assert!(result.sections.contains(&".hash_cache".to_string()));
    }

    #[test]
    fn test_export_missing_kb() {
        let dir = TempDir::new().unwrap();
        let output = dir.path().join("nonexistent.tar.gz");
        let err = export_knowledge_base(
            &dir.path().join("nonexistent"),
            &output,
            ExportOptions::default(),
        );
        assert!(err.is_err());
    }

    #[test]
    fn test_export_empty_dir() {
        let dir = TempDir::new().unwrap();
        let kb = dir.path().join("empty");
        fs::create_dir_all(&kb).unwrap();
        let output = dir.path().join("empty.tar.gz");
        let err = export_knowledge_base(&kb, &output, ExportOptions::default());
        assert!(err.is_err());
        let msg = format!("{}", err.unwrap_err());
        assert!(msg.contains("no schema/"));
    }
}
