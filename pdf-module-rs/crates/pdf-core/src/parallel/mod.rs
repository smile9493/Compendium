//! Parallel processing module for batch PDF extraction.
//!
//! Provides CPU-bound parallel processing via Rayon for batch PDF files
//! and page-level parallel extraction for large documents.

pub mod batch_processor;
pub mod work_stealing;

pub use batch_processor::{BatchConfig, BatchError, BatchProcessor};
pub use work_stealing::{AdaptiveScheduler, PageTask};
