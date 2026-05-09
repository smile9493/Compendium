//! Wiki knowledge base — extraction metadata, raw storage, and log management.
//!
//! Manages the wiki file system: saves extracted data as YAML metadata,
//! writes raw extraction outputs, maintains versioned hashes for incremental
//! compilation, and keeps an audit log.

use std::fs::{self, File};
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::dto::StructuredExtractionResult;
use crate::error::{PdfModuleError, PdfResult};

/// Extraction metadata stored alongside each raw extraction.
///
/// Includes source file information, SHA-256 hash for incremental detection,
/// and quality scoring for VLM fallback decisions.
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
///
/// Operations:
/// - `save_raw()` / `load_raw()` — YAML-serialized extraction data
/// - `save_metadata()` / `load_metadata()` — per-file extraction provenance
/// - `log()` / `read_log()` — human-readable audit log
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

        self.save_raw_file(&raw_path, &metadata, &extraction_result.extracted_text)?;

        self.generate_index()?;

        Ok(WikiExtractionResult {
            raw_path,
            index_path: self.base_path.join("wiki").join("index.md"),
            log_path: self.base_path.join("wiki").join("log.md"),
            page_count: extraction_result.page_count,
        })
    }

    fn save_raw_file(
        &self,
        path: &Path,
        metadata: &ExtractionMetadata,
        text: &str,
    ) -> PdfResult<()> {
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

        let mut file = File::create(path)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to create raw file: {}", e)))?;

        file.write_all(content.as_bytes())
            .map_err(|e| PdfModuleError::Storage(format!("Failed to write raw file: {}", e)))?;

        Ok(())
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

    pub fn generate_index(&self) -> PdfResult<PathBuf> {
        let index_path = self.base_path.join("wiki").join("index.md");
        let wiki_dir = self.base_path.join("wiki");

        let mut entities: Vec<EntityInfo> = Vec::new();

        if wiki_dir.exists() {
            for entry in fs::read_dir(&wiki_dir)
                .map_err(|e| PdfModuleError::Storage(format!("Read wiki dir error: {}", e)))?
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                if path.extension().map(|e| e == "md").unwrap_or(false)
                    && name != "index.md"
                    && name != "log.md"
                {
                    if let Some(info) = Self::parse_entity(&path) {
                        entities.push(info);
                    }
                }
            }
        }

        entities.sort_by(|a, b| a.domain.cmp(&b.domain).then(a.title.cmp(&b.title)));

        let content = Self::build_index(&entities);

        let mut file = File::create(&index_path)
            .map_err(|e| PdfModuleError::Storage(format!("Index create error: {}", e)))?;

        file.write_all(content.as_bytes())
            .map_err(|e| PdfModuleError::Storage(format!("Index write error: {}", e)))?;

        Ok(index_path)
    }

    fn parse_entity(path: &Path) -> Option<EntityInfo> {
        let content = fs::read_to_string(path).ok()?;
        let filename = path.file_name()?.to_string_lossy().to_string();

        let title = content
            .lines()
            .find(|l| l.starts_with("# "))
            .map(|l| l[2..].to_string())
            .unwrap_or_else(|| filename.replace(".md", ""));

        let (domain, name) = if title.starts_with('[') {
            let end = title.find(']')?;
            (title[1..end].to_string(), title[end + 2..].to_string())
        } else {
            ("未分类".to_string(), title)
        };

        let abstract_text = content
            .lines()
            .skip_while(|l| !l.contains("[!ABSTRACT]"))
            .nth(1)
            .map(|l| l.trim().to_string())
            .unwrap_or_default();

        Some(EntityInfo {
            filename,
            domain,
            title: name,
            abstract_text,
        })
    }

    fn build_index(entities: &[EntityInfo]) -> String {
        let mut content = String::new();

        content.push_str("# 知识索引\n\n");
        content.push_str("> [!ABSTRACT] 摘要\n");
        content.push_str("> 本页面是 Wiki 知识库的总导航图，按领域分类组织。\n\n");

        if entities.is_empty() {
            content.push_str("*暂无词条。请使用 AI Agent 处理 raw/ 中的原始素材。*\n\n");
        } else {
            let mut current_domain = String::new();

            for entity in entities {
                if entity.domain != current_domain {
                    current_domain = entity.domain.clone();
                    content.push_str(&format!("\n## [{}]\n\n", current_domain));
                    content.push_str("| 词条 | 摘要 |\n");
                    content.push_str("|------|------|\n");
                }

                content.push_str(&format!(
                    "| [[{}]] | {} |\n",
                    entity.filename, entity.abstract_text
                ));
            }
        }

        content.push_str("\n---\n\n");
        content.push_str(&format!("- **词条总数**: {}\n", entities.len()));
        content.push_str("- [[log.md]] - 编译日志\n");

        content
    }

    fn extract_source_name(path: &Path) -> String {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    }

    fn compute_file_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

struct EntityInfo {
    filename: String,
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

你是一个专业的**知识库管理员**。请根据 `schema/CLAUDE.md` 的规范，处理这份 PDF 提取内容。

## 执行流程

1. **深度通读**：阅读以下提取内容，判断知识所属领域
2. **概念提炼**：提炼 10-15 个核心概念（非机械按章节切片，而是提炼原子化的技术概念）
3. **存量检索**：检查 `wiki/` 目录中是否已存在相关词条
4. **执行编译**：
   - 若概念已存在：将新见解融入现有词条
   - 若概念不存在：创建新词条，使用 `[领域] 概念名称.md` 格式命名
5. **更新索引**：更新 `wiki/index.md` 和 `wiki/log.md`

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

        Self {
            metadata,
            content: extraction_result.extracted_text.clone(),
            prompt,
        }
    }

    pub fn to_markdown(&self) -> String {
        format!("# {}\n\n{}", self.metadata.source_name, self.content)
    }
}
