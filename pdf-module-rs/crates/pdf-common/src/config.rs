//! Unified configuration management.
//!
//! Consolidates configuration types from pdf-core and pdf-etl.

use crate::PdfError;
use serde::{Deserialize, Serialize, Serializer};

/// Serialize an optional secret string as `"REDACTED"` (or omit if None).
fn redact_secret<S: Serializer>(value: &Option<String>, serializer: S) -> Result<S::Ok, S::Error> {
    match value {
        Some(_) => serializer.serialize_some("REDACTED"),
        None => serializer.serialize_none(),
    }
}

/// Serialize a secret string as `"REDACTED"`.
fn redact_string<S: Serializer>(value: &str, serializer: S) -> Result<S::Ok, S::Error> {
    if value.is_empty() {
        serializer.serialize_some("")
    } else {
        serializer.serialize_some("REDACTED")
    }
}

/// Application environment
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    #[default]
    Development,
    Staging,
    Production,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether caching is enabled
    pub enabled: bool,
    /// Maximum number of cached entries
    pub max_size: usize,
    /// Time-to-live in seconds
    pub ttl_seconds: u64,
    /// Optional cache directory path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<String>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size: 1000,
            ttl_seconds: 3600,
            cache_dir: None,
        }
    }
}

impl CacheConfig {
    /// Validate the cache configuration
    pub fn validate(&self) -> crate::Result<()> {
        if self.enabled && self.max_size == 0 {
            return Err(PdfError::Config(
                "cache.max_size must be > 0 when cache is enabled".into(),
            ));
        }
        if self.enabled && self.ttl_seconds == 0 {
            return Err(PdfError::Config(
                "cache.ttl_seconds must be > 0 when cache is enabled".into(),
            ));
        }
        Ok(())
    }
}

/// Audit backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuditBackendConfig {
    File {
        log_dir: String,
    },
    Database {
        #[serde(serialize_with = "redact_string")]
        connection_string: String,
        table_name: String,
    },
    Remote {
        endpoint: String,
        #[serde(serialize_with = "redact_string")]
        api_key: String,
    },
    Memory,
}

impl Default for AuditBackendConfig {
    fn default() -> Self {
        Self::File {
            log_dir: "./logs/audit".to_string(),
        }
    }
}

/// Audit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub enabled: bool,
    pub backend: AuditBackendConfig,
    pub retention_days: u32,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: AuditBackendConfig::default(),
            retention_days: 30,
        }
    }
}

impl AuditConfig {
    /// Validate the audit configuration
    pub fn validate(&self) -> crate::Result<()> {
        if self.enabled && self.retention_days == 0 {
            return Err(PdfError::Config(
                "audit.retention_days must be > 0 when audit is enabled".into(),
            ));
        }
        Ok(())
    }
}

/// Log format
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    #[default]
    Json,
    Text,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (debug, info, warn, error)
    pub level: String,
    /// Log output format
    pub format: LogFormat,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Json,
        }
    }
}

/// Path validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathValidationConfig {
    /// Whether to require absolute paths
    pub require_absolute: bool,
    /// Whether to allow path traversal (..)
    pub allow_traversal: bool,
    /// Optional base directory for relative paths
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_dir: Option<std::path::PathBuf>,
}

impl Default for PathValidationConfig {
    fn default() -> Self {
        Self {
            require_absolute: true,
            allow_traversal: false,
            base_dir: None,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Path validation rules
    pub path_validation: PathValidationConfig,
    /// Maximum file size in megabytes
    pub max_file_size_mb: u64,
    /// Allowed file extensions
    pub allowed_file_types: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            path_validation: PathValidationConfig::default(),
            max_file_size_mb: 100,
            allowed_file_types: vec!["pdf".to_string()],
        }
    }
}

/// Storage backend type
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    #[default]
    Local,
    S3,
    Gcs,
    AzureBlob,
}

/// Local storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStorageConfig {
    pub base_dir: String,
}

/// S3 storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3StorageConfig {
    pub bucket: String,
    pub region: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "redact_secret"
    )]
    pub access_key: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "redact_secret"
    )]
    pub secret_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}

/// GCS storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GCSStorageConfig {
    pub bucket: String,
    pub credentials_path: String,
}

/// Azure Blob storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureStorageConfig {
    pub account: String,
    pub key: String,
    pub container: String,
}

/// File storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStorageConfig {
    pub storage_type: StorageType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local: Option<LocalStorageConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3: Option<S3StorageConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcs: Option<GCSStorageConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure: Option<AzureStorageConfig>,
}

impl Default for FileStorageConfig {
    fn default() -> Self {
        Self {
            storage_type: StorageType::Local,
            local: Some(LocalStorageConfig {
                base_dir: "./data".to_string(),
            }),
            s3: None,
            gcs: None,
            azure: None,
        }
    }
}

impl FileStorageConfig {
    /// Validate storage configuration
    pub fn validate(&self) -> crate::Result<()> {
        match self.storage_type {
            StorageType::Local if self.local.is_none() => Err(PdfError::Config(
                "Local storage config is required for local storage type".into(),
            )),
            StorageType::S3 if self.s3.is_none() => Err(PdfError::Config(
                "S3 storage config is required for S3 storage type".into(),
            )),
            _ => Ok(()),
        }
    }
}

/// Unified application configuration.
///
/// Consolidates `pdf-core::config::ServerConfig` and sub-configs into a
/// single top-level configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Server name
    pub server_name: String,
    /// Server version
    pub server_version: String,
    /// Runtime environment
    pub environment: Environment,
    /// Storage configuration
    pub storage: FileStorageConfig,
    /// Cache configuration
    pub cache: CacheConfig,
    /// Audit configuration
    pub audit: AuditConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Security configuration
    pub security: SecurityConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_name: "pdf-module".to_string(),
            server_version: "0.3.0".to_string(),
            environment: Environment::default(),
            storage: FileStorageConfig::default(),
            cache: CacheConfig::default(),
            audit: AuditConfig::default(),
            logging: LoggingConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl AppConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> crate::Result<Self> {
        let environment = match std::env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            .as_str()
        {
            "production" => Environment::Production,
            "staging" => Environment::Staging,
            _ => Environment::Development,
        };

        let config = Self {
            server_name: std::env::var("SERVER_NAME").unwrap_or_else(|_| "pdf-module".to_string()),
            server_version: std::env::var("SERVER_VERSION").unwrap_or_else(|_| "0.3.0".to_string()),
            environment,
            storage: FileStorageConfig::default(),
            cache: CacheConfig {
                enabled: std::env::var("CACHE_ENABLED")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                max_size: std::env::var("CACHE_MAX_SIZE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1000),
                ttl_seconds: std::env::var("CACHE_TTL_SECONDS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(3600),
                cache_dir: std::env::var("CACHE_DIR").ok(),
            },
            audit: AuditConfig::default(),
            logging: LoggingConfig {
                level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
                format: match std::env::var("LOG_FORMAT")
                    .unwrap_or_else(|_| "json".to_string())
                    .to_lowercase()
                    .as_str()
                {
                    "text" => LogFormat::Text,
                    _ => LogFormat::Json,
                },
            },
            security: SecurityConfig {
                max_file_size_mb: std::env::var("MAX_FILE_SIZE_MB")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(100),
                ..Default::default()
            },
        };
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from a TOML file
    pub fn from_file(path: &str) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| PdfError::Config(format!("Failed to read config file: {}", e)))?;

        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let config: Self = match ext.to_lowercase().as_str() {
            "toml" => toml::from_str(&content)
                .map_err(|e| PdfError::Config(format!("Failed to parse TOML: {}", e))),
            "json" => serde_json::from_str(&content)
                .map_err(|e| PdfError::Config(format!("Failed to parse JSON: {}", e))),
            _ => Err(PdfError::Config(format!(
                "Unsupported config format: {}",
                ext
            ))),
        }?;
        config.validate()?;
        Ok(config)
    }

    /// Validate all configuration sections
    pub fn validate(&self) -> crate::Result<()> {
        self.storage.validate()?;
        self.cache.validate()?;
        self.audit.validate()?;
        Ok(())
    }

    /// Get max file size in bytes
    pub fn max_file_size_bytes(&self) -> u64 {
        self.security.max_file_size_mb * 1024 * 1024
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server_name, "pdf-module");
        assert_eq!(config.environment, Environment::Development);
        assert!(config.cache.enabled);
        assert_eq!(config.security.max_file_size_mb, 100);
    }

    #[test]
    fn test_config_validation() {
        let mut config = AppConfig::default();
        assert!(config.validate().is_ok());

        config.cache.enabled = true;
        config.cache.max_size = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_max_file_size_bytes() {
        let config = AppConfig {
            security: SecurityConfig {
                max_file_size_mb: 200,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(config.max_file_size_bytes(), 200 * 1024 * 1024);
    }
}
