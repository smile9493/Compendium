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

pub mod engine;
pub mod entry;
pub mod hash_cache;
pub mod index;
pub mod patch;
pub mod quality;
pub mod renderer;

pub use engine::KnowledgeEngine;
pub use entry::{CompileStatus, EntryLevel, KnowledgeEntry};
pub use hash_cache::HashCache;
pub use index::{
    detect_communities, graph, rebuild_all, rebuild_vectors, reindex_entry, search,
    search_with_mode, wiki_dir, Community, FulltextIndex, GraphIndex, RebuildStats, SearchMode,
    VectorIndex,
};
pub use patch::{apply_patch, preview_patch, resolve_wiki_path, WikiPatchRequest, WikiPatchResult};
pub use quality::{build_next_actions, analyze_wiki, QualityReport};
pub use renderer::{RenderedEntry, TreeNode, WikiRenderer};
