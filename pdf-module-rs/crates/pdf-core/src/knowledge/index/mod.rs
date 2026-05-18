//! # Cognitive Index Layer
//!
//! Provides fast discovery and association across knowledge entries.
//!
//! - **FulltextIndex**: Tantivy-based full-text search with CJK support
//! - **GraphIndex**: petgraph-based link graph for neighbor discovery, orphan detection, and concept maps
//! - **JiebaTokenizer**: jieba-rs powered Chinese word segmentation tokenizer
//! - **Community Detection**: Label Propagation algorithm for clustering related entries
//! - **VectorIndex**: TF-IDF vector embeddings with cosine similarity search

pub mod cache;
pub mod community;
pub mod facade;
pub mod fulltext;
pub mod fulltext_shard;
pub mod graph;
pub mod graph_partition;
pub mod lazy_loader;
pub mod metadata_store;
pub mod tokenizer;
pub mod vector;

pub use community::{detect_communities, Community};
pub use cache::{IndexCache, KbIndexes};
pub use facade::{
    graph, rebuild_all, rebuild_vectors, reindex_entry, search, search_with_mode,
    search_with_options, search_with_options_ft, wiki_dir, RebuildStats, SearchMeta, SearchMode,
    SearchOptions, SearchResponse,
};
pub use fulltext::FulltextIndex;
pub use fulltext_shard::FulltextShardManager;
pub use graph::GraphIndex;
pub use graph_partition::GraphPartitionStore;
pub use lazy_loader::LazyLoadingCoordinator;
pub use metadata_store::{extract_domain, MetadataStore};
pub use tokenizer::register_cjk_tokenizer;
pub use vector::{
    cosine_similarity, EmbeddingModel, TfidfModel, VectorHit, VectorIndex, VectorStore,
};
