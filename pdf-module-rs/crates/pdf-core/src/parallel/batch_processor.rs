//! Batch processor for parallel PDF extraction.
//!
//! Uses `tokio::task::JoinSet` for concurrent file processing,
//! providing better task lifecycle management and cancellation support.

use crate::dto::{ExtractOptions, StructuredExtractionResult};
use crate::error::{PdfModuleError, PdfResult};
use crate::extractor::McpPdfPipeline;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::task::JoinSet;

/// Errors that can occur during batch processing.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BatchError {
    #[error("Extraction failed: {0}")]
    Extraction(#[from] PdfModuleError),

    #[error("Task join failed: {0}")]
    JoinError(#[from] tokio::task::JoinError),
}

/// Configuration for batch PDF processing.
///
/// # Example
///
/// ```no_run
/// use pdf_core::parallel::BatchConfig;
///
/// let config = BatchConfig {
///     max_files_parallel: 4,
///     max_pages_parallel: 8,
///     chunk_size: 20,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum number of files to process in parallel.
    /// Default: number of CPU cores.
    pub max_files_parallel: usize,

    /// Maximum number of pages to extract in parallel per file.
    /// Default: 4.
    pub max_pages_parallel: usize,

    /// Number of files to process in each chunk for progress reporting.
    /// Default: 10.
    pub chunk_size: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_files_parallel: num_cpus::get(),
            max_pages_parallel: 4,
            chunk_size: 10,
        }
    }
}

/// Batch processor for parallel PDF extraction.
///
/// Uses `tokio::task::JoinSet` for concurrent file processing,
/// providing better task lifecycle management and cancellation support.
///
/// # Features
///
/// - **Parallel processing**: Process multiple PDF files concurrently
/// - **Backpressure**: Limit concurrent tasks to prevent resource exhaustion
/// - **Progress tracking**: Optional callback for progress reporting
/// - **Error isolation**: Failed extractions don't affect other files
///
/// # Example
///
/// ```ignore
/// use pdf_core::{McpPdfPipeline, ServerConfig, parallel::{BatchProcessor, BatchConfig}};
/// use pdf_core::dto::ExtractOptions;
/// use std::sync::Arc;
///
/// let config = ServerConfig::from_env()?;
/// let pipeline = Arc::new(McpPdfPipeline::new(&config)?);
/// let processor = BatchProcessor::new(pipeline, BatchConfig::default());
///
/// let files = vec!["doc1.pdf".into(), "doc2.pdf".into()];
/// let options = ExtractOptions::default();
/// let results = processor.process_batch_async(files, options).await?;
///
/// for (path, result) in results {
///     match result {
///         Ok(extraction) => println!("{}: {} pages", path.display(), extraction.pages.len()),
///         Err(e) => eprintln!("{}: {}", path.display(), e),
///     }
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct BatchProcessor {
    pipeline: Arc<McpPdfPipeline>,
    config: BatchConfig,
}

impl BatchProcessor {
    pub fn new(pipeline: Arc<McpPdfPipeline>, config: BatchConfig) -> Self {
        Self { pipeline, config }
    }

    /// Process multiple PDF files in parallel using async approach.
    ///
    /// Uses `JoinSet` to process files concurrently up to the
    /// configured parallelism limit.
    #[tracing::instrument(skip(self, files, options))]
    pub async fn process_batch_async(
        &self,
        files: Vec<PathBuf>,
        options: ExtractOptions,
    ) -> Result<Vec<(PathBuf, PdfResult<StructuredExtractionResult>)>, BatchError> {
        let pipeline = Arc::clone(&self.pipeline);
        let max_parallel = self.config.max_files_parallel;

        let mut set = JoinSet::new();
        let mut results = Vec::with_capacity(files.len());

        for file_path in files {
            let pipeline = Arc::clone(&pipeline);
            let options = options.clone();

            set.spawn(async move {
                let result = pipeline.extract_structured(&file_path, &options).await;
                (file_path, result)
            });

            if set.len() >= max_parallel {
                if let Some(res) = set.join_next().await {
                    results.push(res?);
                }
            }
        }

        while let Some(res) = set.join_next().await {
            results.push(res?);
        }

        Ok(results)
    }

    /// Process batch with progress callback.
    ///
    /// # Arguments
    ///
    /// * `files` - List of PDF file paths to process
    /// * `options` - Extraction options
    /// * `progress_callback` - Called with (completed_count, total_count) after each file
    #[tracing::instrument(skip(self, files, options, progress_callback))]
    pub async fn process_batch_with_progress<F>(
        &self,
        files: Vec<PathBuf>,
        options: ExtractOptions,
        mut progress_callback: F,
    ) -> Result<Vec<(PathBuf, PdfResult<StructuredExtractionResult>)>, BatchError>
    where
        F: FnMut(usize, usize) + Send,
    {
        let pipeline = Arc::clone(&self.pipeline);
        let total = files.len();
        let max_parallel = self.config.max_files_parallel;
        let processed = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let mut set = JoinSet::new();
        let mut results = Vec::with_capacity(files.len());

        for file_path in files {
            let pipeline = Arc::clone(&pipeline);
            let options = options.clone();
            let processed = Arc::clone(&processed);

            set.spawn(async move {
                let result = pipeline.extract_structured(&file_path, &options).await;
                let count = processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                (file_path, result, count + 1)
            });

            if set.len() >= max_parallel {
                if let Some(res) = set.join_next().await {
                    let (file_path, result, count) = res?;
                    progress_callback(count, total);
                    results.push((file_path, result));
                }
            }
        }

        while let Some(res) = set.join_next().await {
            let (file_path, result, count) = res?;
            progress_callback(count, total);
            results.push((file_path, result));
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.max_files_parallel, num_cpus::get());
        assert_eq!(config.max_pages_parallel, 4);
        assert_eq!(config.chunk_size, 10);
    }

    #[test]
    fn test_batch_config_custom() {
        let config = BatchConfig {
            max_files_parallel: 2,
            max_pages_parallel: 1,
            chunk_size: 5,
        };
        assert_eq!(config.max_files_parallel, 2);
        assert_eq!(config.max_pages_parallel, 1);
        assert_eq!(config.chunk_size, 5);
    }

    #[tokio::test]
    async fn test_batch_processor_empty_input() {
        use crate::config::ServerConfig;

        let config = ServerConfig::default();
        let pipeline = Arc::new(McpPdfPipeline::new(&config).unwrap());
        let batch_config = BatchConfig::default();
        let processor = BatchProcessor::new(pipeline, batch_config);

        let result = processor
            .process_batch_async(vec![], ExtractOptions::default())
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_batch_processor_with_progress_empty() {
        use crate::config::ServerConfig;

        let config = ServerConfig::default();
        let pipeline = Arc::new(McpPdfPipeline::new(&config).unwrap());
        let batch_config = BatchConfig::default();
        let processor = BatchProcessor::new(pipeline, batch_config);

        let mut call_count = 0;
        let result = processor
            .process_batch_with_progress(vec![], ExtractOptions::default(), |_done, _total| {
                call_count += 1;
            })
            .await;
        assert!(result.is_ok());
        assert_eq!(call_count, 0);
    }
}
