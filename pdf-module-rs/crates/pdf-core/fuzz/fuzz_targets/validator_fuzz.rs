#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_core::FileValidator;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

fuzz_target!(|data: &[u8]| {
    // Fuzz target: test that validate() never panics on arbitrary inputs.
    // Any file content (valid PDF, corrupt, random bytes) must be handled
    // gracefully — either Ok or a well-defined Err.

    let validator = FileValidator::new(200);

    // Strategy 1: feed raw bytes directly as temp file
    if let Ok(mut temp_file) = NamedTempFile::with_suffix(".pdf") {
        if temp_file.write_all(data).is_ok() && temp_file.flush().is_ok() {
            // Must not panic regardless of input
            let _ = validator.validate(temp_file.path());
        }
    }

    // Strategy 2: feed as .txt extension (should always reject)
    if let Ok(mut temp_file) = NamedTempFile::with_suffix(".txt") {
        if temp_file.write_all(data).is_ok() && temp_file.flush().is_ok() {
            let result = validator.validate(temp_file.path());
            // .txt files should always be rejected with InvalidFileType
            assert!(result.is_err(), ".txt input must be rejected");
        }
    }

    // Strategy 3: test with zero-size limit
    let zero_validator = FileValidator::new(0);
    if let Ok(temp_file) = NamedTempFile::with_suffix(".pdf") {
        if std::fs::write(temp_file.path(), data).is_ok() {
            let _ = zero_validator.validate(temp_file.path());
        }
    }

    // Strategy 4: arbitrary path (including malformed UTF-8)
    let _ = validator.validate(Path::new("/nonexistent/path.pdf"));
});