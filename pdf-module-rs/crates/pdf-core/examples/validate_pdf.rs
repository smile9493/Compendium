use pdf_core::FileValidator;
use std::path::Path;

fn main() {
    // max_size_mb=200: allow up to 200MB PDF files
    let validator = FileValidator::new(200);
    let path = Path::new("example.pdf");

    match validator.validate(path) {
        Ok(info) => println!("{path:?} is valid ({:.1}MB)", info.file_size_mb),
        Err(e) => eprintln!("Validation error: {e}"),
    }
}
