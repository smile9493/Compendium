//! Local collaboration: audit log, patch proposals, entry locks.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::patch::{apply_patch, preview_patch, WikiPatchRequest, WikiPatchResult};

fn rsut_dir(knowledge_base: &Path) -> PathBuf {
    knowledge_base.join("wiki").join(".rsut")
}

fn audit_path(knowledge_base: &Path) -> PathBuf {
    rsut_dir(knowledge_base).join("audit.log")
}

fn proposals_dir(knowledge_base: &Path) -> PathBuf {
    rsut_dir(knowledge_base).join("proposals")
}

fn locks_dir(knowledge_base: &Path) -> PathBuf {
    rsut_dir(knowledge_base).join("locks")
}

/// Audit event types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Patch,
    Recompile,
    Compile,
    ProposalSubmit,
    ProposalApply,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: String,
    pub action: AuditAction,
    pub entry_path: Option<String>,
    pub actor: Option<String>,
    pub detail: Option<String>,
}

/// Append-only audit log under `wiki/.rsut/audit.log`.
pub fn append_audit(knowledge_base: &Path, event: AuditEvent) -> PdfResult<()> {
    let dir = rsut_dir(knowledge_base);
    fs::create_dir_all(&dir).map_err(storage_err)?;
    let line = serde_json::to_string(&event).map_err(|e| storage_err(e.to_string()))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(audit_path(knowledge_base))
        .map_err(storage_err)?;
    writeln!(file, "{line}").map_err(storage_err)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchProposal {
    pub id: String,
    pub created_at: String,
    pub request: WikiPatchRequest,
    pub preview: WikiPatchResult,
    pub status: String,
}

/// Submit a patch proposal without modifying wiki.
pub fn submit_patch_proposal(
    knowledge_base: &Path,
    request: WikiPatchRequest,
    actor: Option<String>,
) -> PdfResult<PatchProposal> {
    ensure_lock_free(knowledge_base, &request.entry_path)?;
    let preview = preview_patch(knowledge_base, &request)?;
    let id = Uuid::new_v4().to_string();
    let proposal = PatchProposal {
        id: id.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        request: request.clone(),
        preview,
        status: "pending".to_string(),
    };
    let dir = proposals_dir(knowledge_base);
    fs::create_dir_all(&dir).map_err(storage_err)?;
    let path = dir.join(format!("{id}.json"));
    let raw = serde_json::to_string_pretty(&proposal).map_err(|e| storage_err(e.to_string()))?;
    fs::write(&path, raw).map_err(storage_err)?;
    append_audit(
        knowledge_base,
        AuditEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
            action: AuditAction::ProposalSubmit,
            entry_path: Some(request.entry_path),
            actor,
            detail: Some(id),
        },
    )?;
    Ok(proposal)
}

/// Summary of a patch proposal for listing (no full diff payload).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatchProposalSummary {
    pub id: String,
    pub created_at: String,
    pub entry_path: String,
    pub status: String,
}

/// List patch proposals under `wiki/.rsut/proposals/`, newest first.
///
/// When `status_filter` is set, only proposals with matching `status` are returned.
pub fn list_patch_proposals(
    knowledge_base: &Path,
    status_filter: Option<&str>,
) -> PdfResult<Vec<PatchProposalSummary>> {
    let dir = proposals_dir(knowledge_base);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut summaries = Vec::new();
    for entry in fs::read_dir(&dir).map_err(storage_err)? {
        let entry = entry.map_err(storage_err)?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let raw = fs::read_to_string(&path).map_err(storage_err)?;
        let proposal: PatchProposal =
            serde_json::from_str(&raw).map_err(|e| storage_err(e.to_string()))?;
        if let Some(filter) = status_filter {
            if proposal.status != filter {
                continue;
            }
        }
        summaries.push(PatchProposalSummary {
            id: proposal.id,
            created_at: proposal.created_at,
            entry_path: proposal.request.entry_path,
            status: proposal.status,
        });
    }
    summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(summaries)
}

/// Apply a previously submitted proposal.
pub fn apply_patch_proposal(
    knowledge_base: &Path,
    proposal_id: &str,
    actor: Option<String>,
) -> PdfResult<WikiPatchResult> {
    let path = proposals_dir(knowledge_base).join(format!("{proposal_id}.json"));
    let raw = fs::read_to_string(&path)
        .map_err(|_| PdfModuleError::Storage(format!("proposal not found: {proposal_id}")))?;
    let proposal: PatchProposal =
        serde_json::from_str(&raw).map_err(|e| storage_err(e.to_string()))?;
    ensure_lock_free(knowledge_base, &proposal.request.entry_path)?;
    let result = apply_patch(knowledge_base, &proposal.request)?;
    append_audit(
        knowledge_base,
        AuditEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
            action: AuditAction::ProposalApply,
            entry_path: Some(proposal.request.entry_path),
            actor,
            detail: Some(proposal_id.to_string()),
        },
    )?;
    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryLock {
    pub entry_path: String,
    pub holder: String,
    pub expires_at: u64,
}

