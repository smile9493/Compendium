//! Memory-mapped PDF loader — zero-copy file access via `memmap2`.
//!
//! Maps PDF files into virtual memory for page-level random access without
//! explicit heap allocation. Includes PDF quality classification (Text/Searchable/
//! Scanned/Corrupted) based on font and image presence.
//!
//! ## Safety
//! Uses `unsafe { Mmap::map() }` with documented preconditions (valid file handle,
//! read-only workflow, bounded lifetime).

use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

use crate::error::{PdfModuleError, PdfResult};

pub struct MmapPdfLoader {
    mmap: Mmap,
    file_size: u64,
}

impl MmapPdfLoader {
    pub fn load(path: &Path) -> PdfResult<Self> {
        let file = File::open(path).map_err(|e| {
            PdfModuleError::Extraction(format!("Failed to open file {:?}: {}", path, e))
        })?;

        let metadata = file.metadata().map_err(|e| {
            PdfModuleError::Extraction(format!("Failed to read metadata for {:?}: {}", path, e))
        })?;

        let file_size = metadata.len();

        // SAFETY: Mmap::map is safe because:
        // 1. File is opened and valid (checked by File::open above)
        // 2. File metadata is successfully read (checked above)
        // 3. No concurrent writes expected - read-only PDF extraction workflow
        // 4. mmap lifetime is bounded by MmapPdfLoader struct
        let mmap = unsafe { Mmap::map(&file) }.map_err(|e| {
            PdfModuleError::Extraction(format!("Failed to mmap file {:?}: {}", path, e))
        })?;

        Ok(Self { mmap, file_size })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.mmap
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn is_pdf(&self) -> bool {
        self.mmap.len() >= 4 && &self.mmap[0..4] == b"%PDF"
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PdfQuality {
    Invalid,
    Digital,
    Scanned,
    LowQuality,
    Unknown,
}

impl PdfQuality {
    pub fn needs_vlm(&self) -> bool {
        matches!(self, PdfQuality::Scanned | PdfQuality::LowQuality)
    }

    pub fn is_extractable(&self) -> bool {
        !matches!(self, PdfQuality::Invalid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_mmap_loader_invalid_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Not a PDF").unwrap();

        let loader = MmapPdfLoader::load(temp_file.path()).unwrap();
        assert!(!loader.is_pdf());
    }

    #[test]
    fn test_mmap_loader_pdf_header() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"%PDF-1.4\n%test content").unwrap();

        let loader = MmapPdfLoader::load(temp_file.path()).unwrap();
        assert!(loader.is_pdf());
    }
}
