use std::fs;
use std::path::Path;

fn main() {
    // Ensure the SPA dist directory exists so rust-embed doesn't fail at compile time.
    // The real SPA build happens before CI test/release steps or via npm run build.
    let dist = Path::new("pdf-web-ui/dist");
    if !dist.exists() {
        fs::create_dir_all(dist).expect("create pdf-web-ui/dist for rust-embed");
        fs::write(dist.join(".gitkeep"), b"").ok();
    }

    // Parse the repo-root VERSION file and expose each field as a compile-time env var.
    // The VERSION file lives at the workspace root (../../.. from this crate).
    let version_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../VERSION");
    println!("cargo:rerun-if-changed={}", version_path.display());

    let mut found_file = false;
    if let Ok(content) = fs::read_to_string(&version_path) {
        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                let env_key = format!("VERSION_{}", key.trim());
                println!("cargo:rustc-env={}={}", env_key, value.trim());
            }
        }
        found_file = true;
    }

    // Fallback: read from environment (for Docker builds without VERSION file).
    // Set PDF_VERSION_MAJOR, PDF_VERSION_MINOR, etc. as build args.
    if !found_file {
        for key in &["MAJOR", "MINOR", "BUILD", "PATCH"] {
            let env_key = format!("VERSION_{}", key);
            if std::env::var(&env_key).is_err()
                && let Ok(val) = std::env::var(format!("PDF_VERSION_{}", key))
            {
                println!("cargo:rustc-env={}={}", env_key, val);
            }
        }
    }

    // Also expose CARGO_PKG_VERSION for the semver string
    println!("cargo:rerun-if-changed=Cargo.toml");
}
