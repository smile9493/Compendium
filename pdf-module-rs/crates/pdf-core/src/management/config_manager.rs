//! Configuration manager with atomic persistence.
//!
//! Loads runtime configuration from a JSON file (or falls back to defaults),
//! supports querying and setting individual keys, and writes changes atomically
//! via write-to-temp + rename.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{PdfModuleError, PdfResult};

const CONFIG_FILENAME: &str = "config.json";

/// Managed runtime configuration.
///
/// Wraps a flat key-value map that is persisted as JSON.
/// For structured configuration, use `ServerConfig` directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigManager {
    #[serde(skip)]
    config_path: PathBuf,
    data: HashMap<String, String>,
}

impl ConfigManager {
    /// Create a new `ConfigManager` bound to a knowledge base directory.
    /// The config file lives at `<kb_path>/.rsut_index/config.json`.
    pub fn new(kb_path: &Path) -> Self {
        let config_path = kb_path.join(".rsut_index").join(CONFIG_FILENAME);
        Self {
            config_path,
            data: HashMap::new(),
        }
    }

    /// Load configuration from disk. Returns an empty config if the file does not exist.
    pub fn load(&mut self) -> PdfResult<()> {
        if !self.config_path.exists() {
            return Ok(());
        }
        let content = fs::read_to_string(&self.config_path)
            .map_err(|e| PdfModuleError::Config(format!("Failed to read config file: {}", e)))?;
        self.data = serde_json::from_str(&content)
            .map_err(|e| PdfModuleError::Config(format!("Failed to parse config JSON: {}", e)))?;
        Ok(())
    }

    /// Get a configuration value by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }

    /// Set a configuration value. Persists atomically.
    pub fn set(&mut self, key: &str, value: &str) -> PdfResult<()> {
        self.data.insert(key.to_string(), value.to_string());
        self.save()
    }

    /// Remove a configuration value. Persists atomically.
    pub fn remove(&mut self, key: &str) -> PdfResult<()> {
        self.data.remove(key);
        self.save()
    }

    /// Return all configuration entries as a flat map.
    pub fn all(&self) -> &HashMap<String, String> {
        &self.data
    }

    /// Persist the current state to disk using atomic write (tmp + rename).
    pub fn save(&self) -> PdfResult<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                PdfModuleError::Config(format!("Failed to create config dir: {}", e))
            })?;
        }
        let json = serde_json::to_string_pretty(&self.data)
            .map_err(|e| PdfModuleError::Config(format!("Failed to serialize config: {}", e)))?;
        let tmp_path = self.config_path.with_extension("json.tmp");
        fs::write(&tmp_path, &json).map_err(|e| {
            PdfModuleError::Config(format!("Failed to write config tmp file: {}", e))
        })?;
        fs::rename(&tmp_path, &self.config_path).map_err(|e| {
            PdfModuleError::Config(format!("Failed to rename config tmp file: {}", e))
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_set_and_get() {
        let dir = TempDir::new().expect("tmpdir");
        let mut cm = ConfigManager::new(dir.path());
        cm.set("vlm_api_key", "sk-test").expect("set");
        assert_eq!(cm.get("vlm_api_key"), Some("sk-test"));
    }

    #[test]
    fn test_persistence() {
        let dir = TempDir::new().expect("tmpdir");
        {
            let mut cm = ConfigManager::new(dir.path());
            cm.set("key1", "val1").expect("set");
        }
        let mut cm2 = ConfigManager::new(dir.path());
        cm2.load().expect("load");
        assert_eq!(cm2.get("key1"), Some("val1"));
    }

    #[test]
    fn test_remove() {
        let dir = TempDir::new().expect("tmpdir");
        let mut cm = ConfigManager::new(dir.path());
        cm.set("k", "v").expect("set");
        cm.remove("k").expect("remove");
        assert_eq!(cm.get("k"), None);
    }
}
