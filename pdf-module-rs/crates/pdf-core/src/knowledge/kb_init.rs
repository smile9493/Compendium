//! Initialize a new knowledge base from embedded Karpathy-style templates.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{PdfModuleError, PdfResult};

const TEMPLATE_FILES: &[(&str, &str)] = &[
    ("schema/AGENTS.md", include_str!("../../../../templates/knowledge_base/schema/AGENTS.md")),
    ("schema/CLAUDE.md", include_str!("../../../../templates/knowledge_base/schema/CLAUDE.md")),
    ("wiki/index.md", include_str!("../../../../templates/knowledge_base/wiki/index.md")),
    ("wiki/log.md", include_str!("../../../../templates/knowledge_base/wiki/log.md")),
    ("raw/.gitkeep", include_str!("../../../../templates/knowledge_base/raw/.gitkeep")),
];

/// Result of initializing a knowledge base directory.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InitKnowledgeBaseResult {
    pub knowledge_base: PathBuf,
    pub created_files: Vec<String>,
    pub skipped_files: Vec<String>,
}

/// Create `raw/`, `wiki/`, `schema/` and seed template files (no overwrite).
pub fn init_knowledge_base(path: impl AsRef<Path>) -> PdfResult<InitKnowledgeBaseResult> {
    let kb = path.as_ref().to_path_buf();
    if kb.exists()
        && kb
            .read_dir()
            .map_err(|e| PdfModuleError::Storage(format!("read kb dir: {}", e)))?
            .filter_map(|e| e.ok())
            .any(|e| {
                let n = e.file_name();
                let s = n.to_string_lossy();
                s != "." && s != ".."
            })
    {
        return Err(PdfModuleError::Storage(format!(
            "Knowledge base path is not empty: {}",
            kb.display()
        )));
    }

    fs::create_dir_all(kb.join("raw"))
        .map_err(|e| PdfModuleError::Storage(format!("create raw: {}", e)))?;
    fs::create_dir_all(kb.join("wiki"))
        .map_err(|e| PdfModuleError::Storage(format!("create wiki: {}", e)))?;
    fs::create_dir_all(kb.join("schema"))
        .map_err(|e| PdfModuleError::Storage(format!("create schema: {}", e)))?;
    fs::create_dir_all(kb.join("wiki/.versions"))
        .map_err(|e| PdfModuleError::Storage(format!("create .versions: {}", e)))?;

    let mut created = Vec::new();
    let mut skipped = Vec::new();

    for (rel, content) in TEMPLATE_FILES {
        let dest = kb.join(rel);
        if dest.exists() {
            skipped.push(rel.to_string());
            continue;
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| PdfModuleError::Storage(format!("create parent: {}", e)))?;
        }
        fs::write(&dest, content)
            .map_err(|e| PdfModuleError::Storage(format!("write {}: {}", rel, e)))?;
        created.push(rel.to_string());
    }

    Ok(InitKnowledgeBaseResult {
        knowledge_base: kb,
        created_files: created,
        skipped_files: skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_creates_schema_and_wiki() {
        let dir = std::env::temp_dir().join(format!("kb_init_{}", uuid::Uuid::new_v4()));
        let r = init_knowledge_base(&dir).unwrap();
        assert!(r.created_files.iter().any(|f| f.contains("AGENTS.md")));
        assert!(dir.join("schema/AGENTS.md").exists());
        assert!(dir.join("wiki/index.md").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
