//! Vector embedding engine for semantic search.
//!
//! Provides cosine-similarity-based vector search over knowledge entries.
//! Current MVP uses TF-IDF vectors derived from jieba tokenization.
//! The `EmbeddingModel` trait allows swapping in ONNX model-based
//! embeddings (e.g. BGE-small) when the dependency cost is justified.
//!
//! ## Architecture
//!
//! ```text
//! ┌────────────────────┐
//! │  EmbeddingEngine   │
//! ├────────────────────┤
//! │ - EmbeddingModel   │ → trait: text → Vec<f32>
//! │ - VectorStore      │ → HNSW-like flat index + metadata
//! │ - VectorIndex      │ → disk persistence (bincode)
//! └────────────────────┘
//! ```
//!
//! The vector index is an **enhancement** over the Tantivy fulltext index,
//! not a replacement. Search results are merged via Reciprocal Rank Fusion.

use jieba_rs::Jieba;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tracing::{debug, info};

use crate::error::{PdfModuleError, PdfResult};

static JIEBA: LazyLock<Jieba> = LazyLock::new(Jieba::new);

// ──────────────────────── Embedding Model Trait ────────────────────────

/// Trait for embedding models that convert text into fixed-dimension vectors.
///
/// Implementations:
/// - `TfidfModel`: pure Rust TF-IDF (MVP, zero external deps)
/// - Future: ONNX Runtime BGE-small, etc.
pub trait EmbeddingModel: Send + Sync {
    /// Embed a piece of text into a fixed-dimension vector.
    fn embed(&self, text: &str) -> Vec<f32>;

    /// The dimension of the output vectors.
    fn dimension(&self) -> usize;
}

// ──────────────────────── TF-IDF Model ────────────────────────

/// A simple TF-IDF embedding model.
///
/// Builds a vocabulary from the corpus, then represents each document
/// as a sparse TF-IDF vector projected into a fixed-dimension space
/// via hashing trick (feature hashing).
#[derive(Clone)]
pub struct TfidfModel {
    /// Dimension of the output vector (hash buckets).
    dimension: usize,
    /// IDF weights: term → log(N / df).
    idf: HashMap<String, f32>,
    /// Total number of documents seen during training.
    doc_count: f32,
}

impl TfidfModel {
    /// Create a new TF-IDF model with the given output dimension.
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            idf: HashMap::new(),
            doc_count: 0.0,
        }
    }

    /// Train the model on a corpus of documents (jieba-tokenized).
    pub fn train(&mut self, documents: &[String]) {
        self.doc_count = documents.len() as f32;
        let mut doc_freq: HashMap<String, usize> = HashMap::new();

        for doc in documents {
            let tokens: std::collections::HashSet<String> =
                Self::tokenize(doc).into_iter().collect();
            for token in tokens {
                *doc_freq.entry(token).or_default() += 1;
            }
        }

        for (term, df) in doc_freq {
            // Smoothed IDF: log((N + 1) / (df + 1)) + 1
            let idf = ((self.doc_count + 1.0) / (df as f32 + 1.0)).ln() + 1.0;
            self.idf.insert(term, idf);
        }
    }

    fn tokenize(text: &str) -> Vec<String> {
        JIEBA
            .cut(text, false)
            .into_iter()
            .map(|w| w.trim().to_lowercase())
            .filter(|w| !w.is_empty() && w.len() > 1)
            .collect()
    }

    /// Compute the TF-IDF vector for a document using feature hashing.
    fn tfidf_vector(&self, text: &str) -> Vec<f32> {
        let tokens = Self::tokenize(text);
        let mut tf: HashMap<String, u32> = HashMap::new();
        for token in &tokens {
            *tf.entry(token.clone()).or_default() += 1;
        }
        let total = tokens.len().max(1) as f32;
        let mut vec = vec![0.0f32; self.dimension];

        for (term, count) in &tf {
            let tf_val = (*count as f32) / total;
            let idf_val = self.idf.get(term).copied().unwrap_or(1.0);
            let weight = tf_val * idf_val;
            // Feature hashing: map term to a bucket via hashing
            let bucket = Self::hash_to_bucket(term, self.dimension);
            vec[bucket] += weight;
        }

        // L2 normalize
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vec {
                *v /= norm;
            }
        }
        vec
    }

    fn hash_to_bucket(term: &str, dim: usize) -> usize {
        // DEVIATION: P0-determinism - Using stable FNV-1a hash algorithm for deterministic
        // feature hashing across Rust versions. DefaultHasher is explicitly documented as
        // NOT portable across Rust versions, which would corrupt saved bincode vectors.
        // FNV-1a is a well-defined, version-independent algorithm.
        // Mode: standard, Review: not-required
        let mut hasher = fnv::FnvHasher::default();
        term.hash(&mut hasher);
        (hasher.finish() as usize) % dim
    }
}

