use pdf_core::knowledge::entry::KnowledgeEntry;

fn main() {
    // Test uppercase L1 (as in actual files)
    let md = r#"---
title: "Test"
domain: "IT"
tags: ["nginx"]
level: L1
status: compiled
quality_score: 0.86
created: 2026-05-08
updated: 2026-05-08
---

# Test"#;
    match KnowledgeEntry::from_markdown(md) {
        Some(e) => eprintln!("level=L1: OK -> level={}", e.level),
        None => eprintln!("level=L1: FAIL"),
    }

    // Test lowercase l1 (as in our tests)
    let md2 = r#"---
title: "Test"
domain: "IT"
tags: ["nginx"]
level: l1
status: compiled
quality_score: 0.86
created: 2026-05-08
updated: 2026-05-08
---

# Test"#;
    match KnowledgeEntry::from_markdown(md2) {
        Some(e) => eprintln!("level=l1: OK -> level={}", e.level),
        None => eprintln!("level=l1: FAIL"),
    }
}