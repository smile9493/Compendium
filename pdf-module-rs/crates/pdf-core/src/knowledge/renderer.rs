//! Wiki entry rendering for HTTP/MCP APIs and tree views.
//!
//! Strips YAML front matter and returns `body_markdown` for client-side rendering
//! (Vue SPA uses `marked`). Does not generate server-side HTML.

use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::entry::KnowledgeEntry;

/// Rendered Markdown entry with parsed front matter for the Vue SPA.
#[derive(Debug, Clone, Serialize)]
pub struct RenderedEntry {
    pub title: String,
    pub domain: String,
    pub tags: Vec<String>,
    pub level: String,
    pub quality_score: f32,
    pub status: String,
    pub version: u32,
    /// Markdown body without YAML front matter (rendered client-side via `marked`).
    pub body_markdown: String,
    /// Deprecated: server-side HTML is no longer generated; use `body_markdown`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_html: Option<String>,
    pub related: Vec<String>,
    pub contradictions: Vec<String>,
    pub backlinks: Vec<String>,
    pub source: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TreeNode {
    pub name: String,
    pub path: String,
    pub children: Vec<TreeNode>,
    pub is_entry: bool,
    pub title: Option<String>,
    pub domain: Option<String>,
}

struct LightMeta {
    title: String,
    domain: String,
    tags: Vec<String>,
    level: String,
    quality_score: f32,
    status: String,
    source: Option<String>,
    related: Vec<String>,
    contradictions: Vec<String>,
}

fn parse_light_meta(content: &str) -> Option<LightMeta> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after_first = &trimmed[3..];
    let end = after_first.find("---")?;
    let yaml = &after_first[..end];

    let mut title = String::new();
    let mut domain = String::new();
    let mut tags: Vec<String> = Vec::new();
    let mut level = String::new();
    let mut quality_score = 0.0f32;
    let mut status = String::new();
    let mut source: Option<String> = None;
    let mut related: Vec<String> = Vec::new();
    let mut contradictions: Vec<String> = Vec::new();

    for line in yaml.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, val)) = line.split_once(':') {
            let key = key.trim();
            let val = val.trim();
            match key {
                "title" => title = val.trim_matches('"').trim_matches('\'').to_string(),
                "domain" => domain = val.trim_matches('"').trim_matches('\'').to_string(),
                "level" => level = val.trim_matches('"').trim_matches('\'').to_string(),
                "status" => status = val.trim_matches('"').trim_matches('\'').to_string(),
                "quality_score" => {
                    quality_score = val.parse().unwrap_or(0.0);
                }
                "source" if !val.is_empty() => {
                    source = Some(val.trim_matches('"').trim_matches('\'').to_string());
                }
                "tags" if val.starts_with('[') && val.ends_with(']') => {
                    tags = val[1..val.len() - 1]
                        .split(',')
                        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                "related" if val.starts_with('[') && val.ends_with(']') => {
                    related = val[1..val.len() - 1]
                        .split(',')
                        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                "contradictions" if val.starts_with('[') && val.ends_with(']') => {
                    contradictions = val[1..val.len() - 1]
                        .split(',')
                        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                _ => {}
            }
        }
    }

    if title.is_empty() {
        return None;
    }

    Some(LightMeta {
        title,
        domain,
        tags,
        level,
        quality_score,
        status,
        source,
        related,
        contradictions,
    })
}

fn extract_meta(content: &str) -> (Option<String>, Option<String>) {
    if let Some(m) = parse_light_meta(content) {
        return (Some(m.title), Some(m.domain));
    }
    if let Some(e) = KnowledgeEntry::from_markdown(content) {
        return (Some(e.title), Some(e.domain));
    }
    (None, None)
}

pub struct WikiRenderer {
    wiki_path: PathBuf,
}

impl WikiRenderer {
    pub fn new(wiki_path: &Path) -> Self {
        Self { wiki_path: wiki_path.to_path_buf() }
    }

    pub fn render_entry(&self, relative_path: &str) -> PdfResult<RenderedEntry> {
        let full_path = self.wiki_path.join(relative_path);
        if !full_path.exists() {
            return Err(PdfModuleError::Storage(format!("Entry not found: {}", relative_path)));
        }

        let content = std::fs::read_to_string(&full_path).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to read entry {}: {}", relative_path, e))
        })?;

        let body_md = split_front_matter(&content);
        let body_markdown = body_md.to_string();

        let light = parse_light_meta(&content);
        let entry = KnowledgeEntry::from_markdown(&content);

        let backlinks = self.find_backlinks(relative_path);

        let title = entry
            .as_ref()
            .map(|e| e.title.clone())
            .or_else(|| light.as_ref().map(|m| m.title.clone()))
            .unwrap_or_else(|| extract_title_from_path(relative_path));

        let domain = entry
            .as_ref()
            .map(|e| e.domain.clone())
            .or_else(|| light.as_ref().map(|m| m.domain.clone()))
            .unwrap_or_default();

        let tags = entry
            .as_ref()
            .map(|e| e.tags.clone())
            .or_else(|| light.as_ref().map(|m| m.tags.clone()))
            .unwrap_or_default();

        let level = entry
            .as_ref()
            .map(|e| e.level.to_string())
            .or_else(|| light.as_ref().map(|m| m.level.clone()))
            .unwrap_or_default();

        let quality_score = entry
            .as_ref()
            .map(|e| e.quality_score)
            .or_else(|| light.as_ref().map(|m| m.quality_score))
            .unwrap_or(0.0);

        let status = entry
            .as_ref()
            .map(|e| format!("{:?}", e.status).to_lowercase())
            .or_else(|| light.as_ref().map(|m| m.status.clone()))
            .unwrap_or_default();

        let related = entry
            .as_ref()
            .map(|e| e.related.clone())
            .or_else(|| light.as_ref().map(|m| m.related.clone()))
            .unwrap_or_default();

        let contradictions = entry
            .as_ref()
            .map(|e| e.contradictions.clone())
            .or_else(|| light.as_ref().map(|m| m.contradictions.clone()))
            .unwrap_or_default();

        let source = entry
            .as_ref()
            .and_then(|e| e.source.clone())
            .or_else(|| light.as_ref().and_then(|m| m.source.clone()));

        let created = entry.as_ref().map(|e| e.created.to_rfc3339());
        let updated = entry.as_ref().map(|e| e.updated.to_rfc3339());

        Ok(RenderedEntry {
            title,
            domain,
            tags,
            level,
            quality_score,
            status,
            version: entry.as_ref().map(|e| e.version).unwrap_or(0),
            body_markdown,
            body_html: None,
            related,
            contradictions,
            backlinks,
            source,
            created,
            updated,
        })
    }

    pub fn render_tree(&self) -> PdfResult<TreeNode> {
        if !self.wiki_path.exists() {
            return Ok(TreeNode {
                name: "wiki".to_string(),
                path: String::new(),
                children: vec![],
                is_entry: false,
                title: None,
                domain: None,
            });
        }
        build_tree(&self.wiki_path, &self.wiki_path)
    }

    fn find_backlinks(&self, target_path: &str) -> Vec<String> {
        let mut backlinks = Vec::new();
        let normalized_target = normalize_path(target_path);

        if let Ok(entries) = walk_md_files(&self.wiki_path) {
            for entry_path in entries {
                let relative = match entry_path.strip_prefix(&self.wiki_path) {
                    Ok(r) => r.to_string_lossy().to_string(),
                    Err(_) => continue,
                };

                if normalize_path(&relative) == normalized_target {
                    continue;
                }

                if let Ok(content) = std::fs::read_to_string(&entry_path) {
                    let related: Vec<String>;
                    let contradictions: Vec<String>;

                    if let Some(front) = KnowledgeEntry::from_markdown(&content) {
                        related = front.related;
                        contradictions = front.contradictions;
                    } else if let Some(m) = parse_light_meta(&content) {
                        related = m.related;
                        contradictions = m.contradictions;
                    } else {
                        continue;
                    }

                    let related_match =
                        related.iter().any(|r| normalize_path(r) == normalized_target);
                    let contradict_match =
                        contradictions.iter().any(|c| normalize_path(c) == normalized_target);

                    if related_match || contradict_match {
                        backlinks.push(relative);
                    }
                }
            }
        }

        backlinks.sort();
        backlinks.dedup();
        backlinks
    }
}

fn split_front_matter(content: &str) -> &str {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return trimmed;
    }
    let after_first = &trimmed[3..];
    match after_first.find("---") {
        Some(end) => after_first[end + 3..].trim_start(),
        None => trimmed,
    }
}

fn build_tree(base: &Path, current: &Path) -> PdfResult<TreeNode> {
    let name = current
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "wiki".to_string());

    let relative =
        current.strip_prefix(base).map(|r| r.to_string_lossy().to_string()).unwrap_or_default();

    if current.is_file() {
        let is_entry =
            current.extension().map(|e| e == "md").unwrap_or(false) && !name.starts_with('.');

        let (title, domain) = if is_entry {
            if let Ok(content) = std::fs::read_to_string(current) {
                extract_meta(&content)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        return Ok(TreeNode { name, path: relative, children: vec![], is_entry, title, domain });
    }

    let mut children = Vec::new();
    let mut flat_entries = Vec::new();
    let mut entries_by_domain: HashMap<String, Vec<TreeNode>> = HashMap::new();

    if let Ok(dir_entries) = std::fs::read_dir(current) {
        for entry in dir_entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            if file_name.starts_with('.') || file_name == "index.md" || file_name == "log.md" {
                continue;
            }

            if path.is_dir() {
                let child = build_tree(base, &path)?;
                children.push(child);
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                let child = build_tree(base, &path)?;
                flat_entries.push(child);
            }
        }
    }

    let has_subdirs = children.iter().any(|c| !c.is_entry);

    if has_subdirs {
        flat_entries.sort_by(|a, b| a.name.cmp(&b.name));
        children.extend(flat_entries);
    } else if !flat_entries.is_empty() {
        let domains: Vec<String> = flat_entries
            .iter()
            .filter_map(|e| e.domain.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let dir_name_lower = name.to_lowercase();
        let single_domain_matches_dir =
            domains.len() == 1 && domains[0].to_lowercase() == dir_name_lower;

        if single_domain_matches_dir || domains.len() <= 1 {
            flat_entries.sort_by(|a, b| a.name.cmp(&b.name));
            children.extend(flat_entries);
        } else {
            for entry in flat_entries {
                entries_by_domain
                    .entry(entry.domain.clone().unwrap_or_default())
                    .or_default()
                    .push(entry);
            }
            for (domain, mut entries) in entries_by_domain {
                entries.sort_by(|a, b| a.name.cmp(&b.name));
                children.push(TreeNode {
                    name: if domain.is_empty() { "未分类".to_string() } else { domain.clone() },
                    path: if domain.is_empty() {
                        String::new()
                    } else {
                        domain.to_lowercase().replace(' ', "_")
                    },
                    children: entries,
                    is_entry: false,
                    title: None,
                    domain: Some(domain),
                });
            }
        }
    }

    children.sort_by(|a, b| {
        let a_is_dir = !a.is_entry;
        let b_is_dir = !b.is_entry;
        b_is_dir.cmp(&a_is_dir).then_with(|| a.name.cmp(&b.name))
    });

    Ok(TreeNode { name, path: relative, children, is_entry: false, title: None, domain: None })
}

fn walk_md_files(dir: &Path) -> PdfResult<Vec<PathBuf>> {
    let mut result = Vec::new();
    if !dir.exists() {
        return Ok(result);
    }

    fn walk(dir: &Path, result: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, result);
                } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                    result.push(path);
                }
            }
        }
    }

    walk(dir, &mut result);
    Ok(result)
}

fn normalize_path(p: &str) -> String {
    let p = p.trim_start_matches("wiki/");
    let p = p.trim_start_matches('/');
    p.replace('\\', "/")
}

fn extract_title_from_path(path: &str) -> String {
    let file_name = Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or("Untitled");

    if file_name.starts_with('[') {
        if let Some(end) = file_name.find(']') {
            return file_name[end + 2..].to_string();
        }
    }

    file_name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn render_entry_includes_non_empty_body_markdown() {
        let tmp = TempDir::new().unwrap();
        let wiki = tmp.path().join("wiki");
        fs::create_dir_all(&wiki).unwrap();
        let path = wiki.join("test-entry.md");
        fs::write(
            &path,
            r#"---
title: Test Entry
domain: rust
level: L1
quality_score: 0.9
status: active
---
# Hello

Paragraph with **bold** text.
"#,
        )
        .unwrap();

        let renderer = WikiRenderer::new(&wiki);
        let entry = renderer.render_entry("test-entry.md").unwrap();
        assert!(!entry.body_markdown.is_empty());
        assert!(entry.body_markdown.contains("# Hello"));
        assert!(entry.body_html.is_none());

        let json = serde_json::to_value(&entry).unwrap();
        assert!(json.get("body_html").is_none());
        let md = json.get("body_markdown").and_then(|v| v.as_str());
        assert!(md.is_some_and(|s| !s.is_empty()));
    }
}
