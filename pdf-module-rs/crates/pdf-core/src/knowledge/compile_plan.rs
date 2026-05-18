//! Compile plan generation and task tracking (L1 / L2 / L3).

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::engine::KnowledgeEngine;
use crate::knowledge::entry::{CompileStatus, EntryLevel, KnowledgeEntry};
use crate::knowledge::hash_cache::HashCache;

const PLAN_FILE: &str = "compile_plan.json";

/// Kind of work item in a compile plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PlanTaskKind {
    CreateL1,
    AggregateL2,
    DomainMapL3,
    RecompileEntry,
}

/// Status of a plan task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PlanTaskStatus {
    #[default]
    Pending,
    Done,
    Failed,
}

/// A single task in the compile plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTask {
    pub id: String,
    pub kind: PlanTaskKind,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub status: PlanTaskStatus,
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// Full compile plan persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilePlan {
    pub plan_version: u32,
    pub generated_at: String,
    pub tasks: Vec<PlanTask>,
}

impl CompilePlan {
    pub fn task_mut(&mut self, task_id: &str) -> Option<&mut PlanTask> {
        self.tasks.iter_mut().find(|t| t.id == task_id)
    }
}

/// Read/write `.rsut_index/compile_plan.json`.
#[derive(Clone)]
pub struct CompilePlanStore {
    path: PathBuf,
}

impl CompilePlanStore {
    pub fn new(knowledge_base: &Path) -> Self {
        Self {
            path: knowledge_base.join(".rsut_index").join(PLAN_FILE),
        }
    }

    pub fn read(&self) -> PdfResult<Option<CompilePlan>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&self.path)
            .map_err(|e| PdfModuleError::Storage(e.to_string()))?;
        serde_json::from_str(&content)
            .map_err(|e| PdfModuleError::Storage(e.to_string()))
            .map(Some)
    }

    pub fn write(&self, plan: &CompilePlan) -> PdfResult<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| PdfModuleError::Storage(e.to_string()))?;
        }
        let json = serde_json::to_string_pretty(plan)
            .map_err(|e| PdfModuleError::Storage(e.to_string()))?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json).map_err(|e| PdfModuleError::Storage(e.to_string()))?;
        fs::rename(&tmp, &self.path).map_err(|e| PdfModuleError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn mark_task_done(&self, task_id: &str) -> PdfResult<CompilePlan> {
        let mut plan = self
            .read()?
            .ok_or_else(|| PdfModuleError::FileNotFound("No compile plan".to_string()))?;
        let task = plan
            .task_mut(task_id)
            .ok_or_else(|| PdfModuleError::FileNotFound(format!("Task {task_id} not found")))?;
        task.status = PlanTaskStatus::Done;
        self.write(&plan)?;
        Ok(plan)
    }
}

impl KnowledgeEngine {
    /// Generate a compile plan from raw prompts, aggregation candidates, and recompile queue.
    pub fn plan_compile(&self) -> PdfResult<CompilePlan> {
        let mut tasks = Vec::new();
        let kb = self.knowledge_base();
        let raw_dir = self.raw_dir();
        let wiki_dir = self.wiki_dir();

        let cache = HashCache::load_or_create(kb)?;
        if raw_dir.exists() {
            for entry in fs::read_dir(&raw_dir).map_err(|e| PdfModuleError::Storage(e.to_string()))?
            {
                let entry = entry.map_err(|e| PdfModuleError::Storage(e.to_string()))?;
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "pdf") {
                    if cache.needs_compile(&path).unwrap_or(true) {
                        let stem = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown");
                        let prompt = format!("raw/{stem}.compile_prompt.md");
                        tasks.push(PlanTask {
                            id: format!("l1-{stem}"),
                            kind: PlanTaskKind::CreateL1,
                            depends_on: Vec::new(),
                            status: PlanTaskStatus::Pending,
                            payload: serde_json::json!({
                                "pdf_path": path.to_string_lossy(),
                                "prompt_path": prompt,
                            }),
                        });
                    }
                }
            }
        }

