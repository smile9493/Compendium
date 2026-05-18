//! Agent-friendly wiki entry patching (preview + apply).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::entry::{CompileStatus, EntryLevel, KnowledgeEntry};

fn status_yaml(s: &CompileStatus) -> &'static str {
    match s {
        CompileStatus::Pending => "pending",
        CompileStatus::Compiling => "compiling",
        CompileStatus::Compiled => "compiled",
        CompileStatus::NeedsRecompile => "needs_recompile",
        CompileStatus::Failed => "failed",
    }
}

fn level_yaml(l: &EntryLevel) -> String {
    format!("{l}")
}

/// A single patch operation against a wiki Markdown file.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PatchOp {
    ReplaceSection {
        heading: String,
        new_content: String,
    },
    ReplaceFrontMatter {
        #[serde(default)]
        tags: Option<Vec<String>>,
        #[serde(default)]
        related: Option<Vec<String>>,
        #[serde(default)]
        contradictions: Option<Vec<String>>,
        #[serde(default)]
        quality_score: Option<f32>,
    },
    SearchReplace {
        old: String,
        new: String,
        #[serde(default = "default_occurrence")]
        occurrence: String,
    },
}

fn default_occurrence() -> String {
    "first".to_string()
}

/// Patch request payload.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WikiPatchRequest {
    pub entry_path: String,
    pub operations: Vec<PatchOp>,
}

/// Result of applying or previewing a patch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPatchResult {
    pub entry_path: String,
    pub diff: String,
    pub applied: bool,
    pub new_size_bytes: usize,
}

/// Validate and resolve a wiki-relative path.
pub fn resolve_wiki_path(knowledge_base: &Path, entry_path: &str) -> PdfResult<PathBuf> {
    if entry_path.trim().is_empty() {
        return Err(PdfModuleError::Storage("entry_path must not be empty".into()));
    }
    if entry_path.contains("..") || entry_path.starts_with('/') {
        return Err(PdfModuleError::Storage(format!(
            "entry_path must be relative within wiki/: {entry_path}"
        )));
    }
    if !entry_path.ends_with(".md") {
        return Err(PdfModuleError::Storage(format!("entry_path must end with .md: {entry_path}")));
    }

    let wiki_dir = knowledge_base.join("wiki");
    let target = wiki_dir.join(entry_path);
    let resolved = target.canonicalize().unwrap_or_else(|_| target.clone());
    let wiki_canonical = wiki_dir.canonicalize().unwrap_or(wiki_dir);
    if !resolved.starts_with(&wiki_canonical) {
        return Err(PdfModuleError::Storage(format!(
            "Path traversal detected: {}",
            resolved.display()
        )));
    }
    Ok(target)
}

/// Preview patch without writing to disk.
pub fn preview_patch(
    knowledge_base: &Path,
    request: &WikiPatchRequest,
) -> PdfResult<WikiPatchResult> {
    let target = resolve_wiki_path(knowledge_base, &request.entry_path)?;
    let original = if target.exists() {
        fs::read_to_string(&target).map_err(|e| PdfModuleError::Storage(e.to_string()))?
    } else {
        return Err(PdfModuleError::FileNotFound(target.to_string_lossy().to_string()));
    };

    let patched = apply_operations(&original, &request.operations)?;
    Ok(WikiPatchResult {
        entry_path: request.entry_path.clone(),
        diff: unified_diff(&original, &patched),
        applied: false,
        new_size_bytes: patched.len(),
    })
}

/// Apply patch atomically (write `.md.tmp` then rename).
pub fn apply_patch(
    knowledge_base: &Path,
    request: &WikiPatchRequest,
) -> PdfResult<WikiPatchResult> {
    let target = resolve_wiki_path(knowledge_base, &request.entry_path)?;
    let original = fs::read_to_string(&target).map_err(|e| {
        PdfModuleError::Storage(format!("Failed to read {}: {e}", request.entry_path))
    })?;

    let patched = apply_operations(&original, &request.operations)?;
    let diff = unified_diff(&original, &patched);

    let tmp = target.with_extension("md.tmp");
    fs::write(&tmp, &patched).map_err(|e| PdfModuleError::Storage(e.to_string()))?;
    fs::rename(&tmp, &target).map_err(|e| PdfModuleError::Storage(e.to_string()))?;

    Ok(WikiPatchResult {
        entry_path: request.entry_path.clone(),
        diff,
        applied: true,
        new_size_bytes: patched.len(),
    })
}

fn apply_operations(content: &str, ops: &[PatchOp]) -> PdfResult<String> {
    let mut current = content.to_string();
    for op in ops {
        current = apply_one(&current, op)?;
    }
    Ok(current)
}

fn apply_one(content: &str, op: &PatchOp) -> PdfResult<String> {
    match op {
        PatchOp::ReplaceSection { heading, new_content } => {
            replace_section(content, heading, new_content)
        }
        PatchOp::ReplaceFrontMatter { tags, related, contradictions, quality_score } => {
            merge_front_matter(content, tags, related, contradictions, *quality_score)
        }
        PatchOp::SearchReplace { old, new, occurrence } => {
            search_replace(content, old, new, occurrence)
        }
    }
}

