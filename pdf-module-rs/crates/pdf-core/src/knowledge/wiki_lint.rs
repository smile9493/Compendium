//! Aggregated wiki lint (Karpathy `lint` command).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{PdfModuleError, PdfResult};
use crate::knowledge::entry::KnowledgeEntry;
use crate::knowledge::index::GraphIndex;
use crate::knowledge::knowledge_decay::{DEFAULT_STALE_DAYS, StaleEntry, detect_stale_entries};
use crate::knowledge::quality::analyze_wiki;
use crate::wiki::{NervousEvent, NervousEventKind, sync_nervous_system};

/// Full lint report for a knowledge base.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LintWikiReport {
    pub orphan_entries: Vec<String>,
    pub broken_links: Vec<String>,
    pub contradiction_pairs: Vec<ContradictionPair>,
    pub drift_pair_count: usize,
    pub missing_concept_hints: Vec<String>,
    pub issue_count: usize,
    pub recommended_research: Vec<String>,
    pub stale_entries: Vec<StaleEntry>,
    pub load_bearing_count: usize,
    pub summary: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ContradictionPair {
    pub entry_a: String,
    pub entry_b: String,
}

/// Run Karpathy-style lint checks and append a log line.
pub fn lint_wiki(knowledge_base: &Path) -> PdfResult<LintWikiReport> {
    let wiki_dir = knowledge_base.join("wiki");
    let quality = analyze_wiki(&wiki_dir)?;

    let mut graph = GraphIndex::new();
    let _ = graph.rebuild(&wiki_dir);
    let orphans = graph.find_orphans();
    let load_bearing = graph.load_bearing_entries(3);
    let stale_entries = detect_stale_entries(knowledge_base, DEFAULT_STALE_DAYS)?;

    let entries = collect_wiki_entries(&wiki_dir)?;
    let contradiction_pairs = find_contradiction_pairs(&entries);
    let missing = find_missing_concept_hints(&entries);

    let mut recommended_research: Vec<String> =
        orphans.iter().take(5).map(|p| format!("Link or merge orphan entry: {}", p)).collect();
    for hint in missing.iter().take(5) {
        recommended_research.push(format!("Create dedicated page for frequent concept: {}", hint));
    }

    for stale in stale_entries.iter().take(3) {
        recommended_research.push(format!(
            "Re-validate stale entry ({} days): {}",
            stale.days_since_validated.unwrap_or(stale.days_since_update),
            stale.path
        ));
    }

    let summary = format!(
        "issues={} orphans={} broken_links={} contradictions={} drift_pairs={} missing_hints={} stale={} load_bearing={}",
        quality.issues.len(),
        orphans.len(),
        quality.broken_links.len(),
        contradiction_pairs.len(),
        quality.drift_pairs.len(),
        missing.len(),
        stale_entries.len(),
        load_bearing.len()
    );

    let report = LintWikiReport {
        orphan_entries: orphans,
        broken_links: quality.broken_links.clone(),
        contradiction_pairs,
        drift_pair_count: quality.drift_pairs.len(),
        missing_concept_hints: missing,
        issue_count: quality.issues.len(),
        recommended_research,
        stale_entries,
        load_bearing_count: load_bearing.len(),
        summary: summary.clone(),
    };

    let _ = sync_nervous_system(knowledge_base, NervousEvent::new(NervousEventKind::Lint, summary));

    Ok(report)
}

fn collect_wiki_entries(wiki_dir: &Path) -> PdfResult<Vec<(String, KnowledgeEntry)>> {
    let mut paths = Vec::new();
    scan_wiki_files(wiki_dir, wiki_dir, &mut paths)?;
    let mut results = Vec::new();
    for path in paths {
        let rel = path.strip_prefix(wiki_dir).unwrap_or(&path).to_string_lossy().to_string();
        if let Ok(content) = fs::read_to_string(&path)
            && let Some(entry) = KnowledgeEntry::from_markdown(&content)
        {
            results.push((rel, entry));
        }
    }
    Ok(results)
}

fn scan_wiki_files(_base: &Path, dir: &Path, result: &mut Vec<PathBuf>) -> PdfResult<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)
        .map_err(|e| PdfModuleError::Storage(format!("read dir: {}", e)))?
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "index.md" || name == "log.md" {
            continue;
        }
        if path.is_dir() {
            if name != ".versions" {
                scan_wiki_files(_base, &path, result)?;
            }
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            result.push(path);
        }
    }
    Ok(())
}

fn find_contradiction_pairs(entries: &[(String, KnowledgeEntry)]) -> Vec<ContradictionPair> {
    let paths: HashSet<String> = entries.iter().map(|(p, _)| p.clone()).collect();
    let mut seen = HashSet::new();
    let mut pairs = Vec::new();

    for (path, entry) in entries {
        for contra in &entry.contradictions {
            let normalized = contra.strip_prefix("wiki/").unwrap_or(contra.as_str()).to_string();
            if paths.contains(&normalized) {
                let mut pair = [path.clone(), normalized];
                pair.sort();
                let key = format!("{}|{}", pair[0], pair[1]);
                if seen.insert(key) {
                    pairs.push(ContradictionPair {
                        entry_a: pair[0].clone(),
                        entry_b: pair[1].clone(),
                    });
                }
            }
        }
    }
    pairs
}

fn find_missing_concept_hints(entries: &[(String, KnowledgeEntry)]) -> Vec<String> {
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    let titles_lower: HashSet<String> =
        entries.iter().map(|(_, e)| e.title.to_lowercase()).collect();

    for (_, entry) in entries {
        for tag in &entry.tags {
            let t = tag.trim().to_lowercase();
            if t.len() >= 3 {
                *tag_counts.entry(t).or_default() += 1;
            }
        }
    }

    tag_counts
        .into_iter()
        .filter(|(tag, count)| *count >= 3 && !titles_lower.contains(tag))
        .map(|(tag, _)| tag)
        .take(15)
        .collect()
}
