//! # Management Layer
//!
//! Shared management core for all entry points (MCP tools, CLI, Web panel).
//! Provides configuration management, health reporting, and compile status tracking.
//!
//! All entry points call into these modules to ensure data consistency.

pub mod compile_status;
pub mod config_manager;
#[cfg(feature = "knowledge")]
pub mod health_reporter;
#[cfg(feature = "knowledge")]
pub mod quality_snapshot;
pub mod types;
#[cfg(feature = "knowledge")]
pub mod sync;
pub mod workspace;

pub use compile_status::{CompileFinishStats, CompileGuard, CompileStatusStore};
pub use config_manager::ConfigManager;
#[cfg(feature = "knowledge")]
pub use health_reporter::HealthReporter;
#[cfg(feature = "knowledge")]
pub use quality_snapshot::{
    refresh_quality_snapshot, QualityIssueBrief, QualitySnapshot, QualitySnapshotStore,
};
pub use types::{CompileStatusRecord, HealthReport};
#[cfg(feature = "knowledge")]
pub use sync::{
    build_local_manifest, sync_dir, sync_pull, sync_push, sync_status, FileSyncRemote, SyncManifest,
    SyncRemote, SyncReport, SyncStatus,
};
pub use workspace::{WorkspaceEntry, WorkspaceId, WorkspaceRegistry};
