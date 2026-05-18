//! Multi-knowledge-base workspace registry (`~/.rsut/workspaces.toml`).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::error::{PdfModuleError, PdfResult};

/// Stable identifier for a registered knowledge base.
pub type WorkspaceId = String;

/// Single workspace entry in the registry file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceEntry {
    pub id: WorkspaceId,
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub active: bool,
}

/// On-disk registry format.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct WorkspaceFile {
    #[serde(default, rename = "workspace")]
    workspaces: Vec<WorkspaceEntry>,
}

/// Registry of knowledge bases with active-workspace selection.
#[derive(Debug)]
pub struct WorkspaceRegistry {
    config_path: PathBuf,
    allowed_roots: Vec<PathBuf>,
    inner: RwLock<WorkspaceFile>,
}

impl WorkspaceRegistry {
    /// Load or create registry at `~/.rsut/workspaces.toml` (override via `RSUT_CONFIG_DIR`).
    pub fn load_default() -> PdfResult<Self> {
        let config_dir = config_dir();
        fs::create_dir_all(&config_dir).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to create config dir {}: {e}", config_dir.display()))
        })?;
        let config_path = config_dir.join("workspaces.toml");
        Self::load(&config_path)
    }

    /// Load registry from a specific path.
    pub fn load(config_path: &Path) -> PdfResult<Self> {
        let allowed_roots = parse_allowed_roots();
        let inner = if config_path.exists() {
            let raw = fs::read_to_string(config_path).map_err(|e| {
                PdfModuleError::Storage(format!("Failed to read {}: {e}", config_path.display()))
            })?;
            toml::from_str(&raw).map_err(|e| {
                PdfModuleError::Storage(format!("Invalid workspaces.toml: {e}"))
            })?
        } else {
            WorkspaceFile::default()
        };

        let registry = Self {
            config_path: config_path.to_path_buf(),
            allowed_roots,
            inner: RwLock::new(inner),
        };
        registry.ensure_default_from_env()?;
        Ok(registry)
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// List all registered workspaces (clone).
    pub fn list(&self) -> PdfResult<Vec<WorkspaceEntry>> {
        let file = self.inner.read().map_err(|_| lock_poisoned())?;
        Ok(file.workspaces.clone())
    }

    /// Active workspace id, if any.
    pub fn active_id(&self) -> PdfResult<Option<WorkspaceId>> {
        let file = self.inner.read().map_err(|_| lock_poisoned())?;
        Ok(file
            .workspaces
            .iter()
            .find(|w| w.active)
            .map(|w| w.id.clone()))
    }

    /// Resolve `kb_id` or legacy path string to an absolute KB path.
    pub fn resolve_kb(
        &self,
        kb_id: Option<&str>,
        knowledge_base: Option<&str>,
    ) -> PdfResult<PathBuf> {
        if let Some(id) = kb_id {
            return self.path_for_id(id);
        }
        if let Some(path) = knowledge_base {
            let pb = PathBuf::from(path);
            self.validate_path(&pb)?;
            return Ok(pb);
        }
        if let Some(active) = self.active_id()? {
            return self.path_for_id(&active);
        }
        if let Ok(env) = std::env::var("KNOWLEDGE_BASE_PATH") {
            let pb = PathBuf::from(env);
            self.validate_path(&pb)?;
            return Ok(pb);
        }
        if let Ok(env) = std::env::var("KNOWLEDGE_BASE") {
            let pb = PathBuf::from(env);
            self.validate_path(&pb)?;
            return Ok(pb);
        }
        Err(PdfModuleError::Storage(
            "No knowledge base: set kb_id, knowledge_base, active workspace, or KNOWLEDGE_BASE_PATH"
                .into(),
        ))
    }

    pub fn path_for_id(&self, id: &str) -> PdfResult<PathBuf> {
        let file = self.inner.read().map_err(|_| lock_poisoned())?;
        let entry = file
            .workspaces
            .iter()
            .find(|w| w.id == id)
            .ok_or_else(|| PdfModuleError::Storage(format!("Unknown workspace id: {id}")))?;
        self.validate_path(&entry.path)?;
        Ok(entry.path.clone())
    }

    /// Register or update a workspace.
    pub fn upsert(&self, entry: WorkspaceEntry) -> PdfResult<()> {
        self.validate_path(&entry.path)?;
        if entry.id.trim().is_empty() {
            return Err(PdfModuleError::Storage("workspace id must not be empty".into()));
        }
        let mut file = self.inner.write().map_err(|_| lock_poisoned())?;
        if let Some(existing) = file.workspaces.iter_mut().find(|w| w.id == entry.id) {
            *existing = entry;
        } else {
            file.workspaces.push(entry);
        }
        self.persist(&file)
    }

    /// Remove workspace by id.
    pub fn remove(&self, id: &str) -> PdfResult<bool> {
        let mut file = self.inner.write().map_err(|_| lock_poisoned())?;
        let len_before = file.workspaces.len();
        file.workspaces.retain(|w| w.id != id);
        let removed = file.workspaces.len() < len_before;
        if removed {
            self.persist(&file)?;
        }
        Ok(removed)
    }

    /// Set active workspace (clears other active flags).
    pub fn set_active(&self, id: &str) -> PdfResult<()> {
        let mut file = self.inner.write().map_err(|_| lock_poisoned())?;
        let found = file.workspaces.iter().any(|w| w.id == id);
        if !found {
            return Err(PdfModuleError::Storage(format!("Unknown workspace id: {id}")));
        }
        for w in &mut file.workspaces {
            w.active = w.id == id;
        }
        self.persist(&file)
    }

    fn persist(&self, file: &WorkspaceFile) -> PdfResult<()> {
        let serialized = toml::to_string_pretty(file).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to serialize workspaces: {e}"))
        })?;
        let tmp = self.config_path.with_extension("toml.tmp");
        fs::write(&tmp, &serialized).map_err(|e| {
            PdfModuleError::Storage(format!("Failed to write {}: {e}", tmp.display()))
        })?;
        fs::rename(&tmp, &self.config_path).map_err(|e| {
            PdfModuleError::Storage(format!(
                "Failed to rename {} -> {}: {e}",
                tmp.display(),
                self.config_path.display()
            ))
        })?;
        Ok(())
    }

    fn ensure_default_from_env(&self) -> PdfResult<()> {
        let mut file = self.inner.write().map_err(|_| lock_poisoned())?;
        if !file.workspaces.is_empty() {
            return Ok(());
        }
        let default_path = std::env::var("KNOWLEDGE_BASE_PATH")
            .or_else(|_| std::env::var("KNOWLEDGE_BASE"))
            .unwrap_or_else(|_| "/app/kb".to_string());
        let path = PathBuf::from(default_path);
        if self.validate_path(&path).is_ok() {
            file.workspaces.push(WorkspaceEntry {
                id: "default".to_string(),
                name: "Default".to_string(),
                path,
                active: true,
            });
            drop(file);
            let file = self.inner.read().map_err(|_| lock_poisoned())?;
            self.persist(&file)?;
        }
        Ok(())
    }

    fn validate_path(&self, path: &Path) -> PdfResult<()> {
        if path.as_os_str().is_empty() {
            return Err(PdfModuleError::Storage("empty knowledge base path".into()));
        }
        if path.to_string_lossy().contains("..") {
            return Err(PdfModuleError::Storage(format!(
                "path traversal not allowed: {}",
                path.display()
            )));
        }
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !self.allowed_roots.is_empty() {
            let ok = self.allowed_roots.iter().any(|root| {
                canonical.starts_with(root.canonicalize().unwrap_or_else(|_| root.clone()))
            });
            if !ok {
                return Err(PdfModuleError::Storage(format!(
                    "path {} is not under allowed roots",
                    canonical.display()
                )));
            }
        }
        Ok(())
    }
}

fn config_dir() -> PathBuf {
    std::env::var("RSUT_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs_home().map(|h| h.join(".rsut")).unwrap_or_else(|| PathBuf::from(".rsut"))
        })
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
}

fn parse_allowed_roots() -> Vec<PathBuf> {
    std::env::var("RSUT_ALLOWED_KB_ROOTS")
        .ok()
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default()
}

fn lock_poisoned() -> PdfModuleError {
    PdfModuleError::Storage("workspace registry lock poisoned".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_upsert_and_resolve() {
        let dir = TempDir::new().expect("tempdir");
        let kb = dir.path().join("kb");
        fs::create_dir_all(&kb).expect("mkdir");
        let cfg = dir.path().join("workspaces.toml");
        let reg = WorkspaceRegistry::load(&cfg).expect("load");
        reg.upsert(WorkspaceEntry {
            id: "test".into(),
            name: "Test".into(),
            path: kb.clone(),
            active: true,
        })
        .expect("upsert");
        let resolved = reg.resolve_kb(Some("test"), None).expect("resolve");
        assert_eq!(resolved, kb.canonicalize().unwrap_or(kb));
    }
}