const DEFAULT_LOCK_TTL_SECS: u64 = 300;

/// Acquire lock for an entry (409 if held by another holder).
pub fn acquire_lock(
    knowledge_base: &Path,
    entry_path: &str,
    holder: &str,
    ttl_secs: Option<u64>,
) -> PdfResult<EntryLock> {
    let dir = locks_dir(knowledge_base);
    fs::create_dir_all(&dir).map_err(storage_err)?;
    let lock_path = lock_file_path(knowledge_base, entry_path);
    if lock_path.exists() {
        if let Ok(existing) = read_lock(&lock_path) {
            if existing.holder != holder && !lock_expired(&existing) {
                return Err(PdfModuleError::Storage(format!(
                    "entry locked by {}",
                    existing.holder
                )));
            }
        }
    }
    let ttl = ttl_secs.unwrap_or(DEFAULT_LOCK_TTL_SECS);
    let lock = EntryLock {
        entry_path: entry_path.to_string(),
        holder: holder.to_string(),
        expires_at: now_secs() + ttl,
    };
    let raw = serde_json::to_string(&lock).map_err(|e| storage_err(e.to_string()))?;
    fs::write(&lock_path, raw).map_err(storage_err)?;
    Ok(lock)
}

pub fn release_lock(knowledge_base: &Path, entry_path: &str, holder: &str) -> PdfResult<()> {
    let lock_path = lock_file_path(knowledge_base, entry_path);
    if !lock_path.exists() {
        return Ok(());
    }
    if let Ok(existing) = read_lock(&lock_path) {
        if existing.holder != holder && !lock_expired(&existing) {
            return Err(PdfModuleError::Storage("lock held by another actor".into()));
        }
    }
    fs::remove_file(&lock_path).map_err(storage_err)?;
    Ok(())
}

fn ensure_lock_free(knowledge_base: &Path, entry_path: &str) -> PdfResult<()> {
    let lock_path = lock_file_path(knowledge_base, entry_path);
    if !lock_path.exists() {
        return Ok(());
    }
    if let Ok(existing) = read_lock(&lock_path) {
        if !lock_expired(&existing) {
            return Err(PdfModuleError::Storage(format!(
                "entry locked by {} until {}",
                existing.holder, existing.expires_at
            )));
        }
        let _ = fs::remove_file(&lock_path);
    }
    Ok(())
}

fn lock_file_path(knowledge_base: &Path, entry_path: &str) -> PathBuf {
    let safe = entry_path.replace('/', "__");
    locks_dir(knowledge_base).join(format!("{safe}.lock"))
}

fn read_lock(path: &Path) -> PdfResult<EntryLock> {
    let raw = fs::read_to_string(path).map_err(storage_err)?;
    serde_json::from_str(&raw).map_err(|e| storage_err(e.to_string()))
}

fn lock_expired(lock: &EntryLock) -> bool {
    now_secs() >= lock.expires_at
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn storage_err(e: impl std::fmt::Display) -> PdfModuleError {
    PdfModuleError::Storage(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_list_patch_proposals_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("wiki")).unwrap();
        let list = list_patch_proposals(dir.path(), None).unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_list_patch_proposals_status_filter() {
        let dir = tempfile::tempdir().unwrap();
        let kb = dir.path();
        let pdir = proposals_dir(kb);
        fs::create_dir_all(&pdir).unwrap();

        let pending = r#"{
            "id": "p1",
            "created_at": "2026-01-02T00:00:00Z",
            "status": "pending",
            "request": { "entry_path": "IT/a.md", "operations": [] },
            "preview": { "entry_path": "IT/a.md", "diff": "", "applied": false, "new_size_bytes": 0 }
        }"#;
        let applied = r#"{
            "id": "p2",
            "created_at": "2026-01-01T00:00:00Z",
            "status": "applied",
            "request": { "entry_path": "IT/b.md", "operations": [] },
            "preview": { "entry_path": "IT/b.md", "diff": "", "applied": true, "new_size_bytes": 0 }
        }"#;
        fs::write(pdir.join("p1.json"), pending).unwrap();
        fs::write(pdir.join("p2.json"), applied).unwrap();

        let all = list_patch_proposals(kb, None).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, "p1");

        let pending_only = list_patch_proposals(kb, Some("pending")).unwrap();
        assert_eq!(pending_only.len(), 1);
        assert_eq!(pending_only[0].id, "p1");
    }
}
