//! # Management Layer
//!
//! Shared management core for all entry points (MCP tools, CLI, Web panel).
//! Provides configuration management, health reporting, and compile status tracking.
//!
//! All entry points call into these modules to ensure data consistency.

pub mod config_manager;
#[cfg(feature = "knowledge")]
pub mod health_reporter;
pub mod types;

pub use config_manager::ConfigManager;
#[cfg(feature = "knowledge")]
pub use health_reporter::HealthReporter;
pub use types::{CompileStatusRecord, HealthReport};
