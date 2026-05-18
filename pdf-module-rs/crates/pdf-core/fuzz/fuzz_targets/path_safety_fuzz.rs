#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_core::{FileValidator, PathValidationConfig};
use std::path::Path;

fuzz_target!(|data: &[u8]| {
    // Fuzz target: test that validate_path_safety() never panics on arbitrary
    // path-like byte sequences. Malformed UTF-8, control characters, very long
    // strings — all must be handled gracefully.

    let config_strict = PathValidationConfig {
        require_absolute: true,
        allow_traversal: false,
        base_dir: None,
    };

    let config_lenient = PathValidationConfig {
        require_absolute: false,
        allow_traversal: true,
        base_dir: None,
    };

    // Convert fuzz bytes to a string-like path (best-effort)
    // Even malformed UTF-8 should not cause panic via to_string_lossy()
    let path_str = String::from_utf8_lossy(data);
    let path = Path::new(path_str.as_ref());

    // Must never panic regardless of input
    let _ = FileValidator::validate_path_safety(path, &config_strict);
    let _ = FileValidator::validate_path_safety(path, &config_lenient);

    // Test with traversal-like inputs that contain ".."
    if data.len() >= 2 {
        let traversal_input = format!("/safe/../{}", path_str);
        let traversal_path = Path::new(&traversal_input);
        let _ = FileValidator::validate_path_safety(traversal_path, &config_strict);
    }
});