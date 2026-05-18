# rsut-pdf-mcp Architecture

AI-native knowledge compilation engine — PDF extraction + Karpathy compiler pattern + fulltext search + knowledge graph + vector embeddings. Pure Rust, single binary.

## Design Principles

1. **Karpathy Compiler Mode**: Knowledge pre-compiled to structured Markdown. Markdown is the single source of truth.
2. **AI-Agent as UI**: No external GUI. All interaction via MCP tool calls from AI clients.
3. **Rebuildable Indexes**: All indexes (Tantivy, petgraph, TF-IDF vectors) can be fully reconstructed from wiki Markdown files. Zero data risk.
4. **FFI Safety**: `catch_unwind` levee isolates C++ pdfium panics from Rust.
5. **Pure Rust**: Single binary, zero external services, no database.
6. **Dual-Protocol**: stdio (MCP) + HTTP (Wiki), oneshot signal for reliable co-bootstrap.
7. **Breakwater Architecture**: Facade/Core layered separation — pdf-mcp absorbs protocol chaos, pdf-core maintains deterministic extraction.

> **Note**: 根目录 [`DESIGN.md`](/opt/pdf-module/DESIGN.md) 为 UI 设计系统文档（色彩、排版、组件规范），与本软件架构文档互补但不重叠。软件架构变更不影响 UI 设计系统，反之亦然。

## Architecture

```
┌──────────────────────────────────────────────────┐
│            AI Client (Claude / Cursor)            │
│            23 MCP tools via JSON-RPC              │
└──────────────┬───────────────┬───────────────────┘
               │ stdio         │ HTTP
               ▼               ▼
┌──────────────────────┐ ┌──────────────────┐
│   pdf-mcp (server)   │ │  Wiki HTTP       │
│   JSON-RPC dispatch  │ │  axum + embed    │
├──────────────────────┤ └──────────────────┘
│                      │
│  ┌── PDF Extraction (6) ─────────────────┐
│  │  extract_text / extract_structured     │
│  │  get_page_count / search_keywords      │
│  │  extrude_to_server_wiki                │
│  │  extrude_to_agent_payload              │
│  └────────────────────────────────────────┘
│
│  ┌── Knowledge Engine (7) ───────────────┐
│  │  compile_to_wiki / incremental_compile │
│  │  recompile_entry / aggregate_entries   │
│  │  check_quality / micro_compile         │
│  │  hypothesis_test                       │
│  └────────────────────────────────────────┘
│
│  ┌── Cognitive Index (6) ────────────────┐
│  │  search_knowledge (Tantivy + CJK)     │
│  │  rebuild_index                         │
│  │  get_entry_context / find_orphans      │
│  │  suggest_links / export_concept_map    │
│  └────────────────────────────────────────┘
│
│  ┌── Management (5) ─────────────────────┐
│  │  get_config / set_config               │
│  │  get_health_report                     │
│  │  trigger_incremental_compile           │
│  │  get_compile_status / show_wiki_browser│
│  └────────────────────────────────────────┘
│
│  ┌── Resources (2) ──────────────────────┐
│  │  dashboard (rust_embed)                │
│  │  wiki-browser (rust_embed)             │
│  └────────────────────────────────────────┘
└──────────────────────┬──────────────────────┘
                       │
        ┌──────────────┴──────────────┐
        ▼                             ▼
┌───────────────┐         ┌───────────────────┐
│  PdfiumEngine │         │  VlmGateway       │
│  (local)      │         │  (conditional)    │
│  FFI levee    │         │  GPT-4o / Claude  │
└───────────────┘         │  GLM-4.6v / OCR  │
                          └───────────────────┘
         │                        │
         ▼                        ▼
┌──────────────────────────────────────────────────┐
│              Knowledge Engine                    │
│  Tantivy │ petgraph │ TF-IDF Vector │ bincode   │
│  hash_cache │ cache_db │ community │ tokenizer  │
└──────────────────────────────────────────────────┘
```

## Breakwater Layers

```
┌──────────────────────────────────────────────────────┐
│  Application Layer                                    │
│  pdf-cli (CLI entry)                                  │
│  pdf-mcp main.rs (entry point + HTTP co-bootstrap)    │
├──────────────────────────────────────────────────────┤
│  Facade Layer (MCP)                                   │
│  pdf-mcp server.rs (protocol dispatch)                │
│  pdf-mcp tools/*.rs (handler functions → anyhow)      │
│  pdf-mcp sampling/ (VLM sampling client)              │
│  pdf-mcp http.rs (HTTP server with oneshot signal)    │
├──────────────────────────────────────────────────────┤
│  Facade Layer (Web UI)                                │
│  pdf-mcp http.rs + embed.rs (Vue3 SPA via rust_embed)  │
│  pdf-web (deprecated legacy management sidecar)       │
├──────────────────────────────────────────────────────┤
│  Core Layer                                           │
│  pdf-core (extraction, knowledge, parallel)           │
│  pdf-common (shared types, errors, DTOs)              │
│  pdf-wasm (WASM compilation target, IRON-01~04)       │
│  vlm-visual-gateway (VLM/OCR engine)                  │
├──────────────────────────────────────────────────────┤
│  Metaprogramming Layer                                │
│  pdf-macros (derive macros)                           │
└──────────────────────────────────────────────────────┘
```

