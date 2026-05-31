//! Wiki knowledge base — extraction metadata, raw storage, and log management.
//!
//! Manages the wiki file system: saves extracted data as YAML metadata,
//! writes raw extraction outputs, maintains versioned hashes for incremental
//! compilation, and keeps an audit log.

use std::fs::{self, OpenOptions};
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::dto::StructuredExtractionResult;
use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::entry::KnowledgeEntry;

/// Audit log event kinds (Karpathy build log).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NervousEventKind {
    Extract,
    Save,
    CompileComplete,
    Lint,
    Archive,
    Patch,
    Propagation,
}

impl NervousEventKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Extract => "extract",
            Self::Save => "save",
            Self::CompileComplete => "compile_complete",
            Self::Lint => "lint",
            Self::Archive => "archive",
            Self::Patch => "patch",
            Self::Propagation => "propagation",
        }
    }
}

/// Payload for [`WikiStorage::sync_nervous_system`].
#[derive(Debug, Clone)]
pub struct NervousEvent {
    pub kind: NervousEventKind,
    pub detail: String,
}

impl NervousEvent {
    pub fn new(kind: NervousEventKind, detail: impl Into<String>) -> Self {
        Self { kind, detail: detail.into() }
    }
}

/// Sync index + log after wiki mutations (callable without `WikiStorage`).
pub fn sync_nervous_system(kb_path: impl AsRef<Path>, event: NervousEvent) -> PdfResult<()> {
    let storage = WikiStorage::new(kb_path)?;
    storage.sync_nervous_system(event)
}

/// Extraction metadata stored alongside each raw extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionMetadata {
    pub source_file: String,
    pub source_name: String,
    pub file_hash: String,
    pub extraction_time: DateTime<Utc>,
    pub page_count: u32,
    pub quality_score: f64,
}

/// Wiki file system storage — manages raw/, wiki/, schema/ and log.md.
pub struct WikiStorage {
    base_path: PathBuf,
}

impl WikiStorage {
    pub fn new(base_path: impl AsRef<Path>) -> PdfResult<Self> {
        let base_path = base_path.as_ref().to_path_buf();

        fs::create_dir_all(base_path.join("raw"))
            .map_err(|e| PdfModuleError::Storage(format!("Failed to create raw dir: {}", e)))?;
        fs::create_dir_all(base_path.join("wiki"))
            .map_err(|e| PdfModuleError::Storage(format!("Failed to create wiki dir: {}", e)))?;
        fs::create_dir_all(base_path.join("schema"))
            .map_err(|e| PdfModuleError::Storage(format!("Failed to create schema dir: {}", e)))?;

        Ok(Self { base_path })
    }

    pub fn save_raw(
        &self,
        extraction_result: &StructuredExtractionResult,
        source_file: &Path,
        quality_score: f64,
    ) -> PdfResult<WikiExtractionResult> {
        let file_hash = Self::compute_file_hash(&extraction_result.extracted_text);
        let source_name = Self::extract_source_name(source_file);

        let metadata = ExtractionMetadata {
            source_file: source_file.to_string_lossy().to_string(),
            source_name: source_name.clone(),
            file_hash: file_hash.clone(),
            extraction_time: Utc::now(),
            page_count: extraction_result.page_count,
            quality_score,
        };

        let raw_filename = format!("{}.md", source_name);
        let raw_path = self.base_path.join("raw").join(&raw_filename);

        let versioned =
            self.save_raw_file(&raw_path, &metadata, &extraction_result.extracted_text)?;

        self.sync_nervous_system(NervousEvent::new(
            NervousEventKind::Extract,
            format!(
                "raw={} pages={} versioned={}",
                raw_path.display(),
                metadata.page_count,
                versioned
            ),
        ))?;

        Ok(WikiExtractionResult {
            raw_path,
            index_path: self.base_path.join("wiki").join("index.md"),
            log_path: self.base_path.join("wiki").join("log.md"),
            page_count: extraction_result.page_count,
        })
    }

