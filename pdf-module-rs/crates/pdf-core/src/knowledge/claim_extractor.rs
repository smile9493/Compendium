//! Claim-evidence extraction from knowledge entry Markdown bodies.
//!
//! Identifies factual statements and their supporting references
//! to enable provenance tracking and cross-entry consistency checks.

use crate::knowledge::entry::Claim;

/// Extract claim-evidence pairs from a Markdown body.
///
/// Uses heuristic patterns: numbered claims, bold statements, blockquotes as evidence.
/// Returns a list of extracted claims (may be empty for unstructured bodies).
pub fn extract_claims(body: &str) -> Vec<Claim> {
    let mut claims = Vec::new();
    let lines: Vec<&str> = body.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Pattern 1: Numbered claims — "1. Statement" or "1) Statement".
        if let Some(statement) = extract_numbered_claim(trimmed) {
            // Look ahead for a blockquote as evidence.
            let evidence = find_blockquote_evidence(&lines, i + 1);
            claims.push(Claim {
                statement: statement.to_string(),
                evidence,
                confidence: Default::default(),
            });
            continue;
        }

        // Pattern 2: Bold statement with colon — "**Claim**: evidence text".
        if let Some((claim_text, evidence_text)) = extract_bold_colon_claim(trimmed) {
            claims.push(Claim {
                statement: claim_text.to_string(),
                evidence: if evidence_text.is_empty() {
                    None
                } else {
                    Some(evidence_text.to_string())
                },
                confidence: Default::default(),
            });
        }
    }

    claims
}

/// Extract statement from "1. Statement" or "1) Statement" patterns.
fn extract_numbered_claim(line: &str) -> Option<&str> {
    let bytes = line.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_digit() {
        return None;
    }
    let mut end_num = 0;
    while end_num < bytes.len() && bytes[end_num].is_ascii_digit() {
        end_num += 1;
    }
    if end_num >= bytes.len() {
        return None;
    }
    let rest = &line[end_num..];
    if rest.starts_with(". ") || rest.starts_with(") ") {
        let statement = rest[2..].trim();
        if !statement.is_empty() {
            return Some(statement);
        }
    }
    None
}

/// Extract claim and evidence from "**Bold**: rest" pattern.
fn extract_bold_colon_claim(line: &str) -> Option<(&str, &str)> {
    if !line.starts_with("**") {
        return None;
    }
    let after_open = &line[2..];
    let close = after_open.find("**")?;
    let bold_text = &after_open[..close];
    let rest = &after_open[close + 2..];
    if let Some(colon_pos) = rest.find(':') {
        let evidence = rest[colon_pos + 1..].trim();
        Some((bold_text.trim(), evidence))
    } else {
        Some((bold_text.trim(), ""))
    }
}

/// Scan forward from `start` for a blockquote line "> evidence" and return its content.
fn find_blockquote_evidence(lines: &[&str], start: usize) -> Option<String> {
    for line in lines.iter().skip(start).take(3) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("> ") {
            return Some(rest.trim().to_string());
        }
        // If we hit a non-empty, non-blockquote line, stop looking.
        return None;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_numbered_claims() {
        let body = "1. HTTP/2 uses multiplexing.\n2. Streams share one connection.\n";
        let claims = extract_claims(body);
        assert_eq!(claims.len(), 2);
        assert_eq!(claims[0].statement, "HTTP/2 uses multiplexing.");
        assert_eq!(claims[1].statement, "Streams share one connection.");
    }

    #[test]
    fn test_bold_colon_claim() {
        let body = "**Multiplexing**: Allows concurrent streams over a single TCP connection.";
        let claims = extract_claims(body);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].statement, "Multiplexing");
        assert_eq!(
            claims[0].evidence.as_deref(),
            Some("Allows concurrent streams over a single TCP connection.")
        );
    }

    #[test]
    fn test_blockquote_evidence() {
        let body = "1. RFC 7540 defines HTTP/2.\n> See RFC 7540, Section 1.";
        let claims = extract_claims(body);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].evidence.as_deref(), Some("See RFC 7540, Section 1."));
    }

    #[test]
    fn test_no_claims_in_prose() {
        let body = "This is just regular prose with no numbered items or bold claims.";
        let claims = extract_claims(body);
        assert!(claims.is_empty());
    }
}