impl EmbeddingModel for TfidfModel {
    fn embed(&self, text: &str) -> Vec<f32> {
        self.tfidf_vector(text)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

// ──────────────────────── Vector Entry ────────────────────────

/// A stored vector with its associated metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    /// Relative path of the knowledge entry (e.g., "it/concept.md").
    pub path: String,
    /// The embedding vector.
    pub vector: Vec<f32>,
    /// Title (for display in results).
    pub title: String,
    /// Domain (for filtering).
    pub domain: String,
}

/// A search result from the vector index.
#[derive(Debug, Clone, serde::Serialize)]
pub struct VectorHit {
    /// Relative path of the entry.
    pub path: String,
    /// Title of the entry.
    pub title: String,
    /// Domain of the entry.
    pub domain: String,
    /// Cosine similarity score (0.0 – 1.0).
    pub score: f32,
}

// ──────────────────────── Vector Store ────────────────────────

/// In-memory vector store with brute-force cosine similarity search.
///
/// For the MVP, we use flat (brute-force) search. At scale (1000+ entries),
/// this can be upgraded to HNSW or IVF-PQ without changing the public API.
pub struct VectorStore {
    entries: Vec<VectorEntry>,
}

impl VectorStore {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add or update a vector entry (upsert by path).
    pub fn upsert(&mut self, entry: VectorEntry) {
        if let Some(existing) = self.entries.iter_mut().find(|e| e.path == entry.path) {
            *existing = entry;
        } else {
            self.entries.push(entry);
        }
    }

    /// Search for the top-k most similar entries to a query vector.
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<VectorHit> {
        let mut scored: Vec<VectorHit> = self
            .entries
            .iter()
            .map(|entry| VectorHit {
                path: entry.path.clone(),
                title: entry.title.clone(),
                domain: entry.domain.clone(),
                score: cosine_similarity(query, &entry.vector),
            })
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(top_k);
        scored
    }

    /// Remove an entry by path.
    pub fn remove(&mut self, path: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.path != path);
        self.entries.len() < before
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for VectorStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        (dot / (na * nb)).clamp(-1.0, 1.0)
    }
}

// ──────────────────────── Vector Index (disk persistence) ────────────────────────

/// Serializable snapshot for disk persistence.
#[derive(Serialize, Deserialize)]
struct VectorSnapshot {
    entries: Vec<VectorEntry>,
    dimension: usize,
}

/// Full vector index: combines the embedding model, vector store, and persistence.
pub struct VectorIndex {
    model: TfidfModel,
    store: VectorStore,
    index_dir: PathBuf,
}