fn replace_section(content: &str, heading: &str, new_content: &str) -> PdfResult<String> {
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    let (front, body) = if parts.len() >= 3 {
        (format!("---{}---\n", parts[1]), parts[2])
    } else {
        (String::new(), content)
    };

    let heading_trim = heading.trim();
    let normalized = heading_trim.trim_start_matches('#').trim();
    let lines: Vec<&str> = body.lines().collect();
    let mut start_idx = None;
    let mut end_idx = lines.len();
    let mut heading_line = heading_trim.to_string();

    for (i, line) in lines.iter().enumerate() {
        let line_heading = line.trim_start_matches('#').trim();
        if line.trim() == heading_trim || line_heading == normalized {
            start_idx = Some(i);
            heading_line = (*line).to_string();
            let level = line.chars().take_while(|c| *c == '#').count().max(1);
            for (j, next) in lines.iter().enumerate().skip(i + 1) {
                let next_level = next.chars().take_while(|c| *c == '#').count();
                if next_level > 0 && next_level <= level {
                    end_idx = j;
                    break;
                }
            }
            break;
        }
    }

    let start_idx = start_idx
        .ok_or_else(|| PdfModuleError::Storage(format!("Heading not found: {heading}")))?;

    let mut new_body = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i < start_idx || i >= end_idx {
            new_body.push_str(line);
            new_body.push('\n');
        }
    }
    new_body.push_str(&heading_line);
    new_body.push('\n');
    new_body.push_str(new_content.trim());
    if !new_body.ends_with('\n') {
        new_body.push('\n');
    }

    Ok(format!("{front}{new_body}"))
}

fn merge_front_matter(
    content: &str,
    tags: &Option<Vec<String>>,
    related: &Option<Vec<String>>,
    contradictions: &Option<Vec<String>>,
    quality_score: Option<f32>,
) -> PdfResult<String> {
    let entry = KnowledgeEntry::from_markdown(content)
        .ok_or_else(|| PdfModuleError::Storage("Missing or invalid YAML front matter".into()))?;

    let mut lines: Vec<String> = Vec::new();
    lines.push("---".to_string());
    lines.push(format!("title: \"{}\"", entry.title.replace('"', "\\\"")));
    lines.push(format!("domain: \"{}\"", entry.domain));
    if let Some(t) = tags {
        let joined: String = t.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(", ");
        lines.push(format!("tags: [{joined}]"));
    } else {
        let joined: String =
            entry.tags.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(", ");
        lines.push(format!("tags: [{joined}]"));
    }
    lines.push(format!("level: {}", level_yaml(&entry.level)));
    lines.push(format!("status: {}", status_yaml(&entry.status)));
    let qs = quality_score.unwrap_or(entry.quality_score);
    lines.push(format!("quality_score: {qs}"));
    lines.push(format!("version: {}", entry.version));
    if let Some(r) = related {
        let joined: String = r.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(", ");
        lines.push(format!("related: [{joined}]"));
    } else {
        let joined: String =
            entry.related.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(", ");
        lines.push(format!("related: [{joined}]"));
    }
    if let Some(c) = contradictions {
        let joined: String = c.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(", ");
        lines.push(format!("contradictions: [{joined}]"));
    } else {
        let joined: String =
            entry.contradictions.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(", ");
        lines.push(format!("contradictions: [{joined}]"));
    }
    lines.push("---".to_string());

    let body = content.split("---").nth(2).unwrap_or("");
    Ok(format!("{}\n{}", lines.join("\n"), body.trim_start_matches('\n')))
}

fn search_replace(content: &str, old: &str, new: &str, occurrence: &str) -> PdfResult<String> {
    if old.is_empty() {
        return Err(PdfModuleError::Storage("search_replace.old must not be empty".into()));
    }
    match occurrence.to_lowercase().as_str() {
        "all" => Ok(content.replace(old, new)),
        _ => {
            let pos = content.find(old).ok_or_else(|| {
                PdfModuleError::Storage("search_replace: old string not found".into())
            })?;
            let mut out = String::with_capacity(content.len() - old.len() + new.len());
            out.push_str(&content[..pos]);
            out.push_str(new);
            out.push_str(&content[pos + old.len()..]);
            Ok(out)
        }
    }
}

fn unified_diff(old: &str, new: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let mut out = String::from("--- original\n+++ patched\n");
    let max = old_lines.len().max(new_lines.len());
    for i in 0..max {
        let o = old_lines.get(i).copied().unwrap_or("");
        let n = new_lines.get(i).copied().unwrap_or("");
        if o != n {
            if !o.is_empty() {
                out.push_str(&format!("-{o}\n"));
            }
            if !n.is_empty() {
                out.push_str(&format!("+{n}\n"));
            }
        }
    }
    out
}
