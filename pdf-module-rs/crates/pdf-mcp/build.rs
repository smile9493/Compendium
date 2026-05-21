use std::fs;
use std::path::Path;

fn main() {
    // Ensure the SPA dist directory exists so rust-embed doesn't fail at compile time.
    // The real SPA build happens before CI test/release steps or via npm run build.
    let dist = Path::new("pdf-web-ui/dist");
    if !dist.exists() {
        fs::create_dir_all(dist).expect("create pdf-web-ui/dist for rust-embed");
        // Touch a .gitkeep so the directory is non-empty (rust-embed requires at least
        // one file or the folder itself to exist; an empty dir is sufficient for it not
        // to error, but we add a marker for clarity).
        fs::write(dist.join(".gitkeep"), b"").ok();
    }
}
