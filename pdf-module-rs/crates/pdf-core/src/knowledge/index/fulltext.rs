//! Tantivy-based full-text search index for knowledge entries.
//!
//! Indexes all wiki Markdown files for fast keyword and phrase search.
//! The index is stored at `<knowledge_base>/.rsut_index/tantivy/` and can be
//! fully rebuilt from the wiki files at any time.

use std::fs;
use std::path::{Path, PathBuf};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy};
use tracing::{debug, info};

use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::entry::KnowledgeEntry;
use crate::knowledge::index::tokenizer;

/// A single search hit returned by the fulltext index.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchHit {
    /// Relative path of the entry within wiki/.
    pub path: String,
    /// Title of the entry.
    pub title: String,
    /// Domain of the entry.
    pub domain: String,
    /// Score from Tantivy (higher = more relevant).
    pub score: f32,
    /// Matching snippet (up to 300 chars around the match).
    pub snippet: String,
}

/// Tantivy full-text index for the knowledge wiki.
pub struct FulltextIndex {
    index: Index,
    reader: IndexReader,
    index_dir: PathBuf,
}

// Schema field names
const FIELD_PATH: &str = "path";
const FIELD_TITLE: &str = "title";
const FIELD_DOMAIN: &str = "domain";
const FIELD_TAGS: &str = "tags";
const FIELD_BODY: &str = "body";

impl FulltextIndex {
    /// Open or create the fulltext index at `<knowledge_base>/.rsut_index/tantivy/`.
    pub fn open_or_create(knowledge_base: &Path) -> PdfResult<Self> {
        let index_dir = knowledge_base.join(".rsut_index").join("tantivy");
        Self::open_at(&index_dir)
    }

    /// Open or create a fulltext index at the exact directory path.
    ///
    /// This is used by `FulltextShardManager` to create per-domain indexes
    /// at arbitrary paths under `.rsut_index/tantivy/<domain>/`.
    pub fn open_at(index_dir: &Path) -> PdfResult<Self> {
        fs::create_dir_all(index_dir).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to create tantivy index dir: {}", e))
        })?;

        let mut schema_builder = SchemaBuilder::default();

        // Text fields use the CJK jieba tokenizer for Chinese segmentation support.
        let text_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("cjk")
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();
        let text_options_no_store = TextOptions::default().set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("cjk")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        );

        schema_builder.add_text_field(FIELD_PATH, STRING | STORED);
        schema_builder.add_text_field(FIELD_TITLE, text_options.clone());
        schema_builder.add_text_field(FIELD_DOMAIN, STRING | STORED);
        schema_builder.add_text_field(FIELD_TAGS, text_options.clone());
        schema_builder.add_text_field(FIELD_BODY, text_options_no_store);

        let schema = schema_builder.build();

        let index = if index_dir.join("meta.json").exists() {
            info!(dir = ?index_dir, "Opening existing tantivy index");
            Index::open_in_dir(index_dir).map_err(|e| {
                PdfModuleError::Storage(format!("Failed to open tantivy index: {}", e))
            })?
        } else {
            info!(dir = ?index_dir, "Creating new tantivy index");
            Index::create_in_dir(index_dir, schema.clone()).map_err(|e| {
                PdfModuleError::Storage(format!("Failed to create tantivy index: {}", e))
            })?
        };

        // Register CJK tokenizer
        tokenizer::register_cjk_tokenizer(&index);

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| {
                PdfModuleError::Storage(format!("Failed to create tantivy reader: {}", e))
            })?;

        Ok(Self {
            index,
            reader,
            index_dir: index_dir.to_path_buf(),
        })
    }

    /// Rebuild the entire index by scanning all wiki Markdown files.
    pub fn rebuild(&self, wiki_dir: &Path) -> PdfResult<usize> {
        let schema = self.index.schema();
        let mut writer: IndexWriter = self.index.writer(50_000_000).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to create tantivy writer: {}", e))
        })?;

        // Clear existing documents
        writer.delete_all_documents().map_err(|e| {
            PdfModuleError::Storage(format!("Failed to clear tantivy index: {}", e))
        })?;

        let mut count = 0usize;
        self.scan_and_index(wiki_dir, wiki_dir, &schema, &mut writer, &mut count)?;

        writer.commit().map_err(|e| {
            PdfModuleError::Storage(format!("Failed to commit tantivy index: {}", e))
        })?;

        info!(count = count, "Tantivy index rebuilt");
        Ok(count)
    }

    /// Search the index for a query string.
    pub fn search(&self, query_str: &str, limit: usize) -> PdfResult<Vec<SearchHit>> {
        let schema = self.index.schema();
        let body_field = schema
            .get_field(FIELD_BODY)
            .map_err(|e| PdfModuleError::Storage(format!("Missing body field: {}", e)))?;
        let title_field = schema
            .get_field(FIELD_TITLE)
            .map_err(|e| PdfModuleError::Storage(format!("Missing title field: {}", e)))?;
        let tags_field = schema
            .get_field(FIELD_TAGS)
            .map_err(|e| PdfModuleError::Storage(format!("Missing tags field: {}", e)))?;
        let path_field = schema
            .get_field(FIELD_PATH)
            .map_err(|e| PdfModuleError::Storage(format!("Missing path field: {}", e)))?;
        let domain_field = schema
            .get_field(FIELD_DOMAIN)
            .map_err(|e| PdfModuleError::Storage(format!("Missing domain field: {}", e)))?;

        // Force reload to pick up any recently committed segments
        self.reader.reload().map_err(|e| {
            PdfModuleError::Storage(format!("Failed to reload index reader: {}", e))
        })?;

        let searcher = self.reader.searcher();
        let query_parser =
            QueryParser::for_index(&self.index, vec![title_field, body_field, tags_field]);
        let query = query_parser.parse_query(query_str).map_err(|e| {
            PdfModuleError::Storage(format!("Invalid query '{}': {}", query_str, e))
        })?;

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| PdfModuleError::Storage(format!("Search failed: {}", e)))?;

        let mut hits = Vec::new();
        for (score, doc_addr) in top_docs {
            let doc = searcher
                .doc::<tantivy::TantivyDocument>(doc_addr)
                .map_err(|e| PdfModuleError::Storage(format!("Failed to retrieve doc: {}", e)))?;

            let path = doc
                .get_first(path_field)
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            let title = doc
                .get_first(title_field)
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            let domain = doc
                .get_first(domain_field)
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();

            // Try to read the file for a snippet
            let snippet = self.extract_snippet(&path, query_str);

            hits.push(SearchHit {
                path,
                title,
                domain,
                score,
                snippet,
            });
        }

        Ok(hits)
    }

    #[allow(clippy::only_used_in_recursion)]
    fn scan_and_index(
        &self,
        base: &Path,
        dir: &Path,
        schema: &Schema,
        writer: &mut IndexWriter,
        count: &mut usize,
    ) -> PdfResult<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to read dir: {}", e)))?
        {
            let entry = entry
                .map_err(|e| PdfModuleError::Storage(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();

            if path.is_dir() {
                self.scan_and_index(base, &path, schema, writer, count)?;
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                // Skip index.md and log.md
                if filename == "index.md" || filename == "log.md" {
                    continue;
                }

                if let Ok(content) = fs::read_to_string(&path) {
                    let rel = path
                        .strip_prefix(base)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();

                    let (title, domain, tags, body) =
                        if let Some(entry) = KnowledgeEntry::from_markdown(&content) {
                            let body = content.split("---").nth(2).unwrap_or(&content).to_string();
                            (entry.title, entry.domain, entry.tags.join(" "), body)
                        } else {
                            // Fallback: use filename as title, extract body after front matter
                            let title = filename.replace(".md", "").replace('_', " ");
                            let body = if content.starts_with("---") {
                                content.split("---").nth(2).unwrap_or(&content).to_string()
                            } else {
                                content.clone()
                            };
                            (title, String::new(), String::new(), body)
                        };

                    let path_field = schema.get_field(FIELD_PATH).expect("field exists");
                    let title_field = schema.get_field(FIELD_TITLE).expect("field exists");
                    let domain_field = schema.get_field(FIELD_DOMAIN).expect("field exists");
                    let tags_field = schema.get_field(FIELD_TAGS).expect("field exists");
                    let body_field = schema.get_field(FIELD_BODY).expect("field exists");

                    writer
                        .add_document(doc!(
                            path_field => rel.as_str(),
                            title_field => title.as_str(),
                            domain_field => domain.as_str(),
                            tags_field => tags.as_str(),
                            body_field => body.as_str(),
                        ))
                        .map_err(|e| {
                            PdfModuleError::Storage(format!("Failed to index document: {}", e))
                        })?;

                    *count += 1;
                    debug!(path = %rel, "Indexed entry");
                }
            }
        }

        Ok(())
    }

    fn extract_snippet(&self, rel_path: &str, query: &str) -> String {
        // Try to find the file and extract a snippet around the query match
        let wiki_dir = self
            .index_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("wiki"))
            .unwrap_or_default();
        let full_path = wiki_dir.join(rel_path);

        if let Ok(content) = fs::read_to_string(&full_path) {
            let lower_content = content.to_lowercase();
            let lower_query = query.to_lowercase();
            if let Some(pos) = lower_content.find(&lower_query) {
                let start = pos.saturating_sub(100);
                let end = (pos + query.len() + 200).min(content.len());
                // Find valid UTF-8 boundaries
                let start = content.floor_char_boundary(start);
                let end = content.ceil_char_boundary(end);
                return content[start..end].to_string();
            }
        }
        String::new()
    }
}
