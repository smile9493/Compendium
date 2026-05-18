//! Structured quality issues for MCP workflows.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::PdfResult;
use crate::knowledge::quality::{
    analyze_wiki, build_next_actions, IssueSeverity, QualityIssue, QualityReport,
};

/// Stable issue identifier for `fix_suggest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListedQualityIssue {
    pub issue_id: String,
    pub severity: String,
    pub kind: String,
    pub entry_path: String,
    pub message: String,
}

fn issue_id_for(path: &str, kind: &str, message: &str) -> String {
    let mut h = DefaultHasher::new();
    path.hash(&mut h);
    kind.hash(&mut h);
    message.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn classify_issue(issue: &QualityIssue) -> String {
    let msg = issue.message.to_lowercase();
    if msg.contains("contradiction") {
        "contradiction".to_string()
    } else if msg.contains("broken") || msg.contains("link") {
        "broken_link".to_string()
    } else if msg.contains("orphan") {
        "orphan".to_string()
    } else if msg.contains("quality score") {
        "low_quality_score".to_string()
    } else if msg.contains("tag") {
        "missing_tags".to_string()
    } else {
        "general".to_string()
    }
}

/// List quality issues with stable ids.
pub fn list_quality_issues(
    wiki_dir: &Path,
    severity_filter: Option<&str>,
    limit: usize,
) -> PdfResult<Vec<ListedQualityIssue>> {
    let report = analyze_wiki(wiki_dir)?;
    Ok(collect_issues(&report, severity_filter, limit))
}

fn collect_issues(
    report: &QualityReport,
    severity_filter: Option<&str>,
    limit: usize,
) -> Vec<ListedQualityIssue> {
    let mut out = Vec::new();

    for issue in &report.issues {
        let sev = issue.severity.to_string();
        if let Some(f) = severity_filter {
            if !sev.eq_ignore_ascii_case(f) {
                continue;
            }
        }
        let kind = classify_issue(issue);
        out.push(ListedQualityIssue {
            issue_id: issue_id_for(&issue.entry_path, &kind, &issue.message),
            severity: sev,
            kind,
            entry_path: issue.entry_path.clone(),
            message: issue.message.clone(),
        });
    }

    for path in &report.orphan_entries {
        let msg = "Orphan entry (no links in or out)".to_string();
        let kind = "orphan".to_string();
        out.push(ListedQualityIssue {
            issue_id: issue_id_for(path, &kind, &msg),
            severity: "WARN".to_string(),
            kind,
            entry_path: path.clone(),
            message: msg,
        });
    }

    for path in &report.broken_links {
        let msg = format!("Broken link reference: {path}");
        let kind = "broken_link".to_string();
        out.push(ListedQualityIssue {
            issue_id: issue_id_for(path, &kind, &msg),
            severity: "ERROR".to_string(),
            kind,
            entry_path: path.clone(),
            message: msg,
        });
    }

    out.truncate(limit);
    out
}

/// Suggest fixes for a specific issue id.
pub fn fix_suggest(
    wiki_dir: &Path,
    knowledge_base: &str,
    issue_id: &str,
) -> PdfResult<serde_json::Value> {
    let report = analyze_wiki(wiki_dir)?;
    let all = collect_issues(&report, None, usize::MAX);
    let issue = all.into_iter().find(|i| i.issue_id == issue_id).ok_or_else(|| {
        crate::error::PdfModuleError::FileNotFound(format!("Quality issue not found: {issue_id}"))
    })?;

    let mut suggestions = Vec::new();
    match issue.kind.as_str() {
        "orphan" => {
            suggestions.push(serde_json::json!({
                "tool": "suggest_links",
                "args": { "knowledge_base": knowledge_base, "entry_path": issue.entry_path, "top_k": 5 }
            }));
        }
        "broken_link" => {
            suggestions.push(serde_json::json!({
                "tool": "patch_wiki_entry",
                "args": {
                    "knowledge_base": knowledge_base,
                    "entry_path": issue.entry_path,
                    "operations": [{ "type": "replace_front_matter", "related": [] }]
                }
            }));
        }
        "contradiction" => {
            suggestions.push(serde_json::json!({
                "tool": "hypothesis_test",
                "args": { "knowledge_base": knowledge_base }
            }));
        }
        _ => {}
    }

    if suggestions.is_empty() {
        suggestions = build_next_actions(&report, knowledge_base);
    }

    Ok(serde_json::json!({
        "issue": issue,
        "suggestions": suggestions,
    }))
}

pub fn issues_from_contradictions(pairs: &[(String, String)]) -> Vec<ListedQualityIssue> {
    pairs
        .iter()
        .map(|(a, b)| {
            let msg = format!("Contradiction between {a} and {b}");
            ListedQualityIssue {
                issue_id: issue_id_for(a, "contradiction", &msg),
                severity: IssueSeverity::Error.to_string(),
                kind: "contradiction".to_string(),
                entry_path: a.clone(),
                message: msg,
            }
        })
        .collect()
}
