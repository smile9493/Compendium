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

pub mod cognitive_diversity;
pub mod collab;
pub mod compile_pipeline;
pub mod compile_plan;
pub mod confidence_propagation;
pub mod engine;
pub mod entry;
pub mod export;
pub mod hash_cache;
pub mod hub_threshold;
pub mod import;
pub mod index;
pub mod kb_init;
pub mod knowledge_decay;
pub mod markdown_contract;
pub mod patch;
pub mod publish_gate;
pub mod quality;
pub mod quality_issues;
pub mod renderer;
pub mod wiki_lint;

pub use crate::wiki::{NervousEvent, NervousEventKind, sync_nervous_system};
pub use cognitive_diversity::{
    CognitiveDiversityReport, DEFAULT_HUB_IN_DEGREE, NEAR_DUPLICATE_THRESHOLD,
    analyze_cognitive_diversity, deduplicate_search_hits,
};
pub use collab::{
    AuditAction, AuditEvent, EntryLock, PatchProposal, PatchProposalSummary, acquire_lock,
    append_audit, apply_patch_proposal, list_patch_proposals, release_lock, submit_patch_proposal,
};
pub use compile_pipeline::{
    CompleteCompileJobResult, complete_compile_job, run_incremental_extract, run_single_pdf_extract,
};
pub use compile_plan::{CompilePlan, CompilePlanStore, PlanTask, PlanTaskKind, PlanTaskStatus};
pub use confidence_propagation::{
    ConfidencePropagationReport, PropagatedEntry, PropagationPolicy, apply_propagation,
    compute_propagation, run_propagation,
};
pub use engine::KnowledgeEngine;
pub use entry::{
    CompileStatus, EntryConfidence, EntryLevel, EntryType, KnowledgeEntry, MediaAttachment,
    MediaType, PublishStatus, extract_front_matter_yaml, extract_markdown_body,
};
pub use export::{ExportOptions, ExportResult, export_knowledge_base};
pub use hash_cache::HashCache;
pub use hub_threshold::{KEY_HUB_IN_DEGREE, hub_threshold_for_kb};
pub use import::{ImportOptions, ImportResult, import_knowledge_base};
pub use index::{
    Community, FulltextIndex, GraphIndex, IndexCache, LoadBearingEntry, ProtectionLevel,
    RebuildStats, SearchMeta, SearchMode, SearchOptions, SearchResponse, VectorIndex,
    default_search_mode, detect_communities, graph, rebuild_all, rebuild_all_with_policy,
    rebuild_vectors, reindex_entry, search, search_with_mode, search_with_options, wiki_dir,
};
pub use kb_init::{InitKnowledgeBaseResult, init_knowledge_base};
pub use knowledge_decay::{
    DEFAULT_STALE_DAYS, StaleEntry, decay_score, detect_stale_entries, time_decay_factor,
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
pub use wiki_lint::{LintWikiReport, lint_wiki};
