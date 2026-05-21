//! PDF MCP Module - 宗师级PDF提取管道
//!
//! 遵循 Rust Coding Standards Skills v9.0.0 规范
//! - P0 Safety: 内存安全、FFI隔离、错误处理
//! - P1 Maintainability: 语义命名、代码组织
//! - P2 Compile Time: 无过度泛型化
//! - P3 Performance: 零拷贝、Arena分配
//!
//! ## Cargo Features
//!
//! - `knowledge` (default): Knowledge engine, wiki compilation, fulltext/graph/vector indexes.
//!   Dependencies: tantivy, petgraph, sled, jieba-rs, etc.
//! - `vlm` (default): VLM-powered PDF enhancement via `vlm-visual-gateway`.
//!   Without this feature, the pipeline operates in local-only (Pdfium) mode.

#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(clippy::all)]
#![deny(clippy::await_holding_lock)]
#![deny(clippy::await_holding_refcell_ref)]
#![deny(clippy::large_stack_frames)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![cfg_attr(test, allow(clippy::undocumented_unsafe_blocks))]
#![deny(clippy::todo)]
#![deny(clippy::dbg_macro)]
#![cfg_attr(not(test), warn(clippy::unwrap_used))]
#![cfg_attr(test, allow(clippy::unwrap_used))]
// Harden to deny once all public items have documentation:
// #![deny(missing_docs)]

pub mod config;
#[cfg(feature = "dhat-heap")]
pub mod dhat_profiler;
pub mod dto;
pub mod engine;
pub mod error;
pub mod extraction;
pub mod extractor;
#[cfg(feature = "knowledge")]
pub mod knowledge;
pub mod management;
pub mod mmap_loader;
pub mod parallel;
pub mod quality_probe;
pub mod tracing_setup;
pub mod validator;
#[cfg(feature = "knowledge")]
pub mod wiki;

pub use config::ServerConfig;
pub use extraction::{ExtractionBackend, ExtractionRouter, RemoteExtractionConfig};
pub use extractor::McpPdfPipeline;
#[cfg(feature = "knowledge")]
pub use knowledge::{
    ExportOptions, ExportResult, FulltextIndex, GraphIndex, ImportOptions, ImportResult,
    InitKnowledgeBaseResult, KnowledgeEngine, WikiRenderer, export_knowledge_base,
    extract_front_matter_yaml, extract_markdown_body, import_knowledge_base, init_knowledge_base,
    lint_wiki,
};
pub use tracing_setup::{
    LogFormat, TracingConfig, init_compact, init_development, init_production, init_with_config,
    request_span,
};
pub use validator::{FileValidator, PathValidationConfig};
#[cfg(feature = "knowledge")]
pub use wiki::{NervousEvent, NervousEventKind, WikiStorage, sync_nervous_system};
