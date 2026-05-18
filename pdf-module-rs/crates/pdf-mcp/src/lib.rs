//! pdf-mcp library — MCP server, HTTP API, and tool dispatch.

#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::todo)]
#![deny(clippy::dbg_macro)]

pub mod api_doc;
pub mod embed;
pub mod http;
pub mod metrics;
pub mod protocol;
pub mod sampling;
pub mod server;
pub mod tools;
pub mod upload;
