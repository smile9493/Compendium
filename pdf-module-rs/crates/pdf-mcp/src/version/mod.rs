//! # Version & Update Module
//!
//! Provides compile-time version information and update checking
//! against the project's GitHub Releases.
//!
//! ## Architecture
//!
//! - `VersionInfo` is constructed from env vars injected by `build.rs` (VERSION file)
//!   and `CARGO_PKG_VERSION` (Cargo.toml).
//! - `check_for_updates()` queries GitHub Releases API via `ureq` and caches results.
//! - `prepare_update()` downloads a release asset for self-update or Docker readiness.

pub mod github;

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use parking_lot::Mutex as ParkingLotMutex;

/// Current version of the application, embedded at compile time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub major: u32,
    pub minor: u32,
    pub build: u32,
    pub patch: u32,
    /// Human-readable version string, e.g. "149.0.7825.0"
    pub display: String,
    /// Semantic version from Cargo.toml, e.g. "0.3.0"
    pub semver: String,
    /// Deployment mode detected at startup
    pub deployment_mode: DeploymentMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentMode {
    Docker,
    Native,
    Unknown,
}

/// Information about a GitHub release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub prerelease: bool,
    pub html_url: String,
    pub published_at: String,
    pub assets: Vec<AssetInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub name: String,
    pub size: u64,
    pub download_url: String,
}

/// Result of checking for updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    pub current_version: VersionInfo,
    pub update_available: bool,
    pub latest_release: Option<ReleaseInfo>,
    pub checked_at: String,
    pub deployment_mode: DeploymentMode,
}

/// Status of update preparation/download.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePrepareResult {
    pub status: UpdatePrepareStatus,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdatePrepareStatus {
    /// No update needed (already at latest)
    NotNeeded,
    /// Download in progress
    Downloading,
    /// Update package downloaded and ready
    Ready,
    /// Error during download
    Error,
}

/// Parse a build-number version tag like "v149.0.7825.0" into parts.
pub fn parse_version_tag(tag: &str) -> Option<Vec<u32>> {
    let stripped = tag.strip_prefix('v').unwrap_or(tag);
    let parts: Vec<u32> = stripped.split('.').filter_map(|s| s.parse().ok()).collect();
    if parts.len() == 4 { Some(parts) } else { None }
}

/// Compare two version tuples. Returns `Ordering::Greater` if `a > b`.
pub fn is_newer(current: &[u32], latest: &[u32]) -> bool {
    for (a, b) in current.iter().zip(latest.iter()) {
        match a.cmp(b) {
            std::cmp::Ordering::Less => return true,
            std::cmp::Ordering::Greater => return false,
            std::cmp::Ordering::Equal => continue,
        }
    }
    false
}

/// Server-side update check cache with TTL.
pub struct UpdateCache {
    inner: ParkingLotMutex<Option<CachedCheck>>,
    ttl: Duration,
}

struct CachedCheck {
    result: UpdateCheckResult,
    cached_at: Instant,
}

impl UpdateCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            inner: ParkingLotMutex::new(None),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    pub fn get(&self) -> Option<UpdateCheckResult> {
        let guard = self.inner.lock();
        let cached = guard.as_ref()?;
        if cached.cached_at.elapsed() > self.ttl {
            None
        } else {
            Some(cached.result.clone())
        }
    }

    pub fn set(&self, result: UpdateCheckResult) {
        *self.inner.lock() = Some(CachedCheck {
            result,
            cached_at: Instant::now(),
        });
    }

    pub fn invalidate(&self) {
        *self.inner.lock() = None;
    }
}

/// Build `VersionInfo` from compile-time environment variables.
///
/// Environment vars are set by `build.rs` (VERSION file) and Cargo itself.
pub fn current_version(deployment_mode: DeploymentMode) -> VersionInfo {
    let major: u32 = option_env!("VERSION_MAJOR")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let minor: u32 = option_env!("VERSION_MINOR")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let build: u32 = option_env!("VERSION_BUILD")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let patch: u32 = option_env!("VERSION_PATCH")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let display = format!("{major}.{minor}.{build}.{patch}");
    let semver = env!("CARGO_PKG_VERSION").to_string();

    VersionInfo {
        major,
        minor,
        build,
        patch,
        display,
        semver,
        deployment_mode,
    }
}

/// Detect the deployment mode from environment or runtime signals.
pub fn detect_deployment_mode() -> DeploymentMode {
    // Explicit env override
    if let Ok(mode) = std::env::var("DEPLOYMENT_MODE") {
        return match mode.to_lowercase().as_str() {
            "docker" => DeploymentMode::Docker,
            "native" => DeploymentMode::Native,
            _ => DeploymentMode::Unknown,
        };
    }

    // Heuristic: check for Docker-specific files
    if std::path::Path::new("/.dockerenv").exists() {
        return DeploymentMode::Docker;
    }

    DeploymentMode::Native
}
