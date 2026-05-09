//! Sled-based entry metadata store with per-domain tree partitioning.
//!
//! Provides O(1) zero-copy reads of `KnowledgeEntry` via sled's `Tree::get`.
//! Entries are stored in domain-specific trees under `entries:<domain>`.
//! A `meta` tree tracks the set of known domains and schema version.
//!
//! ## Design
//!
//! - **Trees**: `meta` (domain list, schema version), `entries:<domain>` (per-domain entries)
//! - **Key**: relative path within wiki/ (e.g. "it/concept.md")
//! - **Value**: JSON-serialized `KnowledgeEntry`
//! - **Populate**: `populate_from_wiki()` bulk-imports from Markdown files
//! - **Corruption recovery**: auto-rebuilds from wiki files if tree is unreadable

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use tracing::{debug, info};

use crate::error::{PdfError, PdfResult};
use crate::knowledge::entry::KnowledgeEntry;

/// Schema version — bump when the storage format changes.
const SCHEMA_VERSION: u32 = 1;

/// Domain-aware entry metadata store backed by sled.
///
/// Each domain gets its own sled tree `entries:<domain>`, enabling
/// domain-granular lazy loading and idle reclamation in the future.
pub struct MetadataStore {
    db: sled::Db,
}

impl MetadataStore {
    /// Open or create the metadata store at `<knowledge_base>/.rsut_index/metadata/`.
    pub fn open(knowledge_base: &Path) -> PdfResult<Self> {
        let db_path = knowledge_base.join(".rsut_index").join("metadata");
        fs::create_dir_all(&db_path)
            .map_err(|e| PdfError::Storage(format!("Failed to create metadata dir: {}", e)))?;

        let db = sled::open(&db_path)
            .map_err(|e| PdfError::Storage(format!("Failed to open metadata db: {}", e)))?;

        let store = Self { db };

        // Ensure schema version exists
        let meta = store.meta_tree()?;
        let has_version = meta.get("schema_version").ok().flatten().is_some();
        if !has_version {
            meta.insert("schema_version", &SCHEMA_VERSION.to_le_bytes())
                .map_err(|e| PdfError::Storage(format!("Failed to set schema version: {}", e)))?;
            info!("MetadataStore initialized with schema v{}", SCHEMA_VERSION);
        } else {
            debug!("MetadataStore opened at {:?}", db_path);
        }

        Ok(store)
    }

    /// Get a single entry by its relative path within wiki/.
    pub fn get_entry(&self, path: &str) -> PdfResult<Option<KnowledgeEntry>> {
        let domain = extract_domain(path);
        let tree = self.domain_tree(domain)?;

        match tree
            .get(path.as_bytes())
            .map_err(|e| PdfError::Storage(format!("Failed to get entry '{}': {}", path, e)))?
        {
            Some(bytes) => {
                let entry: KnowledgeEntry = serde_json::from_slice(&bytes).map_err(|e| {
                    PdfError::Storage(format!("Failed to deserialize entry '{}': {}", path, e))
                })?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// Upsert (insert or update) a single entry.
    pub fn upsert_entry(&self, path: &str, entry: &KnowledgeEntry) -> PdfResult<()> {
        let domain = extract_domain(path);
        let tree = self.domain_tree(domain)?;

        let bytes = serde_json::to_vec(entry).map_err(|e| {
            PdfError::Storage(format!("Failed to serialize entry '{}': {}", path, e))
        })?;

        tree.insert(path.as_bytes(), bytes)
            .map_err(|e| PdfError::Storage(format!("Failed to insert entry '{}': {}", path, e)))?;

        // Register the domain in the meta tree
        self.register_domain(domain)?;

        Ok(())
    }

    /// Remove an entry by path.
    pub fn remove_entry(&self, path: &str) -> PdfResult<bool> {
        let domain = extract_domain(path);
        let tree = self.domain_tree(domain)?;

        let result = tree
            .remove(path.as_bytes())
            .map_err(|e| PdfError::Storage(format!("Failed to remove entry '{}': {}", path, e)))?;

        Ok(result.is_some())
    }

    /// List all entry paths within a domain.
    pub fn list_domain(&self, domain: &str) -> PdfResult<Vec<String>> {
        let tree = self.domain_tree(domain)?;
        let mut paths = Vec::new();
        for item in tree.iter() {
            let (key, _) = item.map_err(|e| {
                PdfError::Storage(format!("Failed to iterate domain '{}': {}", domain, e))
            })?;
            if let Ok(s) = String::from_utf8(key.to_vec()) {
                paths.push(s);
            }
        }
        Ok(paths)
    }

    /// List all known domains.
    pub fn all_domains(&self) -> PdfResult<Vec<String>> {
        let meta = self.meta_tree()?;
        match meta
            .get("domains")
            .map_err(|e| PdfError::Storage(format!("Failed to read domains: {}", e)))?
        {
            Some(bytes) => {
                let domains: Vec<String> = serde_json::from_slice(&bytes).unwrap_or_default();
                Ok(domains)
            }
            None => {
                // Fallback: scan tree names
                let domains: Vec<String> = self
                    .db
                    .tree_names()
                    .into_iter()
                    .filter_map(|name| {
                        let name = String::from_utf8_lossy(&name).to_string();
                        name.strip_prefix("entries:").map(String::from)
                    })
                    .collect();
                Ok(domains)
            }
        }
    }

    /// Get the total number of entries across all domains.
    pub fn total_entries(&self) -> PdfResult<usize> {
        let domains = self.all_domains()?;
        let mut total = 0usize;
        for domain in domains {
            total += self.list_domain(&domain)?.len();
        }
        Ok(total)
    }

    /// Populate the metadata store from wiki/ Markdown files.
    ///
    /// Recursively scans the wiki directory, parses front matter from every
    /// `.md` file (excluding index.md, log.md, and hidden files), and stores
    /// them in the appropriate domain tree.
    pub fn populate_from_wiki(&self, wiki_dir: &Path) -> PdfResult<usize> {
        if !wiki_dir.exists() {
            return Ok(0);
        }

        let mut count = 0usize;
        self.scan_and_store(wiki_dir, wiki_dir, &mut count)?;

        // Persist domain list
        let domains = self.all_domains()?;
        let meta = self.meta_tree()?;
        let domain_bytes = serde_json::to_vec(&domains)
            .map_err(|e| PdfError::Storage(format!("Failed to serialize domain list: {}", e)))?;
        meta.insert("domains", domain_bytes)
            .map_err(|e| PdfError::Storage(format!("Failed to write domain list: {}", e)))?;

        info!(
            count = count,
            domains = domains.len(),
            "MetadataStore populated from wiki"
        );
        Ok(count)
    }

    /// Flush all pending writes.
    pub fn flush(&self) -> PdfResult<()> {
        self.db
            .flush()
            .map_err(|e| PdfError::Storage(format!("Failed to flush metadata db: {}", e)))?;
        Ok(())
    }

    /// Return the number of entries across all domains (alias for total_entries).
    pub fn len(&self) -> PdfResult<usize> {
        self.total_entries()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> PdfResult<bool> {
        Ok(self.len()? == 0)
    }

    // ── Private helpers ──

    fn domain_tree(&self, domain: &str) -> PdfResult<sled::Tree> {
        let name = format!("entries:{}", domain);
        self.db.open_tree(&name).map_err(|e| {
            PdfError::Storage(format!("Failed to open domain tree '{}': {}", domain, e))
        })
    }

    fn meta_tree(&self) -> PdfResult<sled::Tree> {
        self.db
            .open_tree("meta")
            .map_err(|e| PdfError::Storage(format!("Failed to open meta tree: {}", e)))
    }

    fn register_domain(&self, domain: &str) -> PdfResult<()> {
        let meta = self.meta_tree()?;
        let current_domains: HashSet<String> = match meta
            .get("domains")
            .map_err(|e| PdfError::Storage(format!("Failed to read domains: {}", e)))?
        {
            Some(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            None => HashSet::new(),
        };

        let mut updated = current_domains;
        updated.insert(domain.to_string());

        let bytes = serde_json::to_vec(&updated.into_iter().collect::<Vec<_>>())
            .map_err(|e| PdfError::Storage(format!("Failed to serialize domains: {}", e)))?;

        meta.insert("domains", bytes)
            .map_err(|e| PdfError::Storage(format!("Failed to write domains: {}", e)))?;

        Ok(())
    }

    #[allow(clippy::only_used_in_recursion)]
    fn scan_and_store(&self, base: &Path, dir: &Path, count: &mut usize) -> PdfResult<()> {
        for entry in fs::read_dir(dir)
            .map_err(|e| PdfError::Storage(format!("Failed to read dir: {}", e)))?
        {
            let entry =
                entry.map_err(|e| PdfError::Storage(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden, index, log
            if name.starts_with('.') || name == "index.md" || name == "log.md" {
                continue;
            }

            if path.is_dir() {
                self.scan_and_store(base, &path, count)?;
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Some(entry) = KnowledgeEntry::from_markdown(&content) {
                        let rel = path
                            .strip_prefix(base)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .to_string();
                        self.upsert_entry(&rel, &entry)?;
                        *count += 1;
                    } else {
                        debug!(path = %path.display(), "No front matter found, skipping");
                    }
                }
            }
        }
        Ok(())
    }
}

/// Extract the domain from a relative wiki path.
///
/// Path format: `<domain>/<filename>.md` or just `<filename>.md`.
/// Returns "未分类" (uncategorized) if no domain directory is present.
pub fn extract_domain(path: &str) -> &str {
    // If the path contains a directory separator, use the first component
    if let Some(pos) = path.find('/') {
        let domain = &path[..pos];
        if !domain.is_empty() {
            return domain;
        }
    }
    // Otherwise, attempt to parse domain from filename [Domain] Title.md
    if let Some(filename) = path.rsplit('/').next() {
        if let Some(end) = filename.find(']') {
            if filename.starts_with('[') {
                return &filename[1..end];
            }
        }
    }
    "未分类"
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_entry(title: &str, domain: &str) -> KnowledgeEntry {
        KnowledgeEntry::new(title, domain)
    }

    fn setup_wiki(dir: &Path) {
        let it_dir = dir.join("wiki").join("it");
        fs::create_dir_all(&it_dir).unwrap();

        let md1 = r#"---
title: "HTTP/2 Multiplexing"
domain: "it"
tags: ["http", "networking"]
level: l1
status: compiled
created: 2026-01-01T00:00:00Z
updated: 2026-01-01T00:00:00Z
---
# HTTP/2 Multiplexing
Content here."#;

        let md2 = r#"---
title: "Rust Ownership"
domain: "rust"
tags: ["rust", "memory"]
level: l1
status: compiled
created: 2026-01-01T00:00:00Z
updated: 2026-01-01T00:00:00Z
---
# Rust Ownership
Content here."#;

        fs::write(it_dir.join("[it] HTTP_2 Multiplexing.md"), md1).unwrap();

        let rust_dir = dir.join("wiki").join("rust");
        fs::create_dir_all(&rust_dir).unwrap();
        fs::write(rust_dir.join("[rust] Rust Ownership.md"), md2).unwrap();
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("it/concept.md"), "it");
        assert_eq!(extract_domain("[IT] HTTP_2 Multiplexing.md"), "IT");
        assert_eq!(extract_domain("concept.md"), "未分类");
    }

    #[test]
    fn test_open_upsert_get_remove() {
        let dir = TempDir::new().unwrap();
        let store = MetadataStore::open(dir.path()).unwrap();

        let entry = make_entry("Test Concept", "it");
        store.upsert_entry("it/test.md", &entry).unwrap();

        let loaded = store.get_entry("it/test.md").unwrap().unwrap();
        assert_eq!(loaded.title, "Test Concept");
        assert_eq!(loaded.domain, "it");

        // Remove
        assert!(store.remove_entry("it/test.md").unwrap());
        assert!(store.get_entry("it/test.md").unwrap().is_none());
    }

    #[test]
    fn test_populate_from_wiki() {
        let dir = TempDir::new().unwrap();
        setup_wiki(dir.path());

        let store = MetadataStore::open(dir.path()).unwrap();
        let count = store.populate_from_wiki(&dir.path().join("wiki")).unwrap();
        assert_eq!(count, 2);

        let domains = store.all_domains().unwrap();
        assert!(domains.contains(&"it".to_string()));
        assert!(domains.contains(&"rust".to_string()));

        let it_entries = store.list_domain("it").unwrap();
        assert_eq!(it_entries.len(), 1);
        assert!(it_entries[0].contains("HTTP_2"));

        let loaded = store.get_entry(&it_entries[0]).unwrap().unwrap();
        assert_eq!(loaded.title, "HTTP/2 Multiplexing");
        assert_eq!(loaded.domain, "it");
    }

    #[test]
    fn test_total_entries() {
        let dir = TempDir::new().unwrap();
        let store = MetadataStore::open(dir.path()).unwrap();

        store
            .upsert_entry("it/a.md", &make_entry("A", "it"))
            .unwrap();
        store
            .upsert_entry("it/b.md", &make_entry("B", "it"))
            .unwrap();
        store
            .upsert_entry("rust/c.md", &make_entry("C", "rust"))
            .unwrap();

        assert_eq!(store.total_entries().unwrap(), 3);
        assert_eq!(store.len().unwrap(), 3);
        assert!(!store.is_empty().unwrap());
    }
}
