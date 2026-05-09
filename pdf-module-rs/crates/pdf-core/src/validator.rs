//! File validator with deep inspection
//! Corresponds to Python: validators.py

use crate::dto::FileInfo;
use crate::error::{PdfModuleError, PdfResult};
use std::path::Path;

const ALLOWED_EXTENSIONS: &[&str] = &[".pdf"];

// Re-export PathValidationConfig from pdf-common (unified source of truth).
pub use pdf_common::config::PathValidationConfig;

/// File validator with four-level validation chain
/// Corresponds to Python: FileValidator
pub struct FileValidator {
    max_size_bytes: u64,
}

impl FileValidator {
    /// Create a new validator with max file size in MB
    pub fn new(max_size_mb: u32) -> Self {
        Self {
            max_size_bytes: max_size_mb as u64 * 1024 * 1024,
        }
    }

    /// Validate a file path
    /// Corresponds to Python: FileValidator.validate()
    pub fn validate(&self, file_path: &Path) -> PdfResult<FileInfo> {
        // 1. Check file exists
        if !file_path.exists() {
            return Err(PdfModuleError::FileNotFound(
                file_path.to_string_lossy().to_string(),
            ));
        }

        // 2. Check extension
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let ext_with_dot = format!(".{}", ext.to_lowercase());
        if !ALLOWED_EXTENSIONS.contains(&ext_with_dot.as_str()) {
            return Err(PdfModuleError::InvalidFileType(format!(
                "Invalid extension '.{}', allowed: {:?}",
                ext, ALLOWED_EXTENSIONS
            )));
        }

        // 3. Check file size
        let file_size = std::fs::metadata(file_path)
            .map_err(|e| PdfModuleError::CorruptedFile(e.to_string()))?
            .len();

        if file_size > self.max_size_bytes {
            return Err(PdfModuleError::FileTooLarge(format!(
                "File size {:.1}MB exceeds limit of {}MB",
                file_size as f64 / 1024.0 / 1024.0,
                self.max_size_bytes / 1024 / 1024
            )));
        }

        if file_size == 0 {
            return Err(PdfModuleError::CorruptedFile("File is empty".to_string()));
        }

        // 4. Deep file inspection using infer crate
        let inferred_type = infer::get_from_path(file_path).map_err(|e| {
            PdfModuleError::CorruptedFile(format!("Cannot read file for sniffing: {}", e))
        })?;

        match inferred_type {
            Some(t) if t.mime_type() == "application/pdf" => {
                // Valid PDF
            }
            Some(t) => {
                // File content type mismatch - possible malicious upload
                return Err(PdfModuleError::InvalidFileType(format!(
                    "File content type mismatch: extension is .pdf but actual type is {} ({}). \
                     Possible malicious file upload attempt.",
                    t.mime_type(),
                    t.extension()
                )));
            }
            None => {
                // infer couldn't identify, fallback to %PDF header check
                let mut file = std::fs::File::open(file_path)
                    .map_err(|e| PdfModuleError::CorruptedFile(e.to_string()))?;
                let mut header = [0u8; 4];
                std::io::Read::read_exact(&mut file, &mut header).map_err(|e| {
                    PdfModuleError::CorruptedFile(format!("Cannot read header: {}", e))
                })?;
                if &header != b"%PDF" {
                    return Err(PdfModuleError::CorruptedFile(format!(
                        "Not a valid PDF, header: {:?}",
                        header
                    )));
                }
            }
        }

        FileInfo::from_path(file_path).map_err(PdfModuleError::Io)
    }

