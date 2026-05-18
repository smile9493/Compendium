//! Snapshot tests for DTO serialization.
//!
//! Uses the `insta` crate to ensure serialized output remains stable
//! across refactors. If a snapshot test fails intentionally (due to
//! deliberate format change), update snapshots with:
//!
//! ```bash
//! cargo test -p pdf-core -- snapshot -- --ignored
//! # Or review and accept:
//! cargo insta review
//! ```
//!
//! Install cargo-insta: `cargo install cargo-insta`

use pdf_core::dto::{FileInfo, TextExtractionResult};
use pretty_assertions::assert_eq;

#[test]
fn snapshot_file_info_default() {
    let info = FileInfo {
        file_path: "/tmp/test-file.pdf".to_string(),
        file_size: 102400,
        file_size_mb: 0.10,
    };

    insta::assert_yaml_snapshot!("file_info_default", &info);
}

#[test]
fn snapshot_file_info_large() {
    let info = FileInfo {
        file_path: "/data/very-large-report.pdf".to_string(),
        file_size: 512 * 1024 * 1024,
        file_size_mb: 512.0,
    };

    insta::assert_json_snapshot!("file_info_large", &info);
}

#[test]
fn snapshot_file_info_small() {
    let info = FileInfo {
        file_path: "/tmp/a.pdf".to_string(),
        file_size: 1,
        file_size_mb: 0.0,
    };

    insta::assert_yaml_snapshot!("file_info_small", &info);
}

#[test]
fn snapshot_text_extraction_result() {
    let result = TextExtractionResult {
        extracted_text: "Hello World\nThis is page 1 content.".to_string(),
        extraction_metadata: None,
    };

    insta::assert_json_snapshot!("text_extraction_result", &result);
}

#[test]
fn snapshot_text_extraction_with_metadata() {
    use pdf_common::dto::TextExtractionMetadata;

    let result = TextExtractionResult {
        extracted_text: "Structured content".to_string(),
        extraction_metadata: Some(TextExtractionMetadata {
            whisper_hash: "abc123def456".to_string(),
            line_metadata: Some(serde_json::json!({
                "lines": 42,
                "encoding": "utf-8",
                "has_images": false
            })),
        }),
    };

    insta::assert_yaml_snapshot!("text_extraction_with_metadata", &result);
}

#[test]
fn snapshot_error_status_codes() {
    // Use insta to verify error → HTTP status code mappings remain stable
    use pdf_core::error::PdfModuleError;

    let status_codes: Vec<(&str, u16)> = vec![
        ("FileNotFound", PdfModuleError::FileNotFound("x".into()).status_code()),
        ("InvalidFileType", PdfModuleError::InvalidFileType("x".into()).status_code()),
        ("FileTooLarge", PdfModuleError::FileTooLarge("x".into()).status_code()),
        ("CorruptedFile", PdfModuleError::CorruptedFile("x".into()).status_code()),
        ("Timeout", PdfModuleError::Timeout(5000).status_code()),
        ("Extraction", PdfModuleError::Extraction("x".into()).status_code()),
    ];

    insta::assert_yaml_snapshot!("error_status_codes", &status_codes);
}

// Manual structural assertions complement snapshot tests
#[test]
fn file_info_roundtrip_via_json() {
    let original = FileInfo {
        file_path: "/test/sample.pdf".to_string(),
        file_size: 4096,
        file_size_mb: 0.0039,
    };

    let json = serde_json::to_string(&original).expect("Serialization must succeed");
    let restored: FileInfo = serde_json::from_str(&json).expect("Deserialization must succeed");

    assert_eq!(original.file_path, restored.file_path);
    assert_eq!(original.file_size, restored.file_size);
    assert!((original.file_size_mb - restored.file_size_mb).abs() < 0.001);
}