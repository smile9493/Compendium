//! # Upload File Storage
//!
//! Thread-safe temporary storage for uploaded PDF files.
//! Maps `file_id` (UUID) → `(temp_file_path, original_filename)`.
//! Files are stored in a temp directory created at server startup.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use uuid::Uuid;

/// Thread-safe store for uploaded file references.
pub struct UploadStore {
    inner: Mutex<HashMap<String, UploadedFile>>,
    temp_dir: PathBuf,
}

/// Metadata for a single uploaded file.
#[derive(Debug, Clone)]
pub struct UploadedFile {
    /// Absolute path to the temp file on disk.
    pub temp_path: PathBuf,
    /// Original filename from the upload.
    pub filename: String,
    /// File size in bytes.
    pub size: u64,
}

impl UploadStore {
    /// Create a new upload store with a temp directory.
    pub fn new() -> Result<Self> {
        let temp_dir =
            std::env::temp_dir().join(format!("rsut-pdf-uploads-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir)
            .with_context(|| format!("Failed to create upload temp dir: {}", temp_dir.display()))?;
        Ok(Self { inner: Mutex::new(HashMap::new()), temp_dir })
    }

    /// Store an uploaded file and return its `file_id`.
    pub fn store(&self, data: &[u8], filename: &str) -> Result<String> {
        let file_id = Uuid::new_v4().to_string();
        let temp_path = self.temp_dir.join(&file_id);

        std::fs::write(&temp_path, data)
            .with_context(|| format!("Failed to write upload to: {}", temp_path.display()))?;

        let file =
            UploadedFile { temp_path, filename: filename.to_string(), size: data.len() as u64 };

        self.inner.lock().expect("UploadStore mutex poisoned").insert(file_id.clone(), file);

        Ok(file_id)
    }

    /// Retrieve a stored file by `file_id`.
    pub fn get(&self, file_id: &str) -> Option<UploadedFile> {
        self.inner.lock().expect("UploadStore mutex poisoned").get(file_id).cloned()
    }

    /// Remove and return a stored file (for cleanup after processing).
    pub fn take(&self, file_id: &str) -> Option<UploadedFile> {
        self.inner.lock().expect("UploadStore mutex poisoned").remove(file_id)
    }

    /// Remove a stored file and delete its temp file from disk.
    pub fn remove(&self, file_id: &str) {
        if let Some(file) = self.take(file_id) {
            let _ = std::fs::remove_file(&file.temp_path);
        }
    }

    /// Return the temp directory path.
    pub fn temp_dir(&self) -> &Path {
        &self.temp_dir
    }

    /// Clean up all stored files and remove the temp directory.
    pub fn cleanup(&self) {
        let files: Vec<String> =
            self.inner.lock().expect("UploadStore mutex poisoned").keys().cloned().collect();
        for id in files {
            self.remove(&id);
        }
        let _ = std::fs::remove_dir_all(&self.temp_dir);
    }

    /// Current number of stored files.
    pub fn count(&self) -> usize {
        self.inner.lock().expect("UploadStore mutex poisoned").len()
    }
}

impl Drop for UploadStore {
    fn drop(&mut self) {
        self.cleanup();
    }
}
