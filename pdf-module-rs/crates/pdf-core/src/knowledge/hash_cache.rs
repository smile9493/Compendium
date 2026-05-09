//! Hash cache for incremental compilation.
//!
//! Uses `rs_merkle` for a proper Merkle tree implementation.
//! Each leaf node represents a source file's SHA-256 hash.
//! The Merkle root serves as a "state fingerprint" for the entire `raw/` directory.
//!
//! Stored as a JSON file at `<knowledge_base>/.hash_cache`.
//! Enables O(n) change detection: rebuild tree, compare roots.

use rs_merkle::{Hasher, MerkleTree};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use crate::error::{PdfModuleError, PdfResult};

/// SHA-256 adapter for rs_merkle.
#[derive(Clone)]
pub struct Sha256Algorithm;

impl Hasher for Sha256Algorithm {
    type Hash = [u8; 32];

    fn hash(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result);
        out
    }
}

/// A single entry in the hash cache, mapping a source file path to its hash
/// and the list of wiki entries compiled from it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// SHA-256 hex digest of the source file.
    pub source_hash: String,
    /// Relative paths of wiki entries compiled from this source.
    pub compiled_entries: Vec<String>,
    /// Timestamp of last compilation (RFC 3339).
    pub last_compiled: String,
    /// SHA-256 of the compilation instructions (e.g., `schema/CLAUDE.md`).
    /// When this changes, entries may need recompilation even if source PDF is unchanged.
    #[serde(default)]
    pub instruction_hash: Option<String>,
    /// Identifier of the AI model used for compilation (e.g., "claude-sonnet-4").
    /// Tracks provenance for quality drift detection.
    #[serde(default)]
    pub model_id: Option<String>,
}

/// Persisted state of the Merkle tree + entry metadata.
#[derive(Debug, Serialize, Deserialize)]
struct PersistedState {
    /// The Merkle root hex string.
    merkle_root: String,
    /// Ordered list of leaf paths (same order as Merkle tree leaves).
    leaf_paths: Vec<String>,
    /// Per-file compilation metadata.
    entries: HashMap<String, CacheEntry>,
}

/// Merkle-based hash cache for incremental compilation.
///
/// Tracks the state of all files in `raw/` using a Merkle tree.
/// The root hash acts as a fingerprint — if the root matches, nothing changed.
pub struct HashCache {
    entries: HashMap<String, CacheEntry>,
    cache_path: PathBuf,
    /// The last known Merkle root (for fast "any change?" checks).
    last_merkle_root: Option<[u8; 32]>,
}

impl HashCache {
    /// Load the hash cache from disk, or create an empty one if not found.
    pub fn load_or_create(knowledge_base: &Path) -> PdfResult<Self> {
        let cache_path = knowledge_base.join(".hash_cache");
        if cache_path.exists() {
            let content = fs::read_to_string(&cache_path).map_err(|e| {
                PdfModuleError::Storage(format!("Failed to read hash cache: {}", e))
            })?;
            let state: PersistedState = serde_json::from_str(&content).map_err(|e| {
                PdfModuleError::Storage(format!("Failed to parse hash cache: {}", e))
            })?;
            let last_root = Self::parse_root_hex(&state.merkle_root);
            info!(entries = state.entries.len(), root = %state.merkle_root, "Loaded hash cache");
            Ok(Self {
                entries: state.entries,
                cache_path,
                last_merkle_root: last_root,
            })
        } else {
            info!("No hash cache found, creating empty cache");
            Ok(Self {
                entries: HashMap::new(),
                cache_path,
                last_merkle_root: None,
            })
        }
    }

