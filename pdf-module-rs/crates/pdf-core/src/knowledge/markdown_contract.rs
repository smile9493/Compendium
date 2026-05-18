//! Structural markdown metrics shared with frontend contract tests.

/// Counts of structural elements in a markdown body (no HTML rendering).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkdownStructure {
    pub heading_count: usize,
    pub wikilink_count: usize,
    pub fenced_code_blocks: usize,
}

/// Analyze markdown body text (without YAML front matter).
pub fn analyze_markdown_body(body: &str) -> MarkdownStructure {
    let mut heading_count = 0usize;
    let mut wikilink_count = 0usize;
    let mut fenced_code_blocks = 0usize;
    let mut in_fence = false;

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_fence {
                in_fence = false;
            } else {
                in_fence = true;
                fenced_code_blocks += 1;
            }
            continue;
        }
        if in_fence {
            continue;
        }
        if trimmed.starts_with('#') {
            let hashes = trimmed.chars().take_while(|c| *c == '#').count();
            if hashes > 0 && trimmed.chars().nth(hashes).is_some_and(|c| c.is_whitespace()) {
                heading_count += 1;
            }
        }
        wikilink_count += count_wikilinks(trimmed);
    }

    MarkdownStructure { heading_count, wikilink_count, fenced_code_blocks }
}

fn count_wikilinks(line: &str) -> usize {
    let mut count = 0usize;
    let mut rest = line;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find("]]") {
            count += 1;
            rest = &after[end + 2..];
        } else {
            break;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_BODY: &str = "\
# Overview\n\n\
See [[IT/related]] and [[other/path]] for more.\n\n\
## Details\n\n\
```rust\n\
fn main() {}\n\
```\n\n\
```python\n\
print(\"hi\")\n\
```\n\n\
# Appendix\n\n\
Another [[wikilink]] here.\n";

    #[test]
    fn test_fixture_structure_counts() {
        let s = analyze_markdown_body(FIXTURE_BODY);
        assert_eq!(s.heading_count, 3, "expected 3 headings");
        assert_eq!(s.wikilink_count, 3, "expected 3 wikilinks");
        assert_eq!(s.fenced_code_blocks, 2, "expected 2 code fences");
    }

    #[test]
    fn test_repo_fixture_file() {
        let raw = include_str!("../../tests/fixtures/wiki_sample.md");
        let body = raw
            .trim_start()
            .strip_prefix("---")
            .and_then(|s| s.split_once("\n---\n").map(|(_, b)| b))
            .unwrap_or(raw);
        let s = analyze_markdown_body(body);
        assert_eq!(s.heading_count, 3);
        assert_eq!(s.wikilink_count, 3);
        assert_eq!(s.fenced_code_blocks, 2);
    }
}
