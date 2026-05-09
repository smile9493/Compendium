//! # Knowledge Engine
//!
//! AI-native knowledge compilation and reasoning engine.
//! Implements the Karpathy compiler pattern: PDFs → structured Markdown → indexed knowledge.
//!
//! ## Architecture
//!
//! - **KnowledgeEntry**: Standardized front matter for all wiki entries
//! - **HashCache**: Merkle-tree-based incremental change detection (rs_merkle)
//! - **KnowledgeEngine**: Orchestrates compile, index, and quality operations
//! - **FulltextIndex**: Tantivy-based full-text search with jieba Chinese segmentation
//! - **GraphIndex**: petgraph-based link graph with disk persistence
//! - **Community Detection**: Label Propagation algorithm for clustering
//! - **VectorIndex**: TF-IDF vector embeddings with cosine similarity search
//! - **CacheDb**: sled-backed K-V store for compilation state

pub mod cache_db;
pub mod engine;
pub mod entry;
pub mod hash_cache;
pub mod index;
pub mod quality;
pub mod renderer;

pub use cache_db::CacheDb;
pub use engine::KnowledgeEngine;
pub use entry::{CompileStatus, EntryLevel, KnowledgeEntry};
pub use hash_cache::HashCache;
pub use index::{detect_communities, Community, FulltextIndex, GraphIndex, VectorIndex};
pub use quality::QualityReport;
pub use renderer::{RenderedEntry, TreeNode, WikiRenderer};
