//! Embedded K-V store for compilation state caching.
//!
//! Uses `sled` (pure Rust embedded database) as a persistent key-value store
//! for compilation state, replacing the JSON-based `.hash_cache` file.
//!
//! ## Design
//!
//! - **Primary key**: source file path (relative)
//! - **Value**: `CacheEntry` serialized via bincode
//! - **Trees**: `entries` (main data), `meta` (schema version, merkle root)
//! - **Migration**: JSON `.hash_cache` is imported on first open; JSON remains
//!   as the export/backup format via `export_json()`.
//!
//! ## Usage
//!
//! ```ignore
//! use pdf_core::knowledge::cache_db::CacheDb;
//!
//! let db = CacheDb::open(knowledge_base)?;
//! db.set_compilation_state("raw/paper.pdf", &entry)?;
//! let entry = db.get_compilation_state("raw/paper.pdf")?;
//! ```

use std::path::{Path, PathBuf};
use tracing::{debug, info};

use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::hash_cache::CacheEntry;

/// Schema version for the K-V store, stored in the `meta` tree.
const SCHEMA_VERSION: u32 = 1;

/// Embedded K-V store backed by sled.
pub struct CacheDb {
    db: sled::Db,
    #[allow(dead_code)]
    db_path: PathBuf,
}

impl CacheDb {
    /// Open or create the cache database at `<knowledge_base>/.cache_db/`.
    ///
    /// On first open, attempts to migrate from `.hash_cache` JSON if it exists.
    pub fn open(knowledge_base: &Path) -> PdfResult<Self> {
        let db_path = knowledge_base.join(".cache_db");
        let db = sled::open(&db_path)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to open cache db: {}", e)))?;

        let this = Self { db, db_path };

        // Check schema version and migrate if needed
        let meta_tree = this
            .db
            .open_tree("meta")
            .map_err(|e| PdfModuleError::Storage(format!("Failed to open meta tree: {}", e)))?;

        let has_version = meta_tree.get("schema_version").ok().flatten().is_some();

        if !has_version {
            this.migrate_from_json(knowledge_base)?;
            meta_tree
                .insert("schema_version", &SCHEMA_VERSION.to_le_bytes())
                .map_err(|e| {
                    PdfModuleError::Storage(format!("Failed to set schema version: {}", e))
                })?;
            info!("Cache DB initialized with schema v{}", SCHEMA_VERSION);
        } else {
            debug!("Cache DB opened at {:?}", this.db_path);
        }

        Ok(this)
    }

    /// Get the compilation state for a source file.
    pub fn get_compilation_state(&self, key: &str) -> PdfResult<Option<CacheEntry>> {
        let tree = self.entries_tree()?;
        match tree
            .get(key)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to get entry: {}", e)))?
        {
            Some(bytes) => {
                let entry: CacheEntry = bincode::deserialize(&bytes).map_err(|e| {
                    PdfModuleError::Storage(format!("Failed to deserialize cache entry: {}", e))
                })?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// Set the compilation state for a source file.
    pub fn set_compilation_state(&self, key: &str, entry: &CacheEntry) -> PdfResult<()> {
        let tree = self.entries_tree()?;
        let bytes = bincode::serialize(entry).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to serialize cache entry: {}", e))
        })?;
        tree.insert(key, bytes)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to insert entry: {}", e)))?;
        Ok(())
    }

    /// Remove a compilation state entry.
    pub fn remove_compilation_state(&self, key: &str) -> PdfResult<bool> {
        let tree = self.entries_tree()?;
        let result = tree
            .remove(key)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to remove entry: {}", e)))?;
        Ok(result.is_some())
    }

    /// Iterate over all compilation state entries.
    pub fn iter_entries(&self) -> PdfResult<Vec<(String, CacheEntry)>> {
        let tree = self.entries_tree()?;
        let mut results = Vec::new();
        for item in tree.iter() {
            let (key_bytes, val_bytes) = item.map_err(|e| {
                PdfModuleError::Storage(format!("Failed to iterate entries: {}", e))
            })?;
            let key = String::from_utf8_lossy(&key_bytes).to_string();
            let entry: CacheEntry = bincode::deserialize(&val_bytes).map_err(|e| {
                PdfModuleError::Storage(format!("Failed to deserialize entry: {}", e))
            })?;
            results.push((key, entry));
        }
        Ok(results)
    }

    /// Get the number of cached entries.
    pub fn len(&self) -> PdfResult<usize> {
        let tree = self.entries_tree()?;
        Ok(tree.len())
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> PdfResult<bool> {
        Ok(self.len()? == 0)
    }

    /// Store a key-value pair in the `meta` tree.
    pub fn set_meta(&self, key: &str, value: &[u8]) -> PdfResult<()> {
        let tree = self.meta_tree()?;
        tree.insert(key, value)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to set meta: {}", e)))?;
        Ok(())
    }

    /// Get a value from the `meta` tree.
    pub fn get_meta(&self, key: &str) -> PdfResult<Option<Vec<u8>>> {
        let tree = self.meta_tree()?;
        let result = tree
            .get(key)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to get meta: {}", e)))?;
        Ok(result.map(|v| v.to_vec()))
    }

    /// Export all entries as a JSON value (for `.hash_cache` export/backup).
    pub fn export_json(&self) -> PdfResult<serde_json::Value> {
        let entries = self.iter_entries()?;
        let map: serde_json::Map<String, serde_json::Value> = entries
            .into_iter()
            .map(|(key, entry)| {
                let val = serde_json::to_value(&entry).unwrap_or(serde_json::Value::Null);
                (key, val)
            })
            .collect();
        Ok(serde_json::Value::Object(map))
    }

    /// Flush all pending writes to disk.
    pub fn flush(&self) -> PdfResult<()> {
        self.db
            .flush()
            .map_err(|e| PdfModuleError::Storage(format!("Failed to flush cache db: {}", e)))?;
        Ok(())
    }

    fn entries_tree(&self) -> PdfResult<sled::Tree> {
        self.db
            .open_tree("entries")
            .map_err(|e| PdfModuleError::Storage(format!("Failed to open entries tree: {}", e)))
    }

    fn meta_tree(&self) -> PdfResult<sled::Tree> {
        self.db
            .open_tree("meta")
            .map_err(|e| PdfModuleError::Storage(format!("Failed to open meta tree: {}", e)))
    }

    /// Migrate entries from the legacy `.hash_cache` JSON file.
    fn migrate_from_json(&self, knowledge_base: &Path) -> PdfResult<()> {
        let json_path = knowledge_base.join(".hash_cache");
        if !json_path.exists() {
            debug!("No .hash_cache found, skipping migration");
            return Ok(());
        }

        let content = std::fs::read_to_string(&json_path).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to read .hash_cache for migration: {}", e))
        })?;

        // Try parsing as PersistedState (new format) or flat map (legacy format)
        // The new format has merkle_root/leaf_paths/entries fields
        if let Ok(state) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(entries) = state.get("entries").and_then(|v| v.as_object()) {
                // New PersistedState format
                let mut migrated = 0usize;
                for (key, val) in entries {
                    if let Ok(entry) = serde_json::from_value::<CacheEntry>(val.clone()) {
                        self.set_compilation_state(key, &entry)?;
                        migrated += 1;
                    }
                }
                info!(
                    count = migrated,
                    "Migrated entries from .hash_cache (new format)"
                );
                return Ok(());
            }

            // Legacy flat map format: { "path": CacheEntry, ... }
            if let Some(map) = state.as_object() {
                let mut migrated = 0usize;
                for (key, val) in map {
                    if let Ok(entry) = serde_json::from_value::<CacheEntry>(val.clone()) {
                        self.set_compilation_state(key, &entry)?;
                        migrated += 1;
                    }
                }
                info!(
                    count = migrated,
                    "Migrated entries from .hash_cache (legacy format)"
                );
                return Ok(());
            }
        }

