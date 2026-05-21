//! Performance benchmarks for validator creation and configuration.
//!
//! Run: `cargo bench -p pdf-core -- validator_bench`

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use pdf_core::{FileValidator, PathValidationConfig};

fn bench_validator_construction(c: &mut Criterion) {
    c.bench_function("validator/new_small_limit", |b| {
        b.iter(|| {
            let v = FileValidator::new(1);
            black_box(v)
        });
    });

    c.bench_function("validator/new_large_limit", |b| {
        b.iter(|| {
            let v = FileValidator::new(1024);
            black_box(v)
        });
    });
}

fn bench_config_creation(c: &mut Criterion) {
    c.bench_function("config/default", |b| {
        b.iter(|| {
            let c = PathValidationConfig::default();
            black_box(c)
        });
    });

    c.bench_function("config/with_base_dir", |b| {
        b.iter(|| {
            let c = PathValidationConfig {
                require_absolute: true,
                allow_traversal: false,
                base_dir: Some(std::path::PathBuf::from("/tmp/allowed")),
            };
            black_box(c)
        });
    });
}

fn bench_clone_validator(c: &mut Criterion) {
    let original = FileValidator::new(200);

    c.bench_function("validator/clone", |b| {
        b.iter(|| {
            let v = original.clone();
            black_box(v)
        });
    });
}

criterion_group!(
    benches,
    bench_validator_construction,
    bench_config_creation,
    bench_clone_validator,
);

criterion_main!(benches);
