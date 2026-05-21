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

pub mod collab;
pub mod compile_pipeline;
pub mod compile_plan;
pub mod engine;
pub mod entry;
pub mod hash_cache;
pub mod index;
pub mod markdown_contract;
pub mod patch;
pub mod publish_gate;
pub mod quality;
pub mod quality_issues;
pub mod renderer;

pub use collab::{
    AuditAction, AuditEvent, EntryLock, PatchProposal, PatchProposalSummary, acquire_lock,
    append_audit, apply_patch_proposal, list_patch_proposals, release_lock, submit_patch_proposal,
};
pub use compile_pipeline::{
    CompleteCompileJobResult, complete_compile_job, run_incremental_extract, run_single_pdf_extract,
};
pub use compile_plan::{CompilePlan, CompilePlanStore, PlanTask, PlanTaskKind, PlanTaskStatus};
pub use engine::KnowledgeEngine;
pub use entry::{CompileStatus, EntryLevel, KnowledgeEntry, PublishStatus};
pub use hash_cache::HashCache;
pub use index::{
    Community, FulltextIndex, GraphIndex, IndexCache, RebuildStats, SearchMeta, SearchMode,
    SearchOptions, SearchResponse, VectorIndex, detect_communities, graph, rebuild_all,
    rebuild_vectors, reindex_entry, search, search_with_mode, search_with_options, wiki_dir,
};
pub use markdown_contract::{MarkdownStructure, analyze_markdown_body};
pub use patch::{WikiPatchRequest, WikiPatchResult, apply_patch, preview_patch, resolve_wiki_path};
pub use publish_gate::{
    GateConfig, GateResult, KEY_AUTO_PUBLISH, KEY_GATE_BLOCK_ON_ERRORS, KEY_QUALITY_MIN_SCORE,
    apply_publish_gate, is_searchable,
};
pub use quality::{QualityReport, analyze_wiki, build_next_actions};
pub use quality_issues::{ListedQualityIssue, fix_suggest, list_quality_issues};
pub use renderer::{RenderedEntry, TreeNode, WikiRenderer};
