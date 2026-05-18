use pdf_core::FileValidator;
use std::io::Write;
use tempfile::NamedTempFile;

fn write_minimal_pdf(file: &mut NamedTempFile) {
    let header = b"%PDF-1.4\n1 0 obj<</Type/Catalog>>endobj\nxref\n0 1\n0000000000 65535 f \ntrailer<</Root 1 0 R>>\n%%EOF\n";
    file.write_all(header).expect("write pdf header");
}

#[test]
fn test_path_validation_accepts_pdf() {
    let mut file = NamedTempFile::with_suffix(".pdf").expect("temp pdf");
    write_minimal_pdf(&mut file);
    let validator = FileValidator::new(256);
    assert!(validator.validate(file.path()).is_ok());
}

#[test]
fn test_path_validation_rejects_exe() {
    let mut file = NamedTempFile::with_suffix(".exe").expect("temp exe");
    file.write_all(b"MZ").expect("write exe stub");
    let validator = FileValidator::new(256);
    assert!(validator.validate(file.path()).is_err());
}
