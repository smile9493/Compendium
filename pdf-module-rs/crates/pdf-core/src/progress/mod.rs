//! Progress tracking module for PDF extraction.
//!
//! Provides real-time progress bars using `indicatif` for both single-file
//! and batch extraction operations.

pub mod tracker;

pub use tracker::{ProgressConfig, ProgressTracker};
