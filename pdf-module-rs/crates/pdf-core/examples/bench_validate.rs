use pdf_core::FileValidator;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

// ============================================================================
// 临时性能基准脚手架 — FileValidator::validate()
//
// 未来迁移到 criterion.rs 的步骤（详见文件末尾注释）：
//   1. 将 `fn generate_temp_pdf()` 提取为共享的测试工具函数。
//   2. 将 `fn bench_single_size()` 的核心调用逻辑封装为一个独立的纯函数，
//      接受 validator 引用和 Path 引用，返回 Result。
//   3. 在 benches/ 目录下创建 criterion 基准文件，引用上述函数，
//      使用 `criterion::black_box` 包裹输入路径。
//   4. 通过 `cargo bench --bench <name>` 获得统计显著性报告。
// ============================================================================

const WARMUP_ITERATIONS: u32 = 10;
const BENCH_ITERATIONS: u32 = 100;
const MAX_FILE_SIZE_MB: u32 = 200;

fn main() {
    let temp_dir = std::env::temp_dir().join("pdf_bench_validate");
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir for benchmarks");

    let validator = FileValidator::new(MAX_FILE_SIZE_MB);

    let scales: &[(&str, usize)] = &[
        ("small  (~10 KB)", 10 * 1024),
        ("medium (~1 MB)", 1000 * 1024),
        ("large  (~100 MB)", 100_000 * 1024),
    ];

    // =========================================================================
    // 预热阶段：用中等规模文件预热文件系统缓存和 I/O 路径
    // =========================================================================
    println!("=== Warm-up phase ===");
    let warmup_path = generate_temp_pdf(&temp_dir, "warmup", 1000 * 1024);
    for i in 0..WARMUP_ITERATIONS {
        let _ = validator.validate(&warmup_path);
        if i == 0 {
            println!("  Warm-up running {} iterations ...", WARMUP_ITERATIONS);
        }
    }
    let _ = fs::remove_file(&warmup_path);
    println!("  Warm-up complete.\n");

    // =========================================================================
    // 正式基准测试
    // =========================================================================
    println!("=== Benchmark: FileValidator::validate() ===");
    println!("{:<20} {:>12} {:>14} {:>14}", "Scale", "File Size", "Total (ms)", "Avg (µs/call)");
    println!("{:-<20} {:-<12} {:-<14} {:-<14}", "", "", "", "");

    for &(label, size_bytes) in scales {
        let file_path = generate_temp_pdf(&temp_dir, label, size_bytes);
        let (total_duration, avg_duration) = bench_single_size(&validator, &file_path);
        let _ = fs::remove_file(&file_path);

        let size_display = if size_bytes >= 1024 * 1024 {
            format!("{:.1} MB", size_bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.1} KB", size_bytes as f64 / 1024.0)
        };

        println!(
            "{:<20} {:>12} {:>12.3} {:>12.1}",
            label,
            size_display,
            total_duration.as_secs_f64() * 1000.0,
            avg_duration.as_secs_f64() * 1_000_000.0,
        );
    }

    println!("{:-<20} {:-<12} {:-<14} {:-<14}", "", "", "", "");
    println!(
        "Results based on {} iterations per scale (warmup={}).",
        BENCH_ITERATIONS, WARMUP_ITERATIONS
    );

    // 清理临时目录
    let _ = fs::remove_dir_all(&temp_dir);
}

// =============================================================================
// 生成可控大小的临时 PDF 文件
//
// 使用 `set_len` 创建稀疏文件以避免大文件写入开销污染基准设置阶段。
// 若需要完全分配的物理文件（例如测量真实 I/O 压力），可替换为缓冲写入循环。
// =============================================================================
fn generate_temp_pdf(dir: &Path, label: &str, size_bytes: usize) -> PathBuf {
    let sanitized = label
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>();
    let file_path = dir.join(format!("bench_{}.pdf", sanitized));

    let mut file = fs::File::create(&file_path)
        .unwrap_or_else(|e| panic!("Failed to create temp file {:?}: {}", file_path, e));

    file.write_all(b"%PDF-1.4\n").unwrap_or_else(|e| panic!("Failed to write PDF header: {}", e));

    if size_bytes > 8 {
        file.set_len(size_bytes as u64)
            .unwrap_or_else(|e| panic!("Failed to set file length: {}", e));
    }

    file_path
}

// =============================================================================
// 对单个文件规模运行基准测试
//
// 返回值：(总耗时, 平均单次调用耗时)
// =============================================================================
fn bench_single_size(
    validator: &FileValidator,
    file_path: &Path,
) -> (std::time::Duration, std::time::Duration) {
    let start = Instant::now();
    for _ in 0..BENCH_ITERATIONS {
        let _ = validator.validate(file_path);
    }
    let total = start.elapsed();
    let avg = total / BENCH_ITERATIONS;
    (total, avg)
}

// =============================================================================
// === 未来迁移到 criterion.rs 的指南 ===
//
// criterion 提供统计显著性分析、异常值检测、HTML 报告生成。
// 迁移步骤：
//
// 1. 提取核心逻辑为可复用函数：
//    ```rust
//    pub fn bench_validate(validator: &FileValidator, path: &Path) {
//        let _ = validator.validate(path);
//    }
//    ```
//
// 2. 在 benches/validate_bench.rs 中编写 criterion 基准：
//    ```rust
//    use criterion::{black_box, Criterion};
//
//    pub fn bench_validate_small(c: &mut Criterion) {
//        let validator = FileValidator::new(200);
//        let path = setup_temp_pdf(10 * 1024);
//        c.bench_function("validate_small_10kb", |b| {
//            b.iter(|| bench_validate(&validator, black_box(&path)))
//        });
//    }
//
//    criterion_group!(benches, bench_validate_small, bench_validate_medium, bench_validate_large);
//    criterion_main!(benches);
//    ```
//
// 3. 在 Cargo.toml 的 [[bench]] 中注册：
//    ```toml
//    [[bench]]
//    name = "validate_bench"
//    harness = false
//    ```
//
// 4. 运行：`cargo bench --bench validate_bench`
//
// 5. criterion 会自动进行预热、采样、计算置信区间 (95%)。
//    报告位于 target/criterion/ 目录下。
// =============================================================================
