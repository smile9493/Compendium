use pdf_core::knowledge::entry::KnowledgeEntry;

fn main() {
    // Test: absolute minimal
    let md1 = r#"---
title: "Test"
domain: "IT"
created: 2026-01-01T00:00:00Z
updated: 2026-01-01T00:00:00Z
---

# Test"#;
    match KnowledgeEntry::from_markdown(md1) {
        Some(e) => eprintln!("FULL_DATETIME: OK -> title={}", e.title),
        None => eprintln!("FULL_DATETIME: FAIL"),
    }

    // Test: date only
    let md2 = r#"---
title: "Test"
domain: "IT"
created: 2026-05-08
updated: 2026-05-08
---

# Test"#;
    match KnowledgeEntry::from_markdown(md2) {
        Some(e) => eprintln!("DATE_ONLY: OK -> title={}", e.title),
        None => eprintln!("DATE_ONLY: FAIL"),
    }

    // Test: no dates at all (default)
    let md3 = r#"---
title: "Test"
domain: "IT"
---

# Test"#;
    match KnowledgeEntry::from_markdown(md3) {
        Some(e) => eprintln!("NO_DATES: OK -> title={}", e.title),
        None => eprintln!("NO_DATES: FAIL"),
    }
}
