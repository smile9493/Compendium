//! Regenerate Code Mode SDK artifacts from the tool manifest.
//!
//! Run: `cargo run -p pdf-mcp-contracts --bin generate-sdk`

use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sdk_dir = root.join("../../templates/sdk");
    std::fs::create_dir_all(&sdk_dir).expect("create templates/sdk");

    let dts_path = sdk_dir.join("compendium.d.ts");
    let index_path = sdk_dir.join("compendium-api-index.json");

    std::fs::write(&dts_path, pdf_mcp_contracts::generate_typescript_sdk())
        .unwrap_or_else(|e| panic!("write {}: {e}", dts_path.display()));
    std::fs::write(&index_path, pdf_mcp_contracts::generate_api_index_json())
        .unwrap_or_else(|e| panic!("write {}: {e}", index_path.display()));

    eprintln!("Wrote {}", dts_path.display());
    eprintln!("Wrote {}", index_path.display());
}
