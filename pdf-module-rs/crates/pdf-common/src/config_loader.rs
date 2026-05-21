//! Hierarchical configuration loader using `figment`.
//!
//! Provides layered configuration loading with priority:
//!   CLI args > Environment variables > config.toml > defaults
//!
//! # Feature flag
//!
//! Enable with `features = ["config-loader"]` in `Cargo.toml`.
//!
//! # Usage
//!
//! ```ignore
//! use pdf_common::config_loader::{load_config, AppSettings};
//!
//! let settings: AppSettings = load_config(None)?;
//! ```

use figment::Figment;
use figment::providers::{Env, Format, Serialized, Toml};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub server: ServerSettings,
    pub database: DatabaseSettings,
    pub logging: LoggingSettings,
    pub security: SecuritySettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            server: ServerSettings::default(),
            database: DatabaseSettings::default(),
            logging: LoggingSettings::default(),
            security: SecuritySettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_workers")]
    pub workers: usize,
}

fn default_host() -> String {
    "0.0.0.0".into()
}
fn default_port() -> u16 {
    8000
}
fn default_workers() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self { host: default_host(), port: default_port(), workers: default_workers() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSettings {
    #[serde(default = "default_db_url")]
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default)]
    pub migrate_on_start: bool,
}

fn default_db_url() -> String {
    "sqlite:./data/app.db?mode=rwc".into()
}
fn default_max_connections() -> u32 {
    5
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            url: default_db_url(),
            max_connections: default_max_connections(),
            migrate_on_start: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSettings {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub json: bool,
}

fn default_log_level() -> String {
    "info".into()
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self { level: default_log_level(), json: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
    #[serde(default = "default_jwt_expiry_hours")]
    pub jwt_expiry_hours: u64,
    #[serde(default)]
    pub allowed_origins: Vec<String>,
}

fn default_jwt_secret() -> String {
    "change-me-in-production".into()
}
fn default_jwt_expiry_hours() -> u64 {
    24
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            jwt_secret: default_jwt_secret(),
            jwt_expiry_hours: default_jwt_expiry_hours(),
            allowed_origins: vec![],
        }
    }
}

/// Load settings from layered sources.
///
/// Priority (highest to lowest):
/// 1. Environment variables prefixed with `APP_` (e.g. `APP_SERVER__PORT=9090`)
/// 2. `config.toml` (or the file at `config_path`)
/// 3. `config.local.toml` (git-ignored overrides)
/// 4. Rust `Default` impl
pub fn load_config<T: DeserializeOwned + Default>(
    config_path: Option<PathBuf>,
) -> Result<T, figment::Error> {
    let config_file = config_path.unwrap_or_else(|| PathBuf::from("config.toml"));

    Figment::new()
        .merge(Serialized::defaults(T::default()))
        .merge(Toml::file(&config_file))
        .merge(Toml::file("config.local.toml"))
        .merge(Env::prefixed("APP_").split("__"))
        .extract()
}

/// Load settings with an explicit profile name.
///
/// After loading defaults and config.toml, applies
/// `config.{profile}.toml` overrides, then env vars.
pub fn load_config_with_profile<T: DeserializeOwned + Default>(
    config_path: Option<PathBuf>,
    profile: &str,
) -> Result<T, figment::Error> {
    let config_file = config_path.unwrap_or_else(|| PathBuf::from("config.toml"));
    let profile_file = format!("config.{}.toml", profile);

    Figment::new()
        .merge(Serialized::defaults(T::default()))
        .merge(Toml::file(&config_file))
        .merge(Toml::file(&profile_file))
        .merge(Env::prefixed("APP_").split("__"))
        .extract()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_server_binds_all_interfaces() {
        let s = ServerSettings::default();
        assert_eq!(s.host, "0.0.0.0");
    }

    #[test]
    fn default_database_is_sqlite() {
        let s = DatabaseSettings::default();
        assert!(s.url.starts_with("sqlite:"));
    }

    #[test]
    fn default_settings_is_serializable() {
        let s = AppSettings::default();
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("0.0.0.0"));
        assert!(json.contains("8000"));
    }

    #[test]
    fn settings_deserializes_from_minimal_toml() {
        let toml_str = r#"
[server]
port = 9090
"#;
        let _settings: AppSettings = toml::from_str(toml_str).expect("should parse partial config");
    }
}