        let candidates = self.identify_aggregation_candidates()?;
        for (i, c) in candidates.into_iter().enumerate() {
            tasks.push(PlanTask {
                id: format!("l2-{}-{}", c.domain, i),
                kind: PlanTaskKind::AggregateL2,
                depends_on: Vec::new(),
                status: PlanTaskStatus::Pending,
                payload: serde_json::json!({
                    "domain": c.domain,
                    "entry_paths": c.entry_paths,
                    "suggested_title": c.suggested_title,
                    "strategy": "graph_lpa",
                }),
            });
        }

        let domains = collect_domains(&wiki_dir)?;
        for domain in domains {
            if !has_l3_for_domain(&wiki_dir, &domain)? {
                tasks.push(PlanTask {
                    id: format!("l3-{}", domain.to_lowercase().replace(' ', "_")),
                    kind: PlanTaskKind::DomainMapL3,
                    depends_on: Vec::new(),
                    status: PlanTaskStatus::Pending,
                    payload: serde_json::json!({ "domain": domain }),
                });
            }
        }

        scan_recompile_tasks(&wiki_dir, &wiki_dir, &mut tasks)?;

        Ok(CompilePlan {
            plan_version: 1,
            generated_at: Utc::now().to_rfc3339(),
            tasks,
        })
    }

    /// Generate and persist the compile plan.
    pub fn generate_compile_plan(&self) -> PdfResult<CompilePlan> {
        let plan = self.plan_compile()?;
        CompilePlanStore::new(self.knowledge_base()).write(&plan)?;
        Ok(plan)
    }
}

fn collect_domains(wiki_dir: &Path) -> PdfResult<Vec<String>> {
    let mut domains = Vec::new();
    if !wiki_dir.exists() {
        return Ok(domains);
    }
    for entry in fs::read_dir(wiki_dir).map_err(|e| PdfModuleError::Storage(e.to_string()))? {
        let entry = entry.map_err(|e| PdfModuleError::Storage(e.to_string()))?;
        if entry.path().is_dir() {
            domains.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    Ok(domains)
}

fn has_l3_for_domain(wiki_dir: &Path, domain: &str) -> PdfResult<bool> {
    let domain_dir = wiki_dir.join(domain);
    if !domain_dir.exists() {
        return Ok(false);
    }
    let mut found = false;
    scan_l3(&domain_dir, &domain_dir, &mut found)?;
    Ok(found)
}

#[allow(clippy::only_used_in_recursion)]
fn scan_l3(base: &Path, dir: &Path, found: &mut bool) -> PdfResult<()> {
    if *found || !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| PdfModuleError::Storage(e.to_string()))? {
        let entry = entry.map_err(|e| PdfModuleError::Storage(e.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            scan_l3(base, &path, found)?;
        } else if path.extension().is_some_and(|e| e == "md") {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some(meta) = KnowledgeEntry::from_markdown(&content) {
                    if meta.level == EntryLevel::L3 {
                        *found = true;
                        return Ok(());
                    }
                }
            }
        }
    }
    Ok(())
}

#[allow(clippy::only_used_in_recursion)]
fn scan_recompile_tasks(base: &Path, dir: &Path, tasks: &mut Vec<PlanTask>) -> PdfResult<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| PdfModuleError::Storage(e.to_string()))? {
        let entry = entry.map_err(|e| PdfModuleError::Storage(e.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            scan_recompile_tasks(base, &path, tasks)?;
        } else if path.extension().is_some_and(|e| e == "md") {
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some(meta) = KnowledgeEntry::from_markdown(&content) {
                    if meta.status == CompileStatus::NeedsRecompile {
                        tasks.push(PlanTask {
                            id: format!("recompile-{}", rel.replace('/', "_")),
                            kind: PlanTaskKind::RecompileEntry,
                            depends_on: Vec::new(),
                            status: PlanTaskStatus::Pending,
                            payload: serde_json::json!({ "entry_path": rel }),
                        });
                    }
                }
            }
        }
    }
    Ok(())
}
