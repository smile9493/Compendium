use pdf_core::knowledge::entry::KnowledgeEntry;

fn main() {
    // Test 1: with quotes (what the test uses)
    let md_quoted = r#"---
title: "Page Range Entry"
domain: "IT"
page: "70-198"
tags: ["nginx"]
level: L1
status: compiled
quality_score: 0.86
created: 2026-05-08
updated: 2026-05-08
---

# Test"#;
    match KnowledgeEntry::from_markdown(md_quoted) {
        Some(e) => eprintln!("QUOTED page: OK -> page={:?}", e.page),
        None => eprintln!("QUOTED page: FAIL"),
    }

    // Test 2: without quotes (what the actual file uses)
    let md_unquoted = r#"---
title: "Page Range Entry"
domain: "IT"
page: 70-198
tags: ["nginx"]
level: L1
status: compiled
quality_score: 0.86
created: 2026-05-08
updated: 2026-05-08
---

# Test"#;
    match KnowledgeEntry::from_markdown(md_unquoted) {
        Some(e) => eprintln!("UNQUOTED page: OK -> page={:?}", e.page),
        None => eprintln!("UNQUOTED page: FAIL"),
    }

    // Test 3: page as number
    let md_num = r#"---
title: "Test"
domain: "IT"
page: 12
tags: ["nginx"]
level: L1
status: compiled
quality_score: 0.86
created: 2026-05-08
updated: 2026-05-08
---

# Test"#;
    match KnowledgeEntry::from_markdown(md_num) {
        Some(e) => eprintln!("NUMERIC page: OK -> page={:?}", e.page),
        None => eprintln!("NUMERIC page: FAIL"),
    }
}