//! Configuration management for PDF module
//! Corresponds to Python: config.py

use crate::dto::{
    AzureStorageConfig, Environment, GCSStorageConfig, LocalStorageConfig, LogFormat, LogOutput,
    S3StorageConfig, StorageType,
};
use crate::error::PdfModuleError;
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

// === Configuration Structures ===

/// Base configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseConfig {
    pub server_name: String,
    pub server_version: String,
    pub environment: Environment,
}

impl Default for BaseConfig {
    fn default() -> Self {
        Self {
            server_name: "pdf-module-mcp".to_string(),
            server_version: "0.2.0".to_string(),
            environment: Environment::Development,
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub max_size: usize,
    pub ttl_seconds: u64,
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

/// Audit backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuditBackendConfig {
    File {
        log_dir: String,
    },
    Database {
        connection_string: String,
        table_name: String,
    },
    Remote {
        endpoint: String,
        api_key: String,
    },
    Memory,
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
            backend: AuditBackendConfig::File {
                log_dir: "./logs/audit".to_string(),
            },
            retention_days: 30,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub outputs: Vec<LogOutput>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Json,
            outputs: vec![LogOutput::Stdout],
        }
    }
}

/// Path validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathValidationConfig {
    pub require_absolute: bool,
    pub allow_traversal: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_dir: Option<String>,
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
    pub path_validation: PathValidationConfig,
    pub max_file_size_mb: u64,
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

/// Complete server configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerConfig {
    /// Base configuration
    pub base: BaseConfig,

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

impl ServerConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, PdfModuleError> {
        let environment = match std::env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            .as_str()
        {
            "development" => Environment::Development,
            "staging" => Environment::Staging,
            "production" => Environment::Production,
            _ => Environment::Development,
        };

        let storage_type = match std::env::var("STORAGE_TYPE")
            .unwrap_or_else(|_| "local".to_string())
            .to_lowercase()
            .as_str()
        {
            "local" => StorageType::Local,
            "s3" => StorageType::S3,
            "gcs" => StorageType::Gcs,
            "azure" => StorageType::AzureBlob,
            _ => StorageType::Local,
        };

        let log_format = match std::env::var("LOG_FORMAT")
            .unwrap_or_else(|_| "json".to_string())
            .to_lowercase()
            .as_str()
        {
            "json" => LogFormat::Json,
            "text" => LogFormat::Text,
            _ => LogFormat::Json,
        };

        Ok(Self {
            base: BaseConfig {
                server_name: std::env::var("SERVER_NAME")
                    .unwrap_or_else(|_| "pdf-module-mcp".to_string()),
                server_version: std::env::var("SERVER_VERSION")
                    .unwrap_or_else(|_| "0.2.0".to_string()),
                environment,
            },
            storage: FileStorageConfig {
                storage_type: storage_type.clone(),
                local: if storage_type == StorageType::Local {
                    Some(LocalStorageConfig {
                        base_dir: std::env::var("LOCAL_STORAGE_BASE_DIR")
                            .unwrap_or_else(|_| "./data".to_string()),
                    })
                } else {
                    None
                },
                s3: if storage_type == StorageType::S3 {
                    Some(S3StorageConfig {
                        bucket: std::env::var("S3_BUCKET").unwrap_or_else(|_| "".to_string()),
                        region: std::env::var("S3_REGION")
                            .unwrap_or_else(|_| "us-east-1".to_string()),
                        prefix: std::env::var("S3_PREFIX").ok(),
                        access_key: std::env::var("S3_ACCESS_KEY").ok(),
                        secret_key: std::env::var("S3_SECRET_KEY").ok(),
                        endpoint: std::env::var("S3_ENDPOINT").ok(),
                    })
                } else {
                    None
                },
                gcs: if storage_type == StorageType::Gcs {
                    Some(GCSStorageConfig {
                        bucket: std::env::var("GCS_BUCKET").unwrap_or_else(|_| "".to_string()),
                        credentials_path: std::env::var("GCS_CREDENTIALS_PATH")
                            .unwrap_or_else(|_| "".to_string()),
                    })
                } else {
                    None
                },
                azure: if storage_type == StorageType::AzureBlob {
                    Some(AzureStorageConfig {
                        account: std::env::var("AZURE_STORAGE_ACCOUNT")
                            .unwrap_or_else(|_| "".to_string()),
                        key: std::env::var("AZURE_STORAGE_KEY").unwrap_or_else(|_| "".to_string()),
                        container: std::env::var("AZURE_CONTAINER")
                            .unwrap_or_else(|_| "".to_string()),
                    })
                } else {
                    None
                },
            },
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
            audit: AuditConfig {
                enabled: std::env::var("AUDIT_ENABLED")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                backend: match std::env::var("AUDIT_BACKEND")
                    .unwrap_or_else(|_| "file".to_string())
                    .to_lowercase()
                    .as_str()
                {
                    "database" => AuditBackendConfig::Database {
                        connection_string: std::env::var("AUDIT_DB_CONNECTION_STRING")
                            .unwrap_or_else(|_| "".to_string()),
                        table_name: std::env::var("AUDIT_DB_TABLE_NAME")
                            .unwrap_or_else(|_| "audit_logs".to_string()),
                    },
                    "remote" => AuditBackendConfig::Remote {
                        endpoint: std::env::var("AUDIT_REMOTE_ENDPOINT")
                            .unwrap_or_else(|_| "".to_string()),
                        api_key: std::env::var("AUDIT_REMOTE_API_KEY")
                            .unwrap_or_else(|_| "".to_string()),
                    },
                    "memory" => AuditBackendConfig::Memory,
                    _ => AuditBackendConfig::File {
                        log_dir: std::env::var("AUDIT_LOG_DIR")
                            .unwrap_or_else(|_| "./logs/audit".to_string()),
                    },
                },
                retention_days: std::env::var("AUDIT_RETENTION_DAYS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30),
            },
            logging: LoggingConfig {
                level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
                format: log_format,
                outputs: vec![LogOutput::Stdout],
            },
            security: SecurityConfig {
                path_validation: PathValidationConfig {
                    require_absolute: std::env::var("PATH_REQUIRE_ABSOLUTE")
                        .unwrap_or_else(|_| "true".to_string())
                        .parse()
                        .unwrap_or(true),
                    allow_traversal: std::env::var("PATH_ALLOW_TRAVERSAL")
                        .unwrap_or_else(|_| "false".to_string())
                        .parse()
                        .unwrap_or(false),
                    base_dir: std::env::var("PATH_BASE_DIR").ok(),
                },
                max_file_size_mb: std::env::var("MAX_FILE_SIZE_MB")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(100),
                allowed_file_types: vec!["pdf".to_string()],
            },
        })
    }

    /// Load configuration from a file (TOML, JSON, or YAML)
    pub fn from_file(path: &str) -> Result<Self, PdfModuleError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| PdfModuleError::Config(format!("Failed to read config file: {}", e)))?;

        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let config: Self = match ext.to_lowercase().as_str() {
            "toml" => toml::from_str(&content)
                .map_err(|e| PdfModuleError::Config(format!("Failed to parse TOML: {}", e)))?,
            "json" => serde_json::from_str(&content)
                .map_err(|e| PdfModuleError::Config(format!("Failed to parse JSON: {}", e)))?,
            "yaml" | "yml" => serde_yaml::from_str(&content)
                .map_err(|e| PdfModuleError::Config(format!("Failed to parse YAML: {}", e)))?,
            _ => {
                return Err(PdfModuleError::Config(format!(
                    "Unsupported config format: {}",
                    ext
                )))
            }
        };

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), PdfModuleError> {
        // Validate storage configuration
        match self.storage.storage_type {
            StorageType::Local if self.storage.local.is_none() => {
                return Err(PdfModuleError::Config(
                    "Local storage config is required for local storage type".to_string(),
                ));
            }
            StorageType::S3 if self.storage.s3.is_none() => {
                return Err(PdfModuleError::Config(
                    "S3 storage config is required for S3 storage type".to_string(),
                ));
            }
            _ => {}
        }

        // Validate cache configuration
        if self.cache.enabled && self.cache.max_size == 0 {
            return Err(PdfModuleError::Config(
                "Cache max size must be greater than 0 when cache is enabled".to_string(),
            ));
        }

        Ok(())
    }

    /// Initialize tracing/logging
    pub fn init_tracing(&self) {
        let level = match self.logging.level.to_lowercase().as_str() {
            "debug" => tracing::Level::DEBUG,
            "info" => tracing::Level::INFO,
            "warn" => tracing::Level::WARN,
            "error" => tracing::Level::ERROR,
            _ => tracing::Level::INFO,
        };

        let _env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level.as_str()));

        let subscriber = tracing_subscriber::fmt()
            .with_max_level(level)
            .with_target(true)
            .with_thread_ids(true);

        match self.logging.format {
            LogFormat::Json => {
                subscriber.json().init();
            }
            LogFormat::Text => {
                subscriber.pretty().init();
            }
        }
    }

    /// Get max file size in bytes
    pub fn max_file_size_bytes(&self) -> u64 {
        self.security.max_file_size_mb * 1024 * 1024
    }
}

// === Legacy Configuration for Backward Compatibility ===

/// Legacy server configuration (for backward compatibility)
#[deprecated(note = "Use ServerConfig instead")]
#[derive(Debug, Clone, Deserialize)]
pub struct LegacyServerConfig {
    pub host: String,
    pub port: u16,
    pub debug: bool,
    pub api_url_prefix: String,
    pub log_level: String,
    pub log_file: Option<String>,
    pub max_file_size_mb: u32,
    pub default_adapter: String,
    pub cache_enabled: bool,
    pub cache_max_size: usize,
    pub cache_ttl_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.base.server_name, "pdf-module-mcp");
        assert_eq!(config.storage.storage_type, StorageType::Local);
        assert!(config.cache.enabled);
        assert!(config.audit.enabled);
    }

    #[test]
    fn test_config_validation() {
        let mut config = ServerConfig::default();

        // Valid config
        assert!(config.validate().is_ok());

        // Invalid cache config
        config.cache.enabled = true;
        config.cache.max_size = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_max_file_size_bytes() {
        let config = ServerConfig {
            security: SecurityConfig {
                max_file_size_mb: 100,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(config.max_file_size_bytes(), 100 * 1024 * 1024);
    }
}