        debug!(".hash_cache content not recognized, skipping migration");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(hash: &str) -> CacheEntry {
        CacheEntry {
            source_hash: hash.into(),
            compiled_entries: vec!["wiki/a.md".into()],
            last_compiled: "2026-05-04T10:00:00Z".into(),
            instruction_hash: None,
            model_id: None,
        }
    }

    #[test]
    fn test_open_set_get() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = CacheDb::open(dir.path()).unwrap();
        db.set_compilation_state("raw/test.pdf", &make_entry("abc123"))
            .unwrap();
        let loaded = db.get_compilation_state("raw/test.pdf").unwrap().unwrap();
        assert_eq!(loaded.source_hash, "abc123");
        assert_eq!(loaded.compiled_entries, vec!["wiki/a.md"]);
    }

    #[test]
    fn test_remove() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = CacheDb::open(dir.path()).unwrap();
        db.set_compilation_state("raw/a.pdf", &make_entry("abc"))
            .unwrap();
        assert_eq!(db.len().unwrap(), 1);
        db.remove_compilation_state("raw/a.pdf").unwrap();
        assert_eq!(db.len().unwrap(), 0);
        assert!(db.get_compilation_state("raw/a.pdf").unwrap().is_none());
    }

    #[test]
    fn test_meta() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = CacheDb::open(dir.path()).unwrap();
        db.set_meta("merkle_root", b"abc123").unwrap();
        assert_eq!(db.get_meta("merkle_root").unwrap().unwrap(), b"abc123");
    }

    #[test]
    fn test_migration_from_json() {
        let dir = tempfile::TempDir::new().unwrap();

        // Create a legacy .hash_cache JSON with PersistedState format
        let json = r#"{
            "merkle_root": "abc",
            "leaf_paths": ["raw/test.pdf"],
            "entries": {
                "raw/test.pdf": {
                    "source_hash": "def456",
                    "compiled_entries": ["wiki/b.md"],
                    "last_compiled": "2026-05-04T10:00:00Z"
                }
            }
        }"#;
        std::fs::write(dir.path().join(".hash_cache"), json).unwrap();

        let db = CacheDb::open(dir.path()).unwrap();
        let loaded = db.get_compilation_state("raw/test.pdf").unwrap().unwrap();
        assert_eq!(loaded.source_hash, "def456");
        assert_eq!(loaded.compiled_entries, vec!["wiki/b.md"]);
    }

    #[test]
    fn test_iter_and_export() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = CacheDb::open(dir.path()).unwrap();

        for i in 0..3 {
            db.set_compilation_state(
                &format!("raw/{}.pdf", i),
                &make_entry(&format!("hash_{}", i)),
            )
            .unwrap();
        }

        let entries = db.iter_entries().unwrap();
        assert_eq!(entries.len(), 3);

        let json = db.export_json().unwrap();
        assert!(json.is_object());
    }
}