    /// Persist the cache to disk.
    pub fn save(&self) -> PdfResult<()> {
        // Rebuild the leaf list from current entries to compute the root.
        let mut leaf_paths: Vec<String> = self.entries.keys().cloned().collect();
        leaf_paths.sort();
        let leaf_hashes: Vec<[u8; 32]> = leaf_paths
            .iter()
            .map(|p| {
                let entry = &self.entries[p];
                Self::hex_to_hash(&entry.source_hash)
            })
            .collect();

        let root_hex = if leaf_hashes.is_empty() {
            String::new()
        } else {
            let tree = MerkleTree::<Sha256Algorithm>::from_leaves(&leaf_hashes);
            tree.root().map(hex::encode).unwrap_or_default()
        };

        let state = PersistedState {
            merkle_root: root_hex,
            leaf_paths,
            entries: self.entries.clone(),
        };
        let json = serde_json::to_string_pretty(&state).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to serialize hash cache: {}", e))
        })?;
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                PdfModuleError::Storage(format!("Failed to create cache dir: {}", e))
            })?;
        }
        fs::write(&self.cache_path, json)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to write hash cache: {}", e)))?;
        debug!(path = ?self.cache_path, "Hash cache saved");
        Ok(())
    }

    /// Compute the Merkle root of all tracked files.
    /// Returns `None` if there are no tracked files.
    pub fn compute_merkle_root(&self) -> Option<[u8; 32]> {
        if self.entries.is_empty() {
            return None;
        }
        let mut leaf_paths: Vec<&String> = self.entries.keys().collect();
        leaf_paths.sort();
        let leaf_hashes: Vec<[u8; 32]> = leaf_paths
            .iter()
            .map(|p| Self::hex_to_hash(&self.entries[p.as_str()].source_hash))
            .collect();
        let tree = MerkleTree::<Sha256Algorithm>::from_leaves(&leaf_hashes);
        tree.root()
    }

    /// Check if the tracked file set has changed since last save.
    /// Returns `true` if the current Merkle root differs from the persisted one.
    pub fn has_changes(&self) -> bool {
        let current = self.compute_merkle_root();
        current != self.last_merkle_root
    }

    /// Compute SHA-256 of a file.
    pub fn hash_file(path: &Path) -> PdfResult<String> {
        let bytes = fs::read(path).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to read file for hashing: {}", e))
        })?;
        Ok(Self::hash_bytes(&bytes))
    }

    /// Compute SHA-256 of a byte slice.
    pub fn hash_bytes(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Check if a source file needs recompilation.
    /// Returns `true` if:
    /// - The source is not in the cache (new file), or
    /// - The source hash has changed (file modified), or
    /// - No compiled entries exist (compilation incomplete).
    pub fn needs_compile(&self, source_path: &Path) -> PdfResult<bool> {
        let key = source_path.to_string_lossy().to_string();
        let current_hash = Self::hash_file(source_path)?;

        match self.entries.get(&key) {
            None => {
                debug!(source = %key, "New source, needs compile");
                Ok(true)
            }
            Some(entry) => {
                let changed = entry.source_hash != current_hash;
                let empty = entry.compiled_entries.is_empty();
                if changed {
                    debug!(source = %key, "Source hash changed, needs recompile");
                } else if empty {
                    debug!(source = %key, "No compiled entries, needs compile");
                }
                Ok(changed || empty)
            }
        }
    }

    /// Get the list of sources that need compilation.
    pub fn get_pending_sources(&self, raw_dir: &Path) -> PdfResult<Vec<PathBuf>> {
        let mut pending = Vec::new();
        if !raw_dir.exists() {
            return Ok(pending);
        }
        for entry in fs::read_dir(raw_dir)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to read raw dir: {}", e)))?
        {
            let entry = entry
                .map_err(|e| PdfModuleError::Storage(format!("Failed to read dir entry: {}", e)))?;
            let path = entry.path();
            if path.extension().map(|e| e == "pdf").unwrap_or(false) && self.needs_compile(&path)? {
                pending.push(path);
            }
        }
        Ok(pending)
    }

    /// Record a successful compilation.
    pub fn record_compile(
        &mut self,
        source_path: &Path,
        compiled_entries: Vec<String>,
    ) -> PdfResult<()> {
        self.record_compile_with_metadata(source_path, compiled_entries, None, None)
    }

    /// Record a successful compilation with full dependency metadata.
    pub fn record_compile_with_metadata(
        &mut self,
        source_path: &Path,
        compiled_entries: Vec<String>,
        instruction_hash: Option<String>,
        model_id: Option<String>,
    ) -> PdfResult<()> {
        let key = source_path.to_string_lossy().to_string();
        let hash = Self::hash_file(source_path)?;
        let now = chrono::Utc::now().to_rfc3339();

        self.entries.insert(
            key,
            CacheEntry {
                source_hash: hash,
                compiled_entries,
                last_compiled: now,
                instruction_hash,
                model_id,
            },
        );
        Ok(())
    }

    /// Check if entries need recompilation due to instruction changes.
    ///
    /// Returns the list of source paths whose `instruction_hash` does not match
    /// the given current instruction hash. These entries were compiled with
    /// outdated instructions and may need regeneration.
    pub fn stale_by_instruction(&self, current_instruction_hash: &str) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|(_, entry)| {
                entry
                    .instruction_hash
                    .as_deref()
                    .map(|h| h != current_instruction_hash)
                    .unwrap_or(true)
            })
            .map(|(key, _)| key.as_str())
            .collect()
    }

    /// Get the cache entry for a specific source.
    pub fn get(&self, source_path: &Path) -> Option<&CacheEntry> {
        let key = source_path.to_string_lossy().to_string();
        self.entries.get(&key)
    }

    /// Remove a source from the cache (e.g., when the file is deleted).
    pub fn remove(&mut self, source_path: &Path) -> Option<CacheEntry> {
        let key = source_path.to_string_lossy().to_string();
        self.entries.remove(&key)
    }

    /// Return the number of cached sources.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over all entries.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &CacheEntry)> {
        self.entries.iter()
    }

    fn parse_root_hex(hex_str: &str) -> Option<[u8; 32]> {
        let bytes = hex::decode(hex_str).ok()?;
        if bytes.len() != 32 {
            return None;
        }
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        Some(out)
    }

    fn hex_to_hash(hex_str: &str) -> [u8; 32] {
        Self::parse_root_hex(hex_str).unwrap_or([0u8; 32])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_hash_cache_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let cache_path = dir.path().join(".hash_cache");

        let mut cache = HashCache {
            entries: HashMap::new(),
            cache_path: cache_path.clone(),
            last_merkle_root: None,
        };

        // Create a fake PDF
        let pdf_path = dir.path().join("test.pdf");
        let mut f = fs::File::create(&pdf_path).unwrap();
        f.write_all(b"%PDF-1.4 fake content").unwrap();

        cache
            .record_compile(&pdf_path, vec!["wiki/it/concept.md".into()])
            .unwrap();
        cache.save().unwrap();

        // Reload and verify
        let loaded = HashCache::load_or_create(dir.path()).unwrap();
        assert_eq!(loaded.len(), 1);
        let entry = loaded.get(&pdf_path).unwrap();
        assert_eq!(entry.compiled_entries, vec!["wiki/it/concept.md"]);
    }

    #[test]
    fn test_needs_compile() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut cache = HashCache {
            entries: HashMap::new(),
            cache_path: dir.path().join(".hash_cache"),
            last_merkle_root: None,
        };

        let pdf_path = dir.path().join("test.pdf");
        let mut f = fs::File::create(&pdf_path).unwrap();
        f.write_all(b"%PDF-1.4 content v1").unwrap();

        // New file -> needs compile
        assert!(cache.needs_compile(&pdf_path).unwrap());

        // Record compile
        cache
            .record_compile(&pdf_path, vec!["wiki/it/c.md".into()])
            .unwrap();
        assert!(!cache.needs_compile(&pdf_path).unwrap());

        // Modify file -> needs recompile
        let mut f = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&pdf_path)
            .unwrap();
        f.write_all(b"%PDF-1.4 content v2 modified").unwrap();
        assert!(cache.needs_compile(&pdf_path).unwrap());
    }

    #[test]
    fn test_merkle_root_changes() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut cache = HashCache {
            entries: HashMap::new(),
            cache_path: dir.path().join(".hash_cache"),
            last_merkle_root: None,
        };

        let pdf1 = dir.path().join("a.pdf");
        let pdf2 = dir.path().join("b.pdf");
        fs::File::create(&pdf1)
            .unwrap()
            .write_all(b"content_a")
            .unwrap();
        fs::File::create(&pdf2)
            .unwrap()
            .write_all(b"content_b")
            .unwrap();

        cache
            .record_compile(&pdf1, vec!["wiki/a.md".into()])
            .unwrap();
        cache
            .record_compile(&pdf2, vec!["wiki/b.md".into()])
            .unwrap();

        let root1 = cache.compute_merkle_root().unwrap();
        cache.save().unwrap();

        // Reload — root should match
        let mut loaded = HashCache::load_or_create(dir.path()).unwrap();
        assert_eq!(loaded.last_merkle_root, Some(root1));
        assert!(!loaded.has_changes());

        // Modify file — root should differ
        fs::File::create(&pdf1)
            .unwrap()
            .write_all(b"modified")
            .unwrap();
        loaded
            .entries
            .get_mut(&pdf1.to_string_lossy().to_string())
            .unwrap()
            .source_hash = HashCache::hash_bytes(b"modified");
        assert!(loaded.has_changes());
    }
}
