//! # CLI Configuration
//!
//! Manages `~/.rsut-pdf/config.toml` for persistent settings:
//! default mode, remote server URL, auth token, and knowledge base path.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Default filename for CLI configuration
const CONFIG_DIR: &str = ".rsut-pdf";
const CONFIG_FILE: &str = "config.toml";

/// Top-level CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// Default operating mode: "local" or "remote"
    #[serde(default = "default_mode")]
    pub mode: String,

    /// Remote server URL (e.g. "http://192.168.2.50:9090")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,

    /// Auth token for remote server
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,

    /// Default knowledge base path (local mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knowledge_base: Option<PathBuf>,

    /// Remote knowledge base path override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_knowledge_base: Option<String>,
}

fn default_mode() -> String {
    "local".to_string()
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            server: None,
            token: None,
            knowledge_base: None,
            remote_knowledge_base: None,
        }
    }
}

impl CliConfig {
    /// Resolve the config directory path (~/.rsut-pdf)
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot determine home directory")?;
        Ok(home.join(CONFIG_DIR))
    }

    /// Resolve the config file path (~/.rsut-pdf/config.toml)
    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join(CONFIG_FILE))
    }

    /// Load config from disk, or return defaults if file doesn't exist
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse config: {}", path.display()))
    }

    /// Save config to disk atomically
    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir()?;
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create config dir: {}", dir.display()))?;

        let path = dir.join(CONFIG_FILE);
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        // Atomic write: tmp + rename
        let tmp_path = path.with_extension("toml.tmp");
        std::fs::write(&tmp_path, &content)
            .with_context(|| format!("Failed to write config: {}", tmp_path.display()))?;
        std::fs::rename(&tmp_path, &path)
            .with_context(|| format!("Failed to rename config: {}", path.display()))?;
        Ok(())
    }

    /// Ensure the CLI config directory exists
    #[allow(dead_code)]
    pub fn ensure_dir() -> Result<PathBuf> {
        let dir = Self::config_dir()?;
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create config dir: {}", dir.display()))?;
        Ok(dir)
    }
}

/// Supported config keys for `config set/get`
#[allow(dead_code)]
pub enum ConfigKey {
    Mode,
    Server,
    Token,
    KnowledgeBase,
    RemoteKnowledgeBase,
}

impl std::str::FromStr for ConfigKey {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "mode" => Ok(Self::Mode),
            "server" => Ok(Self::Server),
            "token" => Ok(Self::Token),
            "knowledge_base" | "kb" => Ok(Self::KnowledgeBase),
            "remote_knowledge_base" | "remote_kb" => Ok(Self::RemoteKnowledgeBase),
            _ => Err(anyhow::anyhow!(
                "Unknown config key: {}. Valid keys: mode, server, token, knowledge_base, remote_knowledge_base",
                s
            )),
        }
    }
}

impl ConfigKey {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mode => "mode",
            Self::Server => "server",
            Self::Token => "token",
            Self::KnowledgeBase => "knowledge_base",
            Self::RemoteKnowledgeBase => "remote_knowledge_base",
        }
    }
}