See [FEATURE_MATRIX.md](doc/FEATURE_MATRIX.md) for Cargo feature status.

## Knowledge Base Layout

```
knowledge_base/
├── raw/                   # Source PDFs + extraction markdown
├── wiki/                  # Compiled knowledge (Markdown)
│   ├── index.md           # Auto-generated navigation
│   ├── log.md             # Operation log
│   ├── .versions/         # Backup before recompile (v{N}.md)
│   └── <domain>/          # L1/L2/L3 entries
├── schema/                # Compilation instructions
├── .hash_cache/           # Merkle hash for incremental compile (JSON)
├── .rsut_index/           # Rebuildable indexes
│   ├── tantivy/           # Fulltext search index
│   └── graph.bin          # Knowledge graph persistence (bincode)
└── .cache_db/             # Entry cache
```

## Crates

| Crate | 职责 | 关键能力 |
|-------|------|----------|
| `pdf-common` | 统一错误、DTO、配置、traits | `PdfError` (23 variants), `ToolContext`, `AppConfig` |
| `pdf-macros` | 过程宏 | `#[derive(Builder)]` |
| `pdf-core` | PdfiumEngine + FileValidator + VlmPipeline + **KnowledgeEngine** + **FulltextIndex** + **GraphIndex** + **VectorIndex** | TF-IDF embedding, batch_embed_all, community detection |
| `pdf-mcp` | MCP stdio + HTTP 入口 (JSON-RPC) — 23 tools | `tokio::select!` dispatch, oneshot HTTP bootstrap, resources protocol |
| `pdf-cli` | 统一 CLI (双模式: local/remote) | `clap` derive, `reqwest` for remote, file upload, knowledge management |
| `pdf-web` | **已弃用** 管理 API sidecar (`axum`) | 请使用 `pdf-mcp`（Wiki + 管理 API + 内嵌 `pdf-web-ui`） |
| `pdf-wasm` | WASM 引擎 | `WasmSlice` zero-copy, `bumpalo` arena, `talc` allocator |
| `vlm-visual-gateway` | VLM 条件升级网关 | `catch_unwind` FFI levee, Semaphore rate-limiting, exponential backoff |

## Knowledge Engine Module (`pdf-core::knowledge`)

| 子模块 | 职责 | 关键类型 |
|--------|------|----------|
| `entry` | 统一 front matter 规范 | `KnowledgeEntry`, `EntryLevel`, `CompileStatus` |
| `hash_cache` | Merkle 增量变更检测 | `HashCache` |
| `engine` | 编译调度核心 | `KnowledgeEngine`, `CompileResult`, `RecompileResult`, `CollectContext`, `AggregationCandidate` |
| `renderer` | Markdown → HTML 渲染 | `RenderedEntry`, `TreeNode` |
| `cache_db` | 条目缓存 | `CacheDb` |
| `index::fulltext` | Tantivy 全文检索 (CJK-aware) | `FulltextIndex`, `SearchHit` |
| `index::graph` | petgraph 知识图谱 + 磁盘持久化 | `GraphIndex`, `GraphSnapshot`, `NeighborInfo`, `LinkSuggestion` |
| `index::vector` | TF-IDF 向量嵌入 | `VectorIndex`, `IndexedEntry` |
| `index::community` | 标签社区检测 | `LouvainCluster` |
| `index::tokenizer` | CJK n-gram 分词器 | `register_cjk_tokenizer()` |

## MCP Tool Inventory (23 tools)

### PDF Extraction (6)
| Tool | Description |
|------|-------------|
| `extract_text` | 纯文本提取 |
| `extract_structured` | 结构化提取 (per-page + bbox) |
| `get_page_count` | 页数查询 |
| `search_keywords` | 关键词搜索 (正则 + 二分页定位) |
| `extrude_to_server_wiki` | 提取到 server wiki |
| `extrude_to_agent_payload` | 提取 + 返回 Agent payload 到对话 |

