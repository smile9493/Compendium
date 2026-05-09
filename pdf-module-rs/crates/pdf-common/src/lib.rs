//! # pdf-common
//!
//! Shared types, traits, error definitions, and configuration for the PDF module workspace.
//!
//! ## Modules
//!
//! - [`config`]: Application configuration types
//! - [`dto`]: Data transfer objects for PDF extraction and tool execution
//! - [`error`]: Unified error types (`PdfError`) and result aliases
//! - [`traits`]: Core trait definitions (`FileStorage`, `PluginRegistry`, `ToolHandler`, etc.)
//!
//! ## Key Types
//!
//! - [`PdfError`]: The unified error type for the entire workspace
//! - [`ErrorCategory`]: High-level error classification for monitoring
//! - [`ToolContext`]: Execution context for tool handlers
//! - [`ToolExecutionOptions`]: Configuration options for tool execution

#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(clippy::all)]
#![warn(clippy::await_holding_lock)]
#![warn(clippy::await_holding_refcell_ref)]
#![warn(clippy::large_stack_frames)]
#![warn(clippy::undocumented_unsafe_blocks)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]
#![cfg_attr(not(test), warn(clippy::unwrap_used))]

pub mod config;
pub mod dto;
pub mod error;
pub mod traits;

pub use config::AppConfig;
pub use dto::{ToolContext, ToolExecutionOptions};
pub use error::{ErrorCategory, PdfError, Result};
