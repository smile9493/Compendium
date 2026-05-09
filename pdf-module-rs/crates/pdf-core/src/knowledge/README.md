# Knowledge Engine Module

AI-native knowledge compilation and reasoning engine for `pdf-core`.

## Overview

This module implements the **Karpathy compiler pattern**: raw PDF content is pre-compiled into structured Markdown files, which serve as the single source of truth for all knowledge operations. All indexes (Tantivy fulltext, petgraph graph) are derived from these files and can be fully rebuilt at any time.

## Module Structure

```
knowledge/
├── mod.rs              # Module root, re-exports
├── entry.rs            # KnowledgeEntry — standardized YAML front matter
├── hash_cache.rs       # Merkle hash cache for incremental compilation
├── engine.rs           # KnowledgeEngine — compilation orchestrator
├── quality.rs          # Wiki quality analysis
└── index/
    ├── mod.rs          # Index layer root
    ├── fulltext.rs     # Tantivy full-text search (CJK-aware)
    ├── graph.rs        # petgraph knowledge graph
    └── tokenizer.rs    # CJK n-gram tokenizer for Tantivy
```

## Quick Start

```rust
use pdf_core::knowledge::{KnowledgeEngine, FulltextIndex, GraphIndex};
use std::sync::Arc;

// 1. Create engine
let pipeline = Arc::new(McpPdfPipeline::new(&config)?);
let engine = KnowledgeEngine::new(pipeline, "/path/to/knowledge_base")?;

// 2. Compile a PDF
let result = engine.compile_to_wiki(Path::new("/path/to/paper.pdf"), Some("IT")).await?;

// 3. Search the knowledge base
let idx = FulltextIndex::open_or_create(engine.knowledge_base())?;
idx.rebuild(&engine.wiki_dir())?;
let hits = idx.search("HTTP/2", 10)?;

// 4. Discover relationships
let mut graph = GraphIndex::new();
graph.rebuild(&engine.wiki_dir())?;
let neighbors = graph.get_neighbors("it/http2.md", 2);
let orphans = graph.find_orphans();
let map = graph.export_concept_map("it/http2.md", 2);
```

## Entry Format

Every wiki file uses standardized YAML front matter:

```yaml
---
title: "概念名称"
domain: "IT"
source: "raw/paper.pdf"
page: 3
source_hash: "sha256..."
tags: ["tag1", "tag2"]
level: L1          # L0=raw, L1=atomic, L2=aggregation, L3=domain map
status: compiled   # pending, compiling, compiled, needs_recompile, failed
quality_score: 0.85
version: 1
contradictions: []
related: []
aggregated_from: []
created: 2026-05-04T00:00:00Z
updated: 2026-05-04T00:00:00Z
---

# Body content here...
```

## Compilation Pipeline

```
PDF → compile_to_wiki → raw/ + compile_prompt.md
                          ↓
                     AI Agent reads prompt
                          ↓
                     Creates L1 entries in wiki/<domain>/
                          ↓
                     aggregate_entries finds L1 clusters
                          ↓
                     AI Agent creates L2 summaries
                          ↓
                     recompile_entry handles quality drift
```

## Incremental Compilation

The `HashCache` tracks SHA-256 hashes of source PDFs:

```rust
let mut cache = HashCache::load_or_create(&knowledge_base)?;
if cache.needs_compile(&pdf_path)? {
    // PDF changed since last compile
    cache.record_compile(&source_path, vec!["wiki/it/concept.md".into()])?;
    cache.save()?;
}
```

## Indexing

### Fulltext Search (Tantivy)

- Uses CJK n-gram tokenizer (unigrams + bigrams) for Chinese support
- Indexes: title, body, tags, domain
- Stored at `.rsut_index/tantivy/`
- Fully rebuildable from wiki files

### Knowledge Graph (petgraph)

- Directed graph: entries as nodes, `related`/`contradictions` as edges
- Tag co-occurrence edges (Jaccard ≥ 0.3) for weak relation discovery
- Supports: N-hop neighbors, orphan detection, link suggestion, concept map export

## Quality Analysis

`analyze_wiki()` scans all entries for:
- Missing title, domain, or tags
- Zero quality score
- Orphan entries (no links in or out)
- Broken links (referenced paths that don't exist on disk)

## Future Upgrades

| Feature | Current | Upgrade Path |
|---------|---------|--------------|
| CJK Tokenization | N-gram (character-level) | Add `jieba-tantivy` for word-level segmentation |
| Vector Search | None | Add `ort` (ONNX) + `usearch` for semantic similarity |
| Persistent Graph | Rebuilt from files | Add `sled` for serialized graph cache |