    /// Write raw content; if file exists with different hash, archive to `raw/.versions/`.
    fn save_raw_file(
        &self,
        path: &Path,
        metadata: &ExtractionMetadata,
        text: &str,
    ) -> PdfResult<bool> {
        let new_hash = Self::compute_file_hash(text);
        let mut versioned = false;

        if path.exists() {
            if let Ok(existing) = fs::read_to_string(path)
                && let Some(old_hash) = Self::parse_hash_from_raw(&existing)
                && old_hash == new_hash
            {
                return Ok(false);
            }
            versioned = Self::archive_raw_version(path)?;
        }

        let yaml = serde_yaml::to_string(metadata)
            .map_err(|e| PdfModuleError::Storage(format!("YAML error: {}", e)))?;

        let content = format!(
            "---\n{}---\n\n# {}\n\n## 文档信息\n\n- 页数: {}\n- 质量: {:.0}%\n- 提取时间: {}\n\n## 正文\n\n{}",
            yaml,
            metadata.source_name,
            metadata.page_count,
            metadata.quality_score * 100.0,
            metadata.extraction_time.format("%Y-%m-%d %H:%M:%S UTC"),
            Self::format_text(text)
        );

        fs::write(path, content.as_bytes())
            .map_err(|e| PdfModuleError::Storage(format!("Failed to write raw file: {}", e)))?;

        Ok(versioned)
    }

    fn archive_raw_version(path: &Path) -> PdfResult<bool> {
        let raw_dir =
            path.parent().ok_or_else(|| PdfModuleError::Storage("no raw parent".into()))?;
        let versions = raw_dir.join(".versions");
        fs::create_dir_all(&versions)
            .map_err(|e| PdfModuleError::Storage(format!("versions dir: {}", e)))?;
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("raw");
        let ts = Utc::now().format("%Y%m%dT%H%M%SZ");
        let dest = versions.join(format!("{}.{}.md", stem, ts));
        if path.exists() {
            fs::copy(path, &dest)
                .map_err(|e| PdfModuleError::Storage(format!("archive raw: {}", e)))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn parse_hash_from_raw(content: &str) -> Option<String> {
        let yaml = crate::knowledge::entry::extract_front_matter_yaml(content)?;
        for line in yaml.lines() {
            if line.trim_start().starts_with("file_hash:") {
                return line.split(':').nth(1).map(|s| s.trim().to_string());
            }
        }
        None
    }

    fn format_text(text: &str) -> String {
        text.lines()
            .filter(|l| !l.trim().is_empty())
            .collect::<Vec<_>>()
            .chunks(5)
            .map(|chunk| chunk.join("\n"))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Regenerate `wiki/index.md` from all entries under `wiki/` (recursive).
    pub fn generate_index(&self) -> PdfResult<PathBuf> {
        let index_path = self.base_path.join("wiki").join("index.md");
        let wiki_dir = self.base_path.join("wiki");
        let entities = Self::collect_entities(&wiki_dir)?;
        let content = Self::build_index(&entities);

        fs::write(&index_path, content.as_bytes())
            .map_err(|e| PdfModuleError::Storage(format!("Index write error: {}", e)))?;

        Ok(index_path)
    }

    fn collect_entities(wiki_dir: &Path) -> PdfResult<Vec<EntityInfo>> {
        let mut entities = Vec::new();
        Self::walk_wiki_md(wiki_dir, wiki_dir, &mut entities)?;
        entities.sort_by(|a, b| a.domain.cmp(&b.domain).then(a.title.cmp(&b.title)));
        Ok(entities)
    }

    fn walk_wiki_md(wiki_dir: &Path, dir: &Path, out: &mut Vec<EntityInfo>) -> PdfResult<()> {
        if !dir.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(dir)
            .map_err(|e| PdfModuleError::Storage(format!("Read wiki dir error: {}", e)))?
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                if name == ".versions" {
                    continue;
                }
                Self::walk_wiki_md(wiki_dir, &path, out)?;
                continue;
            }
            if path.extension().map(|e| e == "md").unwrap_or(false)
                && name != "index.md"
                && name != "log.md"
                && let Some(info) = Self::parse_entity(&path, wiki_dir)
            {
                out.push(info);
            }
        }
        Ok(())
    }

    fn parse_entity(path: &Path, wiki_dir: &Path) -> Option<EntityInfo> {
        let content = fs::read_to_string(path).ok()?;
        let rel_path = path.strip_prefix(wiki_dir).ok()?.to_string_lossy().replace('\\', "/");

        let mut domain = "未分类".to_string();
        let mut title = path.file_stem()?.to_string_lossy().to_string();

        if let Some(entry) = KnowledgeEntry::from_markdown(&content) {
            domain = entry.domain.clone();
            title = entry.title.clone();
        } else if let Some(h) = content.lines().find(|l| l.starts_with("# ")) {
            let t = h[2..].trim();
            if let Some(end) = t.find(']')
                && t.starts_with('[')
            {
                domain = t[1..end].to_string();
                title = t[end + 1..].trim().to_string();
            } else {
                title = t.to_string();
            }
        }

        let abstract_text = content
            .lines()
            .skip_while(|l| !l.contains("[!ABSTRACT]"))
            .nth(1)
            .map(|l| l.trim().to_string())
            .unwrap_or_default();

        Some(EntityInfo { rel_path, domain, title, abstract_text })
    }

    fn build_index(entities: &[EntityInfo]) -> String {
        let mut content = String::new();

        content.push_str("# 知识索引\n\n");
        content.push_str("> [!ABSTRACT] 摘要\n");
        content.push_str("> 本页面是 Wiki 知识库的总导航图，按领域分类组织。\n\n");

        if entities.is_empty() {
            content.push_str("*暂无词条。请使用 **ingest** 处理 `raw/` 中的原始素材。*\n\n");
        } else {
            let mut current_domain = String::new();

            for entity in entities {
                if entity.domain != current_domain {
                    current_domain = entity.domain.clone();
                    content.push_str(&format!("\n## [{}]\n\n", current_domain));
                    content.push_str("| 词条 | 摘要 |\n");
                    content.push_str("|------|------|\n");
                }

                content
                    .push_str(&format!("| [[{}]] | {} |\n", entity.rel_path, entity.abstract_text));
            }
        }

        content.push_str("\n---\n\n");
        content.push_str(&format!("- **词条总数**: {}\n", entities.len()));
        content.push_str("- [[log.md]] - 编译日志\n");

        content
    }

    /// Append one line to `wiki/log.md`.
    pub fn append_log(&self, event: &NervousEvent) -> PdfResult<()> {
        let log_path = self.base_path.join("wiki").join("log.md");
        if !log_path.exists() {
            fs::write(&log_path, "# Build Log\n\n")
                .map_err(|e| PdfModuleError::Storage(format!("create log: {}", e)))?;
        }
        let line = format!(
            "- {} **{}** — {}\n",
            Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
            event.kind.as_str(),
            event.detail.replace('\n', " ")
        );
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| PdfModuleError::Storage(format!("open log: {}", e)))?;
        file.write_all(line.as_bytes())
            .map_err(|e| PdfModuleError::Storage(format!("append log: {}", e)))?;
        Ok(())
    }

    /// Read the tail of the build log (last `max_lines` non-empty lines).
    pub fn read_log_tail(&self, max_lines: usize) -> PdfResult<String> {
        let log_path = self.base_path.join("wiki").join("log.md");
        if !log_path.exists() {
            return Ok(String::new());
        }
        let content = fs::read_to_string(&log_path)
            .map_err(|e| PdfModuleError::Storage(format!("read log: {}", e)))?;
        let lines: Vec<_> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        let start = lines.len().saturating_sub(max_lines);
        Ok(lines[start..].join("\n"))
    }

    /// Regenerate index and append audit line.
    pub fn sync_nervous_system(&self, event: NervousEvent) -> PdfResult<()> {
        self.generate_index()?;
        self.append_log(&event)?;
        Ok(())
    }

    fn extract_source_name(path: &Path) -> String {
        path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string()
    }

    pub fn compute_file_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Return the path to the wiki directory.
    pub fn wiki_dir(&self) -> PathBuf {
        self.base_path.join("wiki")
    }

    /// Save a media attachment to the wiki attachments directory.
    ///
    /// Creates `wiki/<domain>/<entry_name>_attachments/` if needed,
    /// then writes `filename` there and returns the full path.
    pub fn save_attachment(
        &self,
        domain: &str,
        entry_name: &str,
        filename: &str,
        data: &[u8],
    ) -> PdfResult<PathBuf> {
        let attachments_dir =
            self.wiki_dir().join(domain).join(format!("{entry_name}_attachments"));
        fs::create_dir_all(&attachments_dir).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to create attachments dir: {e}"))
        })?;
        let path = attachments_dir.join(filename);
        fs::write(&path, data)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to write attachment: {e}")))?;
        Ok(path)
    }
}

struct EntityInfo {
    rel_path: String,
    domain: String,
    title: String,
    abstract_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiExtractionResult {
    pub raw_path: PathBuf,
    pub index_path: PathBuf,
    pub log_path: PathBuf,
    pub page_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPayload {
    pub metadata: ExtractionMetadata,
    pub content: String,
    pub prompt: String,
}

impl AgentPayload {
    pub fn from_extraction(
        extraction_result: &StructuredExtractionResult,
        source_file: &Path,
        quality_score: f64,
    ) -> Self {
        let file_hash = WikiStorage::compute_file_hash(&extraction_result.extracted_text);
        let source_name = WikiStorage::extract_source_name(source_file);

        let metadata = ExtractionMetadata {
            source_file: source_file.to_string_lossy().to_string(),
            source_name,
            file_hash,
            extraction_time: Utc::now(),
            page_count: extraction_result.page_count,
            quality_score,
        };

        let prompt = format!(
            r#"# PDF 提取完成

## 任务说明

你是一个专业的**知识库管理员**。请根据 `schema/AGENTS.md` 的规范，处理这份 PDF 提取内容。

## 执行流程

1. **深度通读**：阅读以下提取内容，判断知识所属领域
2. **概念提炼**：提炼 10-15 个核心概念（非机械按章节切片，而是提炼原子化的技术概念）
3. **存量检索**：检查 `wiki/` 目录中是否已存在相关词条
4. **执行编译**：
   - 若概念已存在：将新见解融入现有词条
   - 若概念不存在：创建新词条，使用 `[领域] 概念名称.md` 格式命名
5. **更新索引**：引擎会自动维护 `wiki/index.md` 和 `wiki/log.md`；你仍需保存词条

## 命名示例

不要按"第1章、第2章"命名，而是提炼原子化概念：
- `[IT] Nginx_多进程通信架构.md`
- `[IT] Nginx_事件驱动模型.md`
- `[IT] Nginx_Upstream负载均衡.md`

## 元数据

| 字段 | 值 |
|------|-----|
| 文档名称 | {} |
| 页数 | {} |
| 质量 | {:.0}% |
| 提取时间 | {} |

---

# 提取内容

以下内容已保存到 `raw/{}.md`，请阅读并提炼核心概念：

{}"#,
            metadata.source_name,
            metadata.page_count,
            metadata.quality_score * 100.0,
            metadata.extraction_time.format("%Y-%m-%d %H:%M:%S UTC"),
            metadata.source_name,
            extraction_result.extracted_text
        );

        Self { metadata, content: extraction_result.extracted_text.clone(), prompt }
    }

    pub fn to_markdown(&self) -> String {
        format!("# {}\n\n{}", self.metadata.source_name, self.content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_kb() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("wiki_test_{}", uuid::Uuid::new_v4()));
        let wiki = dir.join("wiki");
        fs::create_dir_all(wiki.join("it")).unwrap();
        fs::write(
            wiki.join("it").join("nested.md"),
            "---\ntitle: Nested\ndomain: IT\n---\n\n# [IT] Nested\n",
        )
        .unwrap();
        fs::write(wiki.join("log.md"), "# Build Log\n\n").unwrap();
        dir
    }

    #[test]
    fn recursive_index_lists_nested_entries() {
        let dir = sample_kb();
        let storage = WikiStorage::new(&dir).unwrap();
        storage.generate_index().unwrap();
        let index = fs::read_to_string(dir.join("wiki/index.md")).unwrap();
        assert!(index.contains("it/nested.md"), "index should list nested path: {index}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_log_adds_line() {
        let dir = sample_kb();
        let storage = WikiStorage::new(&dir).unwrap();
        storage
            .append_log(&NervousEvent::new(NervousEventKind::Save, "path=it/nested.md"))
            .unwrap();
        let log = fs::read_to_string(dir.join("wiki/log.md")).unwrap();
        assert!(log.contains("save"));
        assert!(log.contains("it/nested.md"));
        let _ = fs::remove_dir_all(&dir);
    }
}
