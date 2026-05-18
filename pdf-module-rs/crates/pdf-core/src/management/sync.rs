//! Git-like content-addressed sync (local-first, optional file remote).

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{PdfModuleError, PdfResult};

const SYNC_DIR: &str = ".rsut_sync";

/// Remote storage for sync blobs and manifests.
pub trait SyncRemote: Send + Sync {
    fn put_object(&self, key: &str, data: &[u8]) -> PdfResult<()>;
    fn get_object(&self, key: &str) -> PdfResult<Vec<u8>>;
    fn has_object(&self, key: &str) -> PdfResult<bool>;
    fn put_manifest(&self, name: &str, data: &[u8]) -> PdfResult<()>;
    fn get_manifest(&self, name: &str) -> PdfResult<Option<Vec<u8>>>;
}

/// Directory-backed remote (`file:///path` or plain path).
pub struct FileSyncRemote {
    base: PathBuf,
}

impl FileSyncRemote {
    pub fn new(base: impl AsRef<Path>) -> Self {
        Self {
            base: base.as_ref().to_path_buf(),
        }
    }

    pub fn from_url(url: &str) -> PdfResult<Self> {
        let path = url
            .strip_prefix("file://")
            .unwrap_or(url);
        let base = PathBuf::from(path);
        fs::create_dir_all(&base).map_err(storage_err)?;
        Ok(Self::new(base))
    }

    fn object_path(&self, key: &str) -> PathBuf {
        self.base.join("objects").join(key)
    }

    fn manifest_path(&self, name: &str) -> PathBuf {
        self.base.join("manifests").join(name)
    }
}

impl SyncRemote for FileSyncRemote {
    fn put_object(&self, key: &str, data: &[u8]) -> PdfResult<()> {
        let path = self.object_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(storage_err)?;
        }
        fs::write(path, data).map_err(storage_err)
    }

    fn get_object(&self, key: &str) -> PdfResult<Vec<u8>> {
        fs::read(self.object_path(key)).map_err(storage_err)
    }

    fn has_object(&self, key: &str) -> PdfResult<bool> {
        Ok(self.object_path(key).exists())
    }

    fn put_manifest(&self, name: &str, data: &[u8]) -> PdfResult<()> {
        let path = self.manifest_path(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(storage_err)?;
        }
        fs::write(path, data).map_err(storage_err)
    }

    fn get_manifest(&self, name: &str) -> PdfResult<Option<Vec<u8>>> {
        let path = self.manifest_path(name);
        if !path.exists() {
            return Ok(None);
        }
        fs::read(path).map(Some).map_err(storage_err)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncManifest {
    pub root_hash: String,
    pub objects: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub local_root: String,
    pub remote_root: Option<String>,
    pub in_sync: bool,
    pub local_objects: usize,
    pub remote_objects: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncReport {
    pub pushed: usize,
    pub pulled: usize,
    pub rebuilt_index_recommended: bool,
}

pub fn sync_dir(knowledge_base: &Path) -> PathBuf {
    knowledge_base.join(SYNC_DIR)
}

/// Build manifest from wiki/, raw/, schema/ file hashes.
pub fn build_local_manifest(knowledge_base: &Path) -> PdfResult<SyncManifest> {
    let mut objects = HashMap::new();
    for sub in ["wiki", "raw", "schema"] {
        let dir = knowledge_base.join(sub);
        if dir.exists() {
            walk_and_hash(&dir, knowledge_base, &mut objects)?;
        }
    }
    let root_hash = merkle_root(&objects);
    Ok(SyncManifest {
        root_hash,
        objects,
    })
}

pub fn sync_status(knowledge_base: &Path, remote: &dyn SyncRemote) -> PdfResult<SyncStatus> {
    let local = build_local_manifest(knowledge_base)?;
    let remote_manifest = remote
        .get_manifest("HEAD")?
        .map(|b| serde_json::from_slice::<SyncManifest>(&b))
        .transpose()
        .map_err(|e| storage_err(&e.to_string()))?;
    let remote_root = remote_manifest.as_ref().map(|m| m.root_hash.clone());
    let in_sync = remote_root.as_deref() == Some(local.root_hash.as_str());
    Ok(SyncStatus {
        local_root: local.root_hash,
        remote_root,
        in_sync,
        local_objects: local.objects.len(),
        remote_objects: remote_manifest.map(|m| m.objects.len()).unwrap_or(0),
    })
}

pub fn sync_push(knowledge_base: &Path, remote: &dyn SyncRemote) -> PdfResult<SyncReport> {
    let manifest = build_local_manifest(knowledge_base)?;
    let mut pushed = 0usize;
    for (rel, hash) in &manifest.objects {
        let key = object_key(hash);
        if !remote.has_object(&key)? {
            let data = fs::read(knowledge_base.join(rel)).map_err(storage_err)?;
            remote.put_object(&key, &data)?;
            pushed += 1;
        }
    }
    let raw = serde_json::to_vec(&manifest).map_err(|e| storage_err(&e.to_string()))?;
    remote.put_manifest("HEAD", &raw)?;
    persist_local_head(knowledge_base, &manifest)?;
    Ok(SyncReport {
        pushed,
        pulled: 0,
        rebuilt_index_recommended: false,
    })
}

pub fn sync_pull(knowledge_base: &Path, remote: &dyn SyncRemote) -> PdfResult<SyncReport> {
    let Some(raw) = remote.get_manifest("HEAD")? else {
        return Err(PdfModuleError::Storage("remote has no HEAD manifest".into()));
    };
    let manifest: SyncManifest =
        serde_json::from_slice(&raw).map_err(|e| storage_err(&e.to_string()))?;
    let mut pulled = 0usize;
    for (rel, hash) in &manifest.objects {
        let dest = knowledge_base.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(storage_err)?;
        }
        let key = object_key(hash);
        let data = remote.get_object(&key)?;
        fs::write(&dest, &data).map_err(storage_err)?;
        pulled += 1;
    }
    persist_local_head(knowledge_base, &manifest)?;
    Ok(SyncReport {
        pushed: 0,
        pulled,
        rebuilt_index_recommended: true,
    })
}

fn persist_local_head(knowledge_base: &Path, manifest: &SyncManifest) -> PdfResult<()> {
    let dir = sync_dir(knowledge_base);
    fs::create_dir_all(&dir).map_err(storage_err)?;
    fs::write(dir.join("HEAD"), &manifest.root_hash).map_err(storage_err)?;
    let raw = serde_json::to_vec_pretty(manifest).map_err(|e| storage_err(&e.to_string()))?;
    fs::write(dir.join("manifest.json"), raw).map_err(storage_err)
}

fn walk_and_hash(
    dir: &Path,
    kb_root: &Path,
    out: &mut HashMap<String, String>,
) -> PdfResult<()> {
    for entry in fs::read_dir(dir).map_err(storage_err)? {
        let entry = entry.map_err(storage_err)?;
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|n| n.to_str()) == Some(".rsut") {
                continue;
            }
            walk_and_hash(&path, kb_root, out)?;
        } else if path.is_file() {
            let rel = path
                .strip_prefix(kb_root)
                .map_err(|_| storage_err("strip prefix"))?
                .to_string_lossy()
                .replace('\\', "/");
            if rel.contains(".rsut_sync/") {
                continue;
            }
            let mut file = fs::File::open(&path).map_err(storage_err)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).map_err(storage_err)?;
            out.insert(rel, hex_hash(&buf));
        }
    }
    Ok(())
}

fn hex_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn merkle_root(objects: &HashMap<String, String>) -> String {
    let mut leaves: Vec<String> = objects.values().cloned().collect();
    leaves.sort();
    let joined = leaves.join("|");
    hex_hash(joined.as_bytes())
}

fn object_key(hash: &str) -> String {
    format!("{}/{}", &hash[..2], &hash[2..])
}

fn storage_err(e: impl std::fmt::Display) -> PdfModuleError {
    PdfModuleError::Storage(e.to_string())
}
