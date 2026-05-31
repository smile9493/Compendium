//! GitHub Releases API client for update checking.
//!
//! Fetches the latest release from the project's GitHub repository
//! and compares the tag version against the current build version.

use serde::Deserialize;
use std::time::Duration;

use super::{
    AssetInfo, ReleaseInfo, UpdateCheckResult, UpdatePrepareResult, UpdatePrepareStatus,
    VersionInfo, is_newer, parse_version_tag,
};

/// GitHub API base for this repository.
const GITHUB_API_RELEASES: &str =
    "https://api.github.com/repos/smile9493/Compendium/releases/latest";

/// Raw GitHub API release response — only fields we need.
#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: String,
    body: Option<String>,
    prerelease: bool,
    html_url: String,
    published_at: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

/// Configuration for the GitHub API client.
pub struct GithubClient {
    /// User-Agent header (required by GitHub API)
    user_agent: String,
    /// Request timeout
    timeout: Duration,
}

impl GithubClient {
    pub fn new() -> Self {
        Self {
            user_agent: format!("pdf-mcp/{}", env!("CARGO_PKG_VERSION")),
            timeout: Duration::from_secs(15),
        }
    }

    /// Fetch the latest release from GitHub.
    pub fn fetch_latest_release(&self) -> Result<ReleaseInfo, String> {
        let response = ureq::AgentBuilder::new()
            .timeout_connect(self.timeout)
            .timeout_read(self.timeout)
            .timeout_write(self.timeout)
            .build()
            .get(GITHUB_API_RELEASES)
            .set("User-Agent", &self.user_agent)
            .set("Accept", "application/vnd.github.v3+json")
            .call()
            .map_err(|e| format!("GitHub API request failed: {e}"))?;

        let release: GithubRelease =
            response.into_json().map_err(|e| format!("Failed to parse GitHub response: {e}"))?;

        Ok(ReleaseInfo {
            tag_name: release.tag_name,
            name: release.name,
            body: release.body.unwrap_or_default(),
            prerelease: release.prerelease,
            html_url: release.html_url,
            published_at: release.published_at,
            assets: release
                .assets
                .into_iter()
                .map(|a| AssetInfo {
                    name: a.name,
                    size: a.size,
                    download_url: a.browser_download_url,
                })
                .collect(),
        })
    }
}

impl Default for GithubClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Perform an update check: compare current version against GitHub latest release.
pub fn check_for_updates(
    client: &GithubClient,
    current: &VersionInfo,
) -> Result<UpdateCheckResult, String> {
    let release = match client.fetch_latest_release() {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Update check failed: {e}");
            return Err(e);
        }
    };

    let current_parts = [current.major, current.minor, current.build, current.patch];
    let latest_parts = parse_version_tag(&release.tag_name);

    let update_available =
        latest_parts.as_ref().is_some_and(|parts| is_newer(&current_parts, parts));

    Ok(UpdateCheckResult {
        current_version: current.clone(),
        update_available,
        latest_release: Some(release),
        checked_at: chrono::Utc::now().to_rfc3339(),
        deployment_mode: current.deployment_mode,
    })
}

/// Download a release asset to a temporary directory.
/// Returns the path to the downloaded file.
pub fn download_release_asset(
    client: &GithubClient,
    asset: &super::AssetInfo,
    progress_callback: impl Fn(u64, u64) + Send + Sync + 'static,
) -> Result<std::path::PathBuf, String> {
    let download_dir = std::env::temp_dir().join("pdf-mcp-update");
    std::fs::create_dir_all(&download_dir)
        .map_err(|e| format!("Failed to create download dir: {e}"))?;

    let output_path = download_dir.join(&asset.name);

    let response = ureq::AgentBuilder::new()
        .timeout_read(Duration::from_secs(600))
        .build()
        .get(&asset.download_url)
        .set("User-Agent", &client.user_agent)
        .set("Accept", "application/octet-stream")
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;

    let total_size =
        response.header("Content-Length").and_then(|s| s.parse().ok()).unwrap_or(asset.size);

    let mut reader = response.into_reader();
    let mut file = std::fs::File::create(&output_path)
        .map_err(|e| format!("Failed to create output file: {e}"))?;

    let mut buf = [0u8; 8192];
    let mut downloaded: u64 = 0;

    loop {
        let n = std::io::Read::read(&mut reader, &mut buf)
            .map_err(|e| format!("Download read error: {e}"))?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut file, &buf[..n])
            .map_err(|e| format!("Download write error: {e}"))?;
        downloaded += n as u64;
        progress_callback(downloaded, total_size);
    }

    Ok(output_path)
}

/// Prepare an update: download the latest release asset.
pub fn prepare_update(
    client: &GithubClient,
    current: &VersionInfo,
    progress_callback: impl Fn(u64, u64) + Send + Sync + 'static,
) -> UpdatePrepareResult {
    let release = match client.fetch_latest_release() {
        Ok(r) => r,
        Err(e) => {
            return UpdatePrepareResult {
                status: UpdatePrepareStatus::Error,
                message: format!("Failed to fetch release: {e}"),
            };
        }
    };

    let current_parts = [current.major, current.minor, current.build, current.patch];
    let latest_parts = parse_version_tag(&release.tag_name);

    let is_update = latest_parts.as_ref().is_some_and(|parts| is_newer(&current_parts, parts));

    if !is_update {
        return UpdatePrepareResult {
            status: UpdatePrepareStatus::NotNeeded,
            message: "Already at the latest version.".to_string(),
        };
    }

    // Find the best asset for this platform
    let Some(asset) = find_best_asset(&release, current) else {
        return UpdatePrepareResult {
            status: UpdatePrepareStatus::Error,
            message: "No suitable release asset found for this platform.".to_string(),
        };
    };

    match download_release_asset(client, &asset, progress_callback) {
        Ok(path) => UpdatePrepareResult {
            status: UpdatePrepareStatus::Ready,
            message: format!(
                "Update downloaded to {}. Restart the server to apply.",
                path.display()
            ),
        },
        Err(e) => UpdatePrepareResult { status: UpdatePrepareStatus::Error, message: e },
    }
}

/// Find the best release asset for the current platform.
fn find_best_asset(release: &ReleaseInfo, _version: &VersionInfo) -> Option<super::AssetInfo> {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;

    for asset in &release.assets {
        let name_lower = asset.name.to_lowercase();
        // Prefer binary assets matching this platform
        if name_lower.contains("pdf-mcp")
            && name_lower.contains(arch)
            && (name_lower.contains(os) || (os == "linux" && name_lower.contains("linux")))
        {
            return Some(asset.clone());
        }
        // Fallback: any pdf-mcp binary
        if name_lower.contains("pdf-mcp") && !name_lower.ends_with(".wasm") {
            return Some(asset.clone());
        }
    }
    None
}