    /// Validate path safety to prevent path traversal attacks
    /// Returns Ok(()) if path is safe, Err otherwise
    pub fn validate_path_safety(path: &Path, config: &PathValidationConfig) -> PdfResult<()> {
        let path_str = path.to_string_lossy();

        // 1. Check for path traversal attempts
        if !config.allow_traversal {
            // Check for ".." components
            for component in path.components() {
                if matches!(component, std::path::Component::ParentDir) {
                    return Err(PdfModuleError::InvalidFileType(
                        "Path traversal detected: '..' not allowed".to_string(),
                    ));
                }
            }

            // Also check for encoded traversal attempts
            if path_str.contains("..") {
                return Err(PdfModuleError::InvalidFileType(
                    "Path traversal detected in path string".to_string(),
                ));
            }
        }

        // 2. Check if absolute path is required
        if config.require_absolute && !path.is_absolute() {
            return Err(PdfModuleError::InvalidFileType(
                "Only absolute paths are allowed".to_string(),
            ));
        }

        // 3. Check file extension
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let ext_with_dot = format!(".{}", ext.to_lowercase());
        if !ALLOWED_EXTENSIONS.contains(&ext_with_dot.as_str()) {
            return Err(PdfModuleError::InvalidFileType(format!(
                "Invalid file extension '.{}', only PDF files are allowed",
                ext
            )));
        }

        // 4. If base_dir is set, verify path is within base directory
        if let Some(base_dir) = &config.base_dir {
            let canonical_path = path
                .canonicalize()
                .map_err(|e| PdfModuleError::FileNotFound(format!("Cannot resolve path: {}", e)))?;
            let canonical_base = base_dir.canonicalize().map_err(|e| {
                PdfModuleError::InvalidFileType(format!("Invalid base directory: {}", e))
            })?;

            if !canonical_path.starts_with(&canonical_base) {
                return Err(PdfModuleError::InvalidFileType(
                    "Path is outside allowed directory".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Validate upload parameters
    /// Corresponds to Python: FileValidator.validate_upload()
    pub fn validate_upload(&self, filename: &str, content_length: Option<u64>) -> PdfResult<()> {
        if filename.is_empty() {
            return Err(PdfModuleError::InvalidFileType(
                "No filename provided".to_string(),
            ));
        }

        let ext = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let ext_with_dot = format!(".{}", ext.to_lowercase());
        if !ALLOWED_EXTENSIONS.contains(&ext_with_dot.as_str()) {
            return Err(PdfModuleError::InvalidFileType(format!(
                "Invalid extension '.{}'",
                ext
            )));
        }

        if let Some(len) = content_length {
            if len > self.max_size_bytes {
                return Err(PdfModuleError::FileTooLarge(format!(
                    "Upload size {:.1}MB exceeds limit of {}MB",
                    len as f64 / 1024.0 / 1024.0,
                    self.max_size_bytes / 1024 / 1024
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_file_not_found() {
        let validator = FileValidator::new(200);
        let result = validator.validate(Path::new("/nonexistent/file.pdf"));
        assert!(matches!(result, Err(PdfModuleError::FileNotFound(_))));
    }

    #[test]
    fn test_invalid_extension() {
        let validator = FileValidator::new(200);
        let mut temp_file = NamedTempFile::with_suffix(".txt").unwrap();
        write!(temp_file, "test content").unwrap();

        let result = validator.validate(temp_file.path());
        assert!(matches!(result, Err(PdfModuleError::InvalidFileType(_))));
    }

    #[test]
    fn test_empty_file() {
        let validator = FileValidator::new(200);
        let temp_file = NamedTempFile::with_suffix(".pdf").unwrap();

        let result = validator.validate(temp_file.path());
        assert!(matches!(result, Err(PdfModuleError::CorruptedFile(_))));
    }

    #[test]
    fn test_validate_upload() {
        let validator = FileValidator::new(200);

        // Valid upload
        let result = validator.validate_upload("test.pdf", Some(1024));
        assert!(result.is_ok());

        // Invalid extension
        let result = validator.validate_upload("test.txt", Some(1024));
        assert!(matches!(result, Err(PdfModuleError::InvalidFileType(_))));

        // Too large
        let result = validator.validate_upload("test.pdf", Some(300 * 1024 * 1024));
        assert!(matches!(result, Err(PdfModuleError::FileTooLarge(_))));
    }
}