### Compilation (7)
| Tool | Description |
|------|-------------|
| `compile_to_wiki` | PDF → raw/ + 编译提示 (知识库入口) |
| `incremental_compile` | Merkle 哈希增量扫描，只编译变更的 PDF |
| `recompile_entry` | 单条目重编译 + 版本备份 + 漂移检测 |
| `aggregate_entries` | L1→L2 聚合候选发现 (标签社区检测) |
| `check_quality` | 全 wiki 质量扫描 |
| `micro_compile` | 即时 PDF 提取 (不写 wiki，注入对话) |
| `hypothesis_test` | 矛盾对发现 + 辩论框架生成 |

### Indexing (6)
| Tool | Description |
|------|-------------|
| `search_knowledge` | Tantivy 全文搜索 (CJK n-gram) |
| `rebuild_index` | 完全重建 Tantivy + petgraph + vector |
| `get_entry_context` | N 跳邻居发现 |
| `find_orphans` | 孤立条目检测 |
| `suggest_links` | Jaccard 相似度链接建议 |
| `export_concept_map` | Mermaid.js 概念图导出 |

### Management (5)
| Tool | Description |
|------|-------------|
| `get_config` | 获取当前配置 |
| `set_config` | 更新配置项 |
| `get_health_report` | 系统健康报告 |
| `trigger_incremental_compile` | 批量增量编译 |
| `get_compile_status` | 编译状态查询 |
| `show_wiki_browser` | Wiki 浏览器入口 |

### Resources (2)
| Resource | Description |
|----------|-------------|
| `rust-pdf://dashboard` | 嵌入式仪表板 (rust_embed) |
| `rust-pdf://wiki-browser` | 嵌入式 Wiki 浏览器 (rust_embed) |

## Entry Format (YAML Front Matter)

```yaml
---
title: "概念名称"
domain: "IT"
source: "raw/paper.pdf"
page: 3
source_hash: "abc123..."
tags: ["http", "networking"]
level: L1
status: compiled
quality_score: 0.85
version: 1
contradictions: ["wiki/other/concept.md"]
related: ["wiki/it/related_concept.md"]
aggregated_from: []
created: 2026-05-04T00:00:00Z
updated: 2026-05-04T00:00:00Z
---
```

## Knowledge Pyramid

```
L3  Domain Map         (导航层，1 per domain)
    ↑ aggregated from
L2  Aggregation         (综述，同子主题多 L1 合并)
    ↑ aggregated from
L1  Atomic Concept      (原子概念，核心知识单元)
    ↑ compiled from
L0  Raw Extraction      (原始提取，PDF → text)
```

## Key Dependencies

| Library | Version | Purpose |
|---------|---------|---------|
| `tantivy` | 0.22 | Full-text search index |
| `petgraph` | 0.7 | Knowledge graph (link analysis) |
| `pdfium-render` | 0.8 | PDF text extraction (FFI) |
| `sha2` | 0.10 | Content hashing (incremental compile) |
| `serde_yaml` | 0.9 | Front matter serialization |
| `bincode` | 1.3 | Graph index disk persistence |
| `bumpalo` | 3.16 | Arena allocation (WASM + pixel buffers) |
| `tokio` | 1.x | Async runtime |
| `axum` | 0.7 | HTTP server (wiki browsing) |
| `rust-embed` | 8 | Embedded static assets |
| `memmap2` | 0.9 | Zero-copy PDF loading |
| `pulldown-cmark` | 0.11 | Markdown → HTML rendering |

## FFI Levee

```rust
pub fn safe_extract_text(data: &[u8]) -> PdfResult<String> {
    catch_unwind(AssertUnwindSafe(|| {
        // pdfium C++ FFI
    }))
    .map_err(|_| PdfModuleError::Extraction("FFI panic".into()))?
    .map_err(|e| PdfModuleError::Extraction(format!("Pdfium: {}", e)))
}
```

## VLM Gateway

```rust
// Semaphore rate-limiting + exponential backoff + retry classification
let permit = gateway.semaphore.acquire().await?;
let result = retry_loop(gateway, request).await;
drop(permit); // release before metrics to avoid holding semaphore
gateway.metrics.observe(result);
```

## WASM Compliance

| Rule | Status |
|------|--------|
| IRON-01 Binary size | `opt-level='z'`, `lto`, `codegen-units=1`, `panic='abort'`, `strip` |
| IRON-02 Zero-copy boundary | `WasmSlice` + `OwnedSlice` safe encapsulation |
| IRON-03 Memory partitioning | Arena per-frame lifecycle (`bumpalo` + `reset()`) |
| IRON-04 Cross-origin isolation | N/A (no SharedArrayBuffer) |
| Forbid unsafe_op | `#![forbid(unsafe_op_in_unsafe_fn)]` |
| No wee_alloc | Uses `talc::TalckWasm` |