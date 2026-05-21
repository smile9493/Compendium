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
        Self { base: base.as_ref().to_path_buf() }
    }

    pub fn from_url(url: &str) -> PdfResult<Self> {
        let path = url.strip_prefix("file://").unwrap_or(url);
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

/// How to resolve path-level hash conflicts during push/pull.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncConflictResolution {
    /// Stop before writing; return `conflicts` for agent review.
    #[default]
    Abort,
    /// Keep local file bytes on conflict.
    PreferLocal,
    /// Keep remote file bytes on conflict.
    PreferRemote,
    /// Use newer filesystem mtime when hashes differ.
    PreferNewest,
}

/// A single path where local and remote manifests disagree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflict {
    pub path: String,
    pub local_hash: Option<String>,
    pub remote_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncReport {
    pub pushed: usize,
    pub pulled: usize,
    pub rebuilt_index_recommended: bool,
    #[serde(default)]
    pub conflicts: Vec<SyncConflict>,
    #[serde(default)]
    pub resolved: usize,
    #[serde(default)]
    pub aborted: bool,
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
    Ok(SyncManifest { root_hash, objects })
}

pub fn sync_status(knowledge_base: &Path, remote: &dyn SyncRemote) -> PdfResult<SyncStatus> {
    let local = build_local_manifest(knowledge_base)?;
    let remote_manifest = remote
        .get_manifest("HEAD")?
        .map(|b| serde_json::from_slice::<SyncManifest>(&b))
        .transpose()
        .map_err(|e| storage_err(e.to_string()))?;
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

pub fn sync_push(
    knowledge_base: &Path,
    remote: &dyn SyncRemote,
    resolution: SyncConflictResolution,
) -> PdfResult<SyncReport> {
    let local = build_local_manifest(knowledge_base)?;
    let remote_manifest = load_remote_head(remote)?;
    let conflicts = detect_push_conflicts(knowledge_base, &local, remote_manifest.as_ref())?;

    if !conflicts.is_empty() && resolution == SyncConflictResolution::Abort {
        return Ok(SyncReport {
            pushed: 0,
            pulled: 0,
            rebuilt_index_recommended: false,
            conflicts,
            resolved: 0,
            aborted: true,
        });
    }

    let mut pushed = 0usize;
    let mut resolved = 0usize;
    let conflict_paths: std::collections::HashSet<_> =
        conflicts.iter().map(|c| c.path.as_str()).collect();

    for (rel, hash) in &local.objects {
        let key = object_key(hash);
        let is_conflict = conflict_paths.contains(rel.as_str());
        if is_conflict {
            match resolution {
                SyncConflictResolution::PreferLocal | SyncConflictResolution::PreferNewest => {
                    if resolution == SyncConflictResolution::PreferNewest
                        && !local_wins_mtime(knowledge_base, rel, remote)?
                    {
                        continue;
                    }
                    let data = fs::read(knowledge_base.join(rel)).map_err(storage_err)?;
                    remote.put_object(&key, &data)?;
                    pushed += 1;
                    resolved += 1;
                }
                SyncConflictResolution::PreferRemote => {
                    // Keep remote object; skip uploading local bytes for this path.
                    resolved += 1;
                }
                SyncConflictResolution::Abort => {}
            }
            continue;
        }
        if !remote.has_object(&key)? {
            let data = fs::read(knowledge_base.join(rel)).map_err(storage_err)?;
            remote.put_object(&key, &data)?;
            pushed += 1;
        }
    }
    let raw = serde_json::to_vec(&local).map_err(|e| storage_err(e.to_string()))?;
    remote.put_manifest("HEAD", &raw)?;
    persist_local_head(knowledge_base, &local)?;
    Ok(SyncReport {
        pushed,
        pulled: 0,
        rebuilt_index_recommended: false,
        conflicts,
        resolved,
        aborted: false,
    })
}

pub fn sync_pull(
    knowledge_base: &Path,
    remote: &dyn SyncRemote,
    resolution: SyncConflictResolution,
) -> PdfResult<SyncReport> {
    let Some(raw) = remote.get_manifest("HEAD")? else {
        return Err(PdfModuleError::Storage("remote has no HEAD manifest".into()));
    };
    let remote_manifest: SyncManifest =
        serde_json::from_slice(&raw).map_err(|e| storage_err(e.to_string()))?;
    let local = build_local_manifest(knowledge_base).ok();
    let conflicts = detect_pull_conflicts(knowledge_base, local.as_ref(), &remote_manifest)?;

    if !conflicts.is_empty() && resolution == SyncConflictResolution::Abort {
        return Ok(SyncReport {
            pushed: 0,
            pulled: 0,
            rebuilt_index_recommended: false,
            conflicts,
            resolved: 0,
            aborted: true,
        });
    }

    let mut pulled = 0usize;
    let mut resolved = 0usize;
    let conflict_by_path: std::collections::HashMap<_, _> =
        conflicts.iter().map(|c| (c.path.as_str(), c)).collect();

    for (rel, hash) in &remote_manifest.objects {
        let dest = knowledge_base.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(storage_err)?;
        }
        let key = object_key(hash);
        if conflict_by_path.contains_key(rel.as_str()) {
            match resolution {
                SyncConflictResolution::PreferLocal => {
                    resolved += 1;
                    continue;
                }
                SyncConflictResolution::PreferRemote => {
                    let data = remote.get_object(&key)?;
                    fs::write(&dest, &data).map_err(storage_err)?;
                    pulled += 1;
                    resolved += 1;
                }
                SyncConflictResolution::PreferNewest => {
                    if local_wins_mtime(knowledge_base, rel, remote)? {
                        resolved += 1;
                        continue;
                    }
                    let data = remote.get_object(&key)?;
                    fs::write(&dest, &data).map_err(storage_err)?;
                    pulled += 1;
                    resolved += 1;
                }
                SyncConflictResolution::Abort => {}
            }
            continue;
        }
        let data = remote.get_object(&key)?;
        fs::write(&dest, &data).map_err(storage_err)?;
        record_remote_mtime(knowledge_base, rel)?;
        pulled += 1;
    }
    persist_local_head(knowledge_base, &remote_manifest)?;
    Ok(SyncReport {
        pushed: 0,
        pulled,
        rebuilt_index_recommended: true,
        conflicts,
        resolved,
        aborted: false,
    })
}

fn load_remote_head(remote: &dyn SyncRemote) -> PdfResult<Option<SyncManifest>> {
    remote
        .get_manifest("HEAD")?
        .map(|b| serde_json::from_slice::<SyncManifest>(&b))
        .transpose()
        .map_err(|e| storage_err(e.to_string()))
}

fn detect_push_conflicts(
    _knowledge_base: &Path,
    local: &SyncManifest,
    remote: Option<&SyncManifest>,
) -> PdfResult<Vec<SyncConflict>> {
    let Some(remote) = remote else {
        return Ok(Vec::new());
    };
    if remote.root_hash == local.root_hash {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for (rel, local_hash) in &local.objects {
        if let Some(remote_hash) = remote.objects.get(rel)
            && remote_hash != local_hash
        {
            out.push(SyncConflict {
                path: rel.clone(),
                local_hash: Some(local_hash.clone()),
                remote_hash: remote_hash.clone(),
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn detect_pull_conflicts(
    knowledge_base: &Path,
    local: Option<&SyncManifest>,
    remote: &SyncManifest,
) -> PdfResult<Vec<SyncConflict>> {
    let Some(local) = local else {
        return Ok(Vec::new());
    };
    if remote.root_hash == local.root_hash {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for (rel, remote_hash) in &remote.objects {
        let local_path = knowledge_base.join(rel);
        if !local_path.is_file() {
            continue;
        }
        let local_hash = local
            .objects
            .get(rel)
            .cloned()
            .or_else(|| fs::read(&local_path).ok().map(|b| hex_hash(&b)));
        if local_hash.as_ref() != Some(remote_hash) {
            out.push(SyncConflict {
                path: rel.clone(),
                local_hash,
                remote_hash: remote_hash.clone(),
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn local_wins_mtime(knowledge_base: &Path, rel: &str, _remote: &dyn SyncRemote) -> PdfResult<bool> {
    let local_path = knowledge_base.join(rel);
    let local_mtime = fs::metadata(&local_path).and_then(|m| m.modified()).ok();
    let sync_mtime_path = sync_dir(knowledge_base).join("remote_mtime").join(rel);
    let remote_mtime = fs::metadata(&sync_mtime_path).and_then(|m| m.modified()).ok();
    match (local_mtime, remote_mtime) {
        (Some(l), Some(r)) => Ok(l > r),
        (Some(_), None) => Ok(true),
        _ => Ok(true),
    }
}

fn record_remote_mtime(knowledge_base: &Path, rel: &str) -> PdfResult<()> {
    let src = knowledge_base.join(rel);
    let dest = sync_dir(knowledge_base).join("remote_mtime").join(rel);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(storage_err)?;
    }
    if src.is_file() {
        fs::copy(&src, &dest).map_err(storage_err)?;
    }
    Ok(())
}

fn persist_local_head(knowledge_base: &Path, manifest: &SyncManifest) -> PdfResult<()> {
    let dir = sync_dir(knowledge_base);
    fs::create_dir_all(&dir).map_err(storage_err)?;
    fs::write(dir.join("HEAD"), &manifest.root_hash).map_err(storage_err)?;
    let raw = serde_json::to_vec_pretty(manifest).map_err(|e| storage_err(e.to_string()))?;
    fs::write(dir.join("manifest.json"), raw).map_err(storage_err)
}

fn walk_and_hash(dir: &Path, kb_root: &Path, out: &mut HashMap<String, String>) -> PdfResult<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_sync_dirs(test_name: &str) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("sync_{test_name}_{}", std::process::id()));
        (base.join("kb"), base.join("remote"))
    }

    #[test]
    fn pull_detects_local_remote_hash_conflict() {
        let (kb, remote_base) = temp_sync_dirs("pull_abort");
        let _ = fs::remove_dir_all(&kb);
        let _ = fs::remove_dir_all(&remote_base);
        fs::create_dir_all(kb.join("wiki")).unwrap();
        fs::write(kb.join("wiki/a.md"), "local v1").unwrap();
        let remote = FileSyncRemote::new(&remote_base);
        let local_manifest = build_local_manifest(&kb).unwrap();
        let raw = serde_json::to_vec(&local_manifest).unwrap();
        remote.put_manifest("HEAD", &raw).unwrap();
        for (rel, hash) in &local_manifest.objects {
            let data = fs::read(kb.join(rel)).unwrap();
            remote.put_object(&object_key(hash), &data).unwrap();
        }
        fs::write(kb.join("wiki/a.md"), "local v2").unwrap();
        let report = sync_pull(&kb, &remote, SyncConflictResolution::Abort).unwrap();
        assert!(report.aborted);
        assert!(!report.conflicts.is_empty());
        assert_eq!(fs::read_to_string(kb.join("wiki/a.md")).unwrap(), "local v2");
        let _ = fs::remove_dir_all(&kb);
        let _ = fs::remove_dir_all(&remote_base);
    }

    #[test]
    fn push_prefer_remote_skips_conflict_paths() {
        let (kb, remote_base) = temp_sync_dirs("push_prefer_remote");
        let _ = fs::remove_dir_all(&kb);
        let _ = fs::remove_dir_all(&remote_base);
        fs::create_dir_all(kb.join("wiki")).unwrap();
        fs::write(kb.join("wiki/c.md"), "local v1").unwrap();
        let remote = FileSyncRemote::new(&remote_base);
        let local_manifest = build_local_manifest(&kb).unwrap();
        remote.put_manifest("HEAD", &serde_json::to_vec(&local_manifest).unwrap()).unwrap();
        for (rel, hash) in &local_manifest.objects {
            let data = fs::read(kb.join(rel)).unwrap();
            remote.put_object(&object_key(hash), &data).unwrap();
        }
        fs::write(kb.join("wiki/c.md"), "local v2").unwrap();
        let report = sync_push(&kb, &remote, SyncConflictResolution::PreferRemote).unwrap();
        assert!(!report.aborted);
        assert!(!report.conflicts.is_empty());
        assert!(report.resolved >= 1);
        assert_eq!(fs::read_to_string(kb.join("wiki/c.md")).unwrap(), "local v2");
        let _ = fs::remove_dir_all(&kb);
        let _ = fs::remove_dir_all(&remote_base);
    }

    #[test]
    fn pull_prefer_remote_resolves_conflict() {
        let (kb, remote_base) = temp_sync_dirs("pull_prefer_remote");
        let _ = fs::remove_dir_all(&kb);
        let _ = fs::remove_dir_all(&remote_base);
        fs::create_dir_all(kb.join("wiki")).unwrap();
        fs::write(kb.join("wiki/b.md"), "local").unwrap();
        let remote = FileSyncRemote::new(&remote_base);
        fs::write(kb.join("wiki/b.md"), "remote-wins").unwrap();
        let manifest = build_local_manifest(&kb).unwrap();
        remote.put_manifest("HEAD", &serde_json::to_vec(&manifest).unwrap()).unwrap();
        for (rel, hash) in &manifest.objects {
            let data = fs::read(kb.join(rel)).unwrap();
            remote.put_object(&object_key(hash), &data).unwrap();
        }
        fs::write(kb.join("wiki/b.md"), "local").unwrap();
        let report = sync_pull(&kb, &remote, SyncConflictResolution::PreferRemote).unwrap();
        assert!(!report.aborted);
        assert_eq!(fs::read_to_string(kb.join("wiki/b.md")).unwrap(), "remote-wins");
        let _ = fs::remove_dir_all(&kb);
        let _ = fs::remove_dir_all(&remote_base);
    }
}
