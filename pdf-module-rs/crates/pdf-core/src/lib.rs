//! PDF MCP Module - 宗师级PDF提取管道
//!
//! 遵循 Rust Coding Standards Skills v9.0.0 规范
//! - P0 Safety: 内存安全、FFI隔离、错误处理
//! - P1 Maintainability: 语义命名、代码组织
//! - P2 Compile Time: 无过度泛型化
//! - P3 Performance: 零拷贝、Arena分配

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

pub mod cache;
pub mod config;
pub mod dto;
pub mod engine;
pub mod error;
pub mod extractor;
pub mod knowledge;
pub mod management;
pub mod mmap_loader;
pub mod parallel;
pub mod progress;
pub mod quality_probe;
pub mod validator;
pub mod vlm_pipeline;
pub mod wiki;

pub use cache::LruTtlCache;
pub use config::ServerConfig;
pub use extractor::McpPdfPipeline;
pub use knowledge::{FulltextIndex, GraphIndex, KnowledgeEngine, WikiRenderer};
pub use validator::{FileValidator, PathValidationConfig};
