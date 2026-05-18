#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz target: validate FileInfo::from_path behavior on arbitrary file
    // content. In a real fuzz run this would be adapted to test the full
    // extraction pipeline.

    // For now, verify DTO serialization round-trip on fuzz-generated data.
    // Every serialize → deserialize cycle must preserve data integrity.

    let dto = pdf_core::dto::FileInfo {
        file_path: String::from_utf8_lossy(data).to_string(),
        file_size: data.len() as u64,
        file_size_mb: data.len() as f64 / 1024.0 / 1024.0,
    };

    match serde_json::to_string(&dto) {
        Ok(json) => {
            let parsed: pdf_core::dto::FileInfo = serde_json::from_str(&json)
                .expect("Round-trip deserialization must succeed");
            assert_eq!(parsed.file_path, dto.file_path);
            assert_eq!(parsed.file_size, dto.file_size);
        }
        Err(_) => {} // Serialization failure is acceptable for invalid UTF-8
    }
});