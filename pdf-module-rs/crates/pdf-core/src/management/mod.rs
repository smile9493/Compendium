//! # Management Layer
//!
//! Shared management core for all entry points (MCP tools, CLI, Web panel).
//! Provides configuration management, health reporting, and compile status tracking.
//!
//! All entry points call into these modules to ensure data consistency.

#[cfg(feature = "knowledge")]
pub mod compile_job;
pub mod compile_status;
pub mod config_manager;
#[cfg(feature = "knowledge")]
pub mod health_reporter;
#[cfg(feature = "knowledge")]
pub mod quality_snapshot;
#[cfg(feature = "knowledge")]
pub mod sync;
pub mod types;
pub mod workspace;

#[cfg(feature = "knowledge")]
pub use compile_job::{
    CompileArtifacts, CompileJob, CompileJobStats, CompileJobStore, CompileJobView, CompileStage,
    CompileStageRecord, CompileTrigger, PipelineStatus, StageStatus, build_compile_status_json,
};
pub use compile_status::{CompileFinishStats, CompileGuard, CompileStatusStore};
pub use config_manager::ConfigManager;
#[cfg(feature = "knowledge")]
pub use health_reporter::HealthReporter;
#[cfg(feature = "knowledge")]
pub use quality_snapshot::{
    QualityIssueBrief, QualitySnapshot, QualitySnapshotStore, refresh_quality_snapshot,
};
#[cfg(feature = "knowledge")]
pub use sync::{
    FileSyncRemote, SyncConflict, SyncConflictResolution, SyncManifest, SyncRemote, SyncReport,
    SyncStatus, build_local_manifest, sync_dir, sync_pull, sync_push, sync_status,
};
pub use types::{CompileStatusRecord, HealthReport};
pub use workspace::{WorkspaceEntry, WorkspaceId, WorkspaceRegistry};