impl VectorIndex {
    /// Open or create a vector index at `<knowledge_base>/.rsut_index/vectors/`.
    pub fn open_or_create(knowledge_base: &Path, dimension: usize) -> PdfResult<Self> {
        let index_dir = knowledge_base.join(".rsut_index").join("vectors");
        fs::create_dir_all(&index_dir).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to create vectors index dir: {}", e))
        })?;

        let model = TfidfModel::new(dimension);
        let store = VectorStore::new();

        Ok(Self {
            model,
            store,
            index_dir,
        })
    }

    /// Train the TF-IDF model on the entire corpus of wiki entries.
    /// Must be called before embedding individual entries.
    pub fn train_model(&mut self, documents: &[String]) {
        self.model.train(documents);
        info!(
            count = documents.len(),
            dim = self.model.dimension(),
            "TF-IDF model trained"
        );
    }

    /// Embed and index a single entry.
    pub fn index_entry(&mut self, path: &str, title: &str, domain: &str, text: &str) {
        let combined = format!("{} {}", title, text);
        let vector = self.model.embed(&combined);
        self.store.upsert(VectorEntry {
            path: path.to_string(),
            vector,
            title: title.to_string(),
            domain: domain.to_string(),
        });
    }

    /// Search for semantically similar entries.
    pub fn search(&self, query: &str, top_k: usize) -> Vec<VectorHit> {
        let query_vec = self.model.embed(query);
        self.store.search(&query_vec, top_k)
    }

    /// Remove an entry from the index.
    pub fn remove(&mut self, path: &str) {
        self.store.remove(path);
    }

    /// Persist the vector index to disk.
    pub fn save(&self) -> PdfResult<()> {
        let snapshot = VectorSnapshot {
            entries: self.store.entries.to_vec(),
            dimension: self.model.dimension(),
        };
        let bytes = bincode::serialize(&snapshot).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to serialize vector index: {}", e))
        })?;
        let path = self.index_dir.join("vectors.bin");
        fs::write(&path, &bytes)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to write vector index: {}", e)))?;
        debug!(entries = self.store.len(), path = ?path, "Vector index saved");
        Ok(())
    }

    /// Load the vector index from disk. Returns `None` if no cached index exists.
    pub fn load(knowledge_base: &Path, dimension: usize) -> PdfResult<Option<Self>> {
        let index_dir = knowledge_base.join(".rsut_index").join("vectors");
        let path = index_dir.join("vectors.bin");
        if !path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&path)
            .map_err(|e| PdfModuleError::Storage(format!("Failed to read vector index: {}", e)))?;
        let snapshot: VectorSnapshot = bincode::deserialize(&bytes).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to deserialize vector index: {}", e))
        })?;

        let mut model = TfidfModel::new(dimension);
        let mut store = VectorStore::new();

        // Rebuild IDF from saved entries
        let docs: Vec<String> = snapshot
            .entries
            .iter()
            .map(|e| format!("{} {}", e.title, e.path))
            .collect();
        model.train(&docs);

        for entry in snapshot.entries {
            store.upsert(entry);
        }

        info!(entries = store.len(), "Vector index loaded from disk");
        Ok(Some(Self {
            model,
            store,
            index_dir,
        }))
    }

    /// Get the number of indexed entries.
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];

        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
        assert!((cosine_similarity(&a, &c)).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_normalized() {
        let a = vec![3.0, 4.0];
        let b = vec![3.0, 4.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_tfidf_model_dimensions() {
        let mut model = TfidfModel::new(128);
        model.train(&[
            "Rust programming language systems".into(),
            "Python machine learning AI".into(),
            "Rust memory safety".into(),
        ]);

        let vec1 = model.embed("Rust programming");
        assert_eq!(vec1.len(), 128);

        // Should be L2-normalized
        let norm: f32 = vec1.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5 || norm < 1e-10);
    }

    #[test]
    fn test_vector_store_search() {
        let mut store = VectorStore::new();
        store.upsert(VectorEntry {
            path: "a.md".into(),
            vector: vec![1.0, 0.0, 0.0],
            title: "Entry A".into(),
            domain: "IT".into(),
        });
        store.upsert(VectorEntry {
            path: "b.md".into(),
            vector: vec![0.0, 1.0, 0.0],
            title: "Entry B".into(),
            domain: "Math".into(),
        });

        let results = store.search(&[1.0, 0.0, 0.0], 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].path, "a.md");
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn test_vector_store_upsert() {
        let mut store = VectorStore::new();
        store.upsert(VectorEntry {
            path: "a.md".into(),
            vector: vec![1.0, 0.0],
            title: "Old".into(),
            domain: "IT".into(),
        });
        store.upsert(VectorEntry {
            path: "a.md".into(),
            vector: vec![0.0, 1.0],
            title: "New".into(),
            domain: "IT".into(),
        });
        assert_eq!(store.len(), 1);
        let results = store.search(&[0.0, 1.0], 1);
        assert_eq!(results[0].title, "New");
    }

    #[test]
    fn test_vector_index_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let docs = vec![
            "Rust systems programming".to_string(),
            "Machine learning with Python".to_string(),
        ];

        {
            let mut idx = VectorIndex::open_or_create(dir.path(), 64).unwrap();
            idx.train_model(&docs);
            idx.index_entry("a.md", "Rust", "IT", "systems programming language");
            idx.index_entry("b.md", "ML", "AI", "machine learning models");
            idx.save().unwrap();
        }

        {
            let idx = VectorIndex::load(dir.path(), 64).unwrap().unwrap();
            assert_eq!(idx.len(), 2);
            let results = idx.search("Rust systems", 2);
            assert!(!results.is_empty());
            assert_eq!(results[0].path, "a.md");
        }
    }
}
