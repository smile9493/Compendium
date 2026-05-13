//! Runtime feature flags for gradual rollout and A/B testing.
//!
//! Lightweight, self-contained toggle system — no external service required.
//! Flags are loaded from a JSON config file or environment variables on startup.
//!
//! # Feature flag types
//!
//! - **Boolean**: on/off for all users
//! - **Percentage**: X% of traffic sees the feature (hash-based, deterministic per user)
//! - **Targeted**: enabled only for specific user IDs or roles
//!
//! # Usage
//!
//! ```ignore
//! use pdf_common::feature_flags::{FeatureFlags, FlagValue};
//!
//! let flags = FeatureFlags::load("config/features.json")?;
//!
//! if flags.is_enabled("new-search-engine", Some("user-42")) {
//!     // Use new search engine
//! }
//! ```
//!
//! # Config format (config/features.json)
//!
//! ```json
//! {
//!   "new-search-engine": {
//!     "type": "percentage",
//!     "value": 20
//!   },
//!   "beta-dashboard": {
//!     "type": "targeted",
//!     "users": ["admin-1", "user-42"]
//!   },
//!   "maintenance-mode": {
//!     "type": "boolean",
//!     "value": false
//!   }
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FlagConfig {
    #[serde(rename = "boolean")]
    Boolean { value: bool },
    #[serde(rename = "percentage")]
    Percentage { value: u8 },
    #[serde(rename = "targeted")]
    Targeted { users: Vec<String> },
}

impl FlagConfig {
    pub fn boolean(value: bool) -> Self {
        Self::Boolean { value }
    }

    pub fn percentage(value: u8) -> Self {
        Self::Percentage {
            value: value.min(100),
        }
    }

    pub fn targeted(users: Vec<String>) -> Self {
        Self::Targeted { users }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagValue {
    On,
    Off,
}

pub struct FeatureFlags {
    flags: RwLock<HashMap<String, FlagConfig>>,
}

impl FeatureFlags {
    /// Load flags from a JSON file and environment variables.
    ///
    /// Environment variables override file values:
    /// `FEATURE_<FLAG_NAME>=true|false|<percentage>`
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let mut flags: HashMap<String, FlagConfig> = HashMap::new();

        let path = path.as_ref();
        if path.exists() {
            let content =
                std::fs::read_to_string(path).map_err(|e| format!("read features.json: {}", e))?;
            flags = serde_json::from_str(&content)
                .map_err(|e| format!("parse features.json: {}", e))?;
        }

        for (key, value) in std::env::vars() {
            if let Some(flag_name) = key.strip_prefix("FEATURE_") {
                let flag_name = flag_name.to_lowercase().replace('_', "-");
                let config = match value.to_lowercase().as_str() {
                    "true" => FlagConfig::boolean(true),
                    "false" => FlagConfig::boolean(false),
                    s if s.parse::<u8>().is_ok() => {
                        FlagConfig::percentage(s.parse().unwrap())
                    }
                    _ => FlagConfig::boolean(false),
                };
                flags.insert(flag_name, config);
            }
        }

        Ok(Self {
            flags: RwLock::new(flags),
        })
    }

    /// Create an empty feature flags instance (all flags default off).
    pub fn empty() -> Self {
        Self {
            flags: RwLock::new(HashMap::new()),
        }
    }

    /// Check if a feature is enabled for the given user context.
    ///
    /// `user_context` is used for percentage-based and targeted rollouts.
    /// Pass `None` for user-agnostic checks (percentage uses random).
    pub fn is_enabled(&self, flag: &str, user_context: Option<&str>) -> bool {
        let flags = self.flags.read().unwrap_or_else(|e| e.into_inner());
        match flags.get(flag) {
            Some(FlagConfig::Boolean { value }) => *value,
            Some(FlagConfig::Percentage { value }) => {
                let hash = deterministic_hash(user_context.unwrap_or(""));
                (hash % 100) < *value as u64
            }
            Some(FlagConfig::Targeted { users }) => {
                user_context.map_or(false, |u| users.iter().any(|allowed| allowed == u))
            }
            None => false,
        }
    }

    /// Get the raw config for a flag.
    pub fn get_config(&self, flag: &str) -> Option<FlagConfig> {
        let flags = self.flags.read().unwrap_or_else(|e| e.into_inner());
        flags.get(flag).cloned()
    }

    /// List all registered flag names.
    pub fn flag_names(&self) -> Vec<String> {
        let flags = self.flags.read().unwrap_or_else(|e| e.into_inner());
        flags.keys().cloned().collect()
    }

    /// Enable a flag at runtime (useful for admin override).
    pub fn set_flag(&self, name: &str, config: FlagConfig) {
        let mut flags = self.flags.write().unwrap_or_else(|e| e.into_inner());
        flags.insert(name.to_string(), config);
    }
}

fn deterministic_hash(key: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boolean_flag_on() {
        let flags = FeatureFlags::empty();
        flags.set_flag("test-flag", FlagConfig::boolean(true));
        assert!(flags.is_enabled("test-flag", None));
    }

    #[test]
    fn boolean_flag_off() {
        let flags = FeatureFlags::empty();
        flags.set_flag("test-flag", FlagConfig::boolean(false));
        assert!(!flags.is_enabled("test-flag", None));
    }

    #[test]
    fn unknown_flag_is_off() {
        let flags = FeatureFlags::empty();
        assert!(!flags.is_enabled("nonexistent", None));
    }

    #[test]
    fn percentage_100_always_on() {
        let flags = FeatureFlags::empty();
        flags.set_flag("pct", FlagConfig::percentage(100));
        assert!(flags.is_enabled("pct", Some("user-a")));
        assert!(flags.is_enabled("pct", Some("user-b")));
    }

    #[test]
    fn percentage_0_always_off() {
        let flags = FeatureFlags::empty();
        flags.set_flag("pct", FlagConfig::percentage(0));
        assert!(!flags.is_enabled("pct", Some("user-a")));
    }

    #[test]
    fn targeted_flag_matches() {
        let flags = FeatureFlags::empty();
        flags.set_flag("beta", FlagConfig::targeted(vec!["admin".into()]));

        assert!(flags.is_enabled("beta", Some("admin")));
        assert!(!flags.is_enabled("beta", Some("user")));
        assert!(!flags.is_enabled("beta", None));
    }

    #[test]
    fn deterministic_percentage_same_user() {
        let flags = FeatureFlags::empty();
        flags.set_flag("pct50", FlagConfig::percentage(50));

        let r1 = flags.is_enabled("pct50", Some("user-42"));
        let r2 = flags.is_enabled("pct50", Some("user-42"));
        assert_eq!(r1, r2);
    }

    #[test]
    fn env_override_parsing() {
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("features.json");
        std::fs::write(
            &config_path,
            r#"{"my-flag": {"type": "boolean", "value": false}}"#,
        )
        .unwrap();

        let flags = FeatureFlags::load(&config_path).unwrap();
        assert!(!flags.is_enabled("my-flag", None));
    }
}