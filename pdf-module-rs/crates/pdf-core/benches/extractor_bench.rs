//! Performance benchmarks for the PDF file validator.
//!
//! Run: `cargo bench -p pdf-core -- extractor_bench`

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use pdf_core::{FileValidator, PathValidationConfig};
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

fn create_pdf_tempfile(size_kb: usize) -> NamedTempFile {
    let mut file = NamedTempFile::with_suffix(".pdf").unwrap();
    let header = b"%PDF-1.4\n1 0 obj<</Type/Catalog>>endobj\nxref\n0 1\n0000000000 65535 f \ntrailer<</Root 1 0 R>>\n%%EOF\n";
    file.write_all(header).unwrap();
    let header_len = header.len();
    if size_kb * 1024 > header_len {
        let padding = vec![b'\n'; size_kb * 1024 - header_len];
        file.write_all(&padding).unwrap();
    }
    file.flush().unwrap();
    file
}

fn bench_validate_small_pdf(c: &mut Criterion) {
    let validator = FileValidator::new(200);
    let temp_file = create_pdf_tempfile(1);

    c.bench_function("validate/small_pdf_1kb", |b| {
        b.iter(|| {
            let result = validator.validate(temp_file.path());
            black_box(result)
        });
    });
}

fn bench_validate_large_pdf(c: &mut Criterion) {
    let validator = FileValidator::new(200);
    let temp_file = create_pdf_tempfile(1024);

    c.bench_function("validate/large_pdf_1mb", |b| {
        b.iter(|| {
            let result = validator.validate(temp_file.path());
            black_box(result)
        });
    });
}

fn bench_validate_nonexistent(c: &mut Criterion) {
    let validator = FileValidator::new(200);
    let path = Path::new("/nonexistent/bench_file.pdf");

    c.bench_function("validate/nonexistent_path", |b| {
        b.iter(|| {
            let result = validator.validate(path);
            black_box(result)
        });
    });
}

fn bench_validate_invalid_extension(c: &mut Criterion) {
    let validator = FileValidator::new(200);
    let tmpdir = tempfile::tempdir().unwrap();
    let path = tmpdir.path().join("test.txt");
    std::fs::write(&path, b"not a pdf").unwrap();

    c.bench_function("validate/invalid_extension", |b| {
        b.iter(|| {
            let result = validator.validate(&path);
            black_box(result)
        });
    });
}

fn bench_path_safety_ok(c: &mut Criterion) {
    let config = PathValidationConfig::default();
    let tmpdir = tempfile::tempdir().unwrap();
    let path = tmpdir.path().join("safe.pdf");
    std::fs::write(&path, b"%PDF-1.4\n%%EOF\n").unwrap();
    let canonical = path.canonicalize().unwrap();

    c.bench_function("path_safety/safe_path", |b| {
        b.iter(|| {
            let result = FileValidator::validate_path_safety(&canonical, &config);
            black_box(result)
        });
    });
}

fn bench_path_safety_traversal(c: &mut Criterion) {
    let config = PathValidationConfig { allow_traversal: false, ..Default::default() };

    c.bench_function("path_safety/traversal_path", |b| {
        b.iter(|| {
            let path = Path::new("/safe/../../../etc/passwd.pdf");
            let result = FileValidator::validate_path_safety(path, &config);
            black_box(result)
        });
    });
}

fn bench_path_safety_long_path(c: &mut Criterion) {
    let config = PathValidationConfig::default();
    let long_path = "/a".repeat(512) + ".pdf";

    c.bench_function("path_safety/long_path_1k_chars", |b| {
        b.iter(|| {
            let path = Path::new(&long_path);
            let result = FileValidator::validate_path_safety(path, &config);
            black_box(result)
        });
    });
}

fn bench_validate_throughput(c: &mut Criterion) {
    let validator = FileValidator::new(200);
    let temp_file = create_pdf_tempfile(8);

    let mut group = c.benchmark_group("validate_throughput");
    group.throughput(criterion::Throughput::Bytes(8 * 1024));
    group.bench_function("8kb_file", |b| {
        b.iter(|| {
            let result = validator.validate(temp_file.path());
            black_box(result)
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_validate_small_pdf,
    bench_validate_large_pdf,
    bench_validate_nonexistent,
    bench_validate_invalid_extension,
    bench_path_safety_ok,
    bench_path_safety_traversal,
    bench_path_safety_long_path,
    bench_validate_throughput,
);

criterion_main!(benches);
