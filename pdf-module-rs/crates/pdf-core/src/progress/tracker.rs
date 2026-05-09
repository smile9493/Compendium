//! Progress tracking system using indicatif for real-time progress display.
//!
//! Supports both single-file page-level progress and batch file-level progress
//! with configurable templates, speed display, and ETA estimation.

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Arc;

/// Configuration for progress display behavior.
#[derive(Debug, Clone)]
pub struct ProgressConfig {
    /// Show processing speed (files/second or pages/second).
    pub show_speed: bool,
    /// Show estimated time remaining.
    pub show_eta: bool,
    /// Show percentage completion.
    pub show_percentage: bool,
    /// Custom progress bar template string.
    pub template: String,
}

impl Default for ProgressConfig {
    fn default() -> Self {
        Self {
            show_speed: true,
            show_eta: true,
            show_percentage: true,
            template: "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}".to_string(),
        }
    }
}

/// Progress tracker that manages multiple progress bars for parallel operations.
///
/// Uses `indicatif::MultiProgress` to display concurrent progress bars
/// for batch and per-file operations.
pub struct ProgressTracker {
    #[allow(dead_code)]
    multi_progress: Arc<MultiProgress>,
    config: ProgressConfig,
}

impl ProgressTracker {
    pub fn new(config: ProgressConfig) -> Self {
        Self {
            multi_progress: Arc::new(MultiProgress::new()),
            config,
        }
    }

    /// Create a progress bar for a single file's page extraction.
    pub fn create_file_progress(&self, file_name: &str, total_pages: u64) -> ProgressBar {
        let pb = self.multi_progress.add(ProgressBar::new(total_pages));
        pb.set_style(
            ProgressStyle::with_template(&self.config.template)
                .expect("valid progress template")
                .progress_chars("█▉▊▋▌▍▎▏  ")
                .tick_chars("⠁⠃⠇⡇⣇⣧⣷⣿"),
        );
        pb.set_message(format!("Processing {}", file_name));
        pb
    }

    /// Create a progress bar for batch file processing.
    pub fn create_batch_progress(&self, total_files: u64) -> ProgressBar {
        let pb = self.multi_progress.add(ProgressBar::new(total_files));
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({percent}%) ETA: {eta}",
            )
            .expect("valid progress template")
            .progress_chars("█▉▊▋▌▍▎▏  "),
        );
        pb.set_message("Batch processing");
        pb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_config_default() {
        let config = ProgressConfig::default();
        assert!(config.show_speed);
        assert!(config.show_eta);
        assert!(config.show_percentage);
        assert!(config.template.contains("{bar:40.cyan/blue}"));
    }

    #[test]
    fn test_progress_tracker_creation() {
        let tracker = ProgressTracker::new(ProgressConfig::default());
        assert!(Arc::strong_count(&tracker.multi_progress) >= 1);
    }
}
