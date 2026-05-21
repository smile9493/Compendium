//! MCP tool contracts — input/output JSON Schema for all pdf-mcp tools.
//!
//! Corresponds to Python: MCP tool manifest generation.

#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(clippy::unwrap_used)]
#![allow(ambiguous_glob_reexports)]

pub const CONTRACT_VERSION: &str = "1.0.0";

mod common;
mod extract;
mod index;
mod knowledge;
mod management;
mod platform;
mod registry;

pub use common::*;
pub use extract::*;
pub use index::*;
pub use knowledge::*;
pub use management::*;
pub use platform::*;
pub use registry::{McpToolSpec, all_tool_specs, manifest_sha256, tool_count};

pub fn schema_for<T: schemars::JsonSchema>() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(T)).expect("schema serializes to JSON")
}
