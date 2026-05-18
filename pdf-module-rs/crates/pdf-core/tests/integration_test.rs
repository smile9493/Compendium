use pdf_core::{FileValidator, PathValidationConfig};
use std::path::PathBuf;

#[test]
fn test_path_validation_accepts_pdf() {
    let config = PathValidationConfig {
        allowed_extensions: vec!["pdf".into()],
        max_file_size_mb: Some(256),
        ..Default::default()
    };
    let validator = FileValidator::new(config);
    let path = PathBuf::from("test.pdf");
    assert!(validator.validate(&path).is_ok());
}

#[test]
fn test_path_validation_rejects_exe() {
    let config = PathValidationConfig::default();
    let validator = FileValidator::new(config);
    let path = PathBuf::from("malware.exe");
    assert!(validator.validate(&path).is_err());
}
