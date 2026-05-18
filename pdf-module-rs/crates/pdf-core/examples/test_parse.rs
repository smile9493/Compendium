use pdf_core::knowledge::entry::KnowledgeEntry;
use std::fs;

fn main() {
    let wiki_dir = "/opt/pdf-module/nginx-wiki/wiki";
    let mut count = 0;
    let mut failed = 0;
    for entry in fs::read_dir(format!("{}/it", wiki_dir)).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            let name = path.file_name().unwrap().to_string_lossy();
            if name == "index.md" || name == "log.md" {
                continue;
            }
            count += 1;
            let content = fs::read_to_string(&path).unwrap();
            match KnowledgeEntry::from_markdown(&content) {
                Some(e) => eprintln!(
                    "OK: {} -> title={} domain={} page={:?} tags={:?}",
                    name, e.title, e.domain, e.page, e.tags
                ),
                None => {
                    eprintln!("FAIL: {}", name);
                    failed += 1;
                }
            }
        }
    }
    eprintln!("\nTotal: {} entries, {} parsed OK, {} failed", count, count - failed, failed);
}
