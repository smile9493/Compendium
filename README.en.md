# PDF MCP Module

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.95%2B-orange.svg)](https://www.rust-lang.org/)
[![MCP](https://img.shields.io/badge/MCP-2024--11--05-blue.svg)](https://modelcontextprotocol.io/)

**AI-Native Knowledge Compilation Engine** — Compile PDF documents into structured knowledge bases, providing long-term memory and reasoning backend for AI clients like Claude and Cursor.

English | [简体中文](./README.md)

## ✨ Features

- 🔥 **Karpathy Compiler Pattern** — PDFs pre-compiled to structured Markdown, knowledge is cumulative and explainable, with L0→L1→L2→L3 knowledge pyramid
- 🧠 **Cognitive Index Layer** — Tantivy full-text search + petgraph knowledge graph + TF-IDF vector embeddings, three-way retrieval fusion
- 🚀 **Pure Rust** — Single binary deployment, zero external service dependencies, high-performance FFI levee
- 🔄 **Incremental Compilation** — Merkle hash detection, only compile changed PDFs
- 🖼️ **VLM Visual Understanding** — Conditional OCR fallback for scanned/image-based PDFs
- 🌐 **Dual-Protocol Server** — stdio (MCP) + HTTP (Wiki browsing), oneshot signal startup
- 🎯 **23 MCP Tools** — Covering PDF extraction, knowledge compilation, cognitive indexing, and resources

## 📦 Installation

### One-line Install

```bash
curl -fsSL https://raw.githubusercontent.com/smile9493/Compendium/main/install.sh | bash
```

### Docker

```bash
docker pull smile9493/pdf-mcp:latest
```

### Build from Source

```bash
git clone https://github.com/smile9493/Compendium.git
cd Compendium/pdf-module-rs
cargo build --release --bin pdf-mcp
```

## 🚀 Quick Start

### 1. Configure AI Client

**Cursor** (`~/.cursor/mcp.json`):

```json
{
  "mcpServers": {
    "pdf-mcp": {
      "command": "/opt/pdf-module/pdf-mcp",
      "env": {
        "PDFIUM_LIB_PATH": "/opt/pdf-module/lib/libpdfium.so"
      }
    }
  }
}
```

**Claude Desktop** (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "pdf-mcp": {
      "command": "/opt/pdf-module/pdf-mcp"
    }
  }
}
```

### 2. Compile PDF to Knowledge Base

```
User: Compile /path/to/paper.pdf into the knowledge base

AI: [Calls compile_to_wiki tool]
PDF compiled to knowledge base:
- Raw extraction: raw/paper.md
- Compile prompt: raw/paper.compile_prompt.md
Please read the extracted content, extract core concepts, and create atomic entries...
```

### 3. Search Knowledge Base

```
User: Search for knowledge about HTTP/2

AI: [Calls search_knowledge tool]
Found 3 related entries:
1. [IT] HTTP/2 Multiplexing (score: 0.92)
2. [IT] HTTP/2 Header Compression (score: 0.85)
3. [Network] HTTP/2 vs HTTP/1.1 Comparison (score: 0.78)
```

## 🛠️ MCP Tools (23)

### PDF Extraction (6)

| Tool | Description |
|------|-------------|
| `extract_text` | Extract plain text from PDF |
| `extract_structured` | Extract structured data (per-page text + bbox) |
| `get_page_count` | Get PDF page count |
| `search_keywords` | Search keywords within PDF (regex support) |
| `extrude_to_server_wiki` | Extract to server-side Wiki |
| `extrude_to_agent_payload` | Return Markdown payload to conversation |

### Knowledge Compilation (7)

| Tool | Description |
|------|-------------|
| `compile_to_wiki` | PDF → knowledge base compilation entry point |
| `incremental_compile` | Incremental compilation (Merkle hash detection) |
| `recompile_entry` | Single entry recompilation + version backup |
| `aggregate_entries` | L1→L2 aggregation candidate discovery |
| `check_quality` | Wiki quality scan (drift/contradiction detection) |
| `micro_compile` | On-demand extraction (not persisted) |
| `hypothesis_test` | Contradiction discovery + debate framework generation |

### Cognitive Index (6)

| Tool | Description |
|------|-------------|
| `search_knowledge` | Tantivy full-text search (CJK tokenizer) |
| `rebuild_index` | Rebuild all indexes |
| `get_entry_context` | N-hop neighbor discovery (graph traversal) |
| `find_orphans` | Orphan entry detection |
| `suggest_links` | Link suggestions (Jaccard similarity) |
| `export_concept_map` | Mermaid.js concept map export |

### Management (5)

| Tool | Description |
|------|-------------|
| `get_config` | Get current configuration |
| `set_config` | Update configuration values |
| `get_health_report` | System health report (engine/index/cache status) |
| `trigger_incremental_compile` | Trigger batch incremental compilation |
| `get_compile_status` | Query compilation task status |
| `show_wiki_browser` | Show Wiki browser entry point |

### Resources (2)

| Resource | Description |
|----------|-------------|
| `rust-pdf://dashboard` | Embedded system dashboard (rust_embed) |
| `rust-pdf://wiki-browser` | Embedded knowledge base browser |

## 🏗️ Architecture

```
┌──────────────────────────────────────────────────┐
│            AI Client (Claude / Cursor)            │
│               23 MCP tools via JSON-RPC           │
└──────────────┬───────────────┬───────────────────┘
               │ stdio         │ HTTP
               ▼               ▼
┌──────────────────────┐ ┌──────────────────┐
│   pdf-mcp (server)   │ │  Wiki HTTP       │
│   JSON-RPC dispatch  │ │  axum + embed    │
├──────────────────────┤ └──────────────────┘
│  Extraction │ Compile│
│  Indexing  │ Manage  │
└──────────────┬───────┘
               │
    ┌──────────┴───────────┐
    ▼                      ▼
┌──────────────┐  ┌──────────────────┐
│ PdfiumEngine │  │ VlmVisualGateway │
│ (FFI levee)  │  │ (Conditional OCR) │
└──────────────┘  └──────────────────┘
         │                │
         ▼                ▼
┌──────────────────────────────────────┐
│         Knowledge Engine             │
│  Tantivy │ petgraph │ TF-IDF Vector  │
│  hash_cache │ cache_db │ bincode     │
└──────────────────────────────────────┘
```

### Breakwater Layers

```
Facade Layer:  pdf-mcp (MCP protocol), vlm-visual-gateway (HTTP facade)
Core Layer:    pdf-core (extraction, knowledge, parallel), pdf-common (shared)
Infra Layer:   pdf-macros (derive macros), pdf-wasm (WASM target)
```

## 📁 Knowledge Base Structure

```
knowledge_base/
├── raw/                   # Raw PDF extractions (YAML)
├── wiki/                  # Compiled knowledge base
│   ├── index.md           # Global navigation
│   ├── log.md             # Operation log
│   ├── .versions/         # Recompile backups
│   └── <domain>/          # Domain entries
├── schema/                # Compilation instructions
├── .hash_cache/           # Merkle hash cache (JSON)
├── .rsut_index/           # Rebuildable indexes
│   ├── tantivy/           # Full-text search index
│   └── graph.bin          # Graph persistence (bincode)
└── .cache_db/             # Entry cache
```

## 📝 Entry Format

```yaml
---
title: "HTTP/2 Multiplexing"
domain: "IT"
source: "raw/rfc7540.pdf"
page: 12
tags: ["http", "networking", "protocol"]
level: L1
status: compiled
quality_score: 0.85
version: 1
contradictions: []
related: ["wiki/it/http1.md"]
created: 2026-05-04T00:00:00Z
updated: 2026-05-04T00:00:00Z
---
# HTTP/2 Multiplexing
Body content...
```

## 🗺️ Knowledge Pyramid

```
L3  Domain Map      (Navigation layer, one per domain)
    ↑ aggregated from
L2  Aggregation      (Summary, multiple L1 on same sub-topic)
    ↑ aggregated from
L1  Atomic Concept   (Atomic concept, core knowledge unit)
    ↑ compiled from
L0  Raw Extraction   (Raw extraction, PDF → text)
```

## ⚙️ Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `PDFIUM_LIB_PATH` | PDFium library path | Auto-detect |
| `VLM_API_KEY` | VLM API key | - |
| `VLM_MODEL` | Model name | `glm-4v-flash` |
| `VLM_ENDPOINT` | API endpoint | Zhipu API |
| `MCP_HTTP_PORT` | HTTP Wiki server port | - (disabled) |
| `KB_PATH` | Knowledge base root path | `./knowledge_base` |

## 📁 Project Structure

```
pdf-module/
├── pdf-module-rs/         # Rust workspace (core engine)
│   ├── crates/
│   │   ├── pdf-common/    # Shared types/errors/DTOs
│   │   ├── pdf-core/      # Extraction/knowledge/parallel engine
│   │   ├── pdf-mcp/       # MCP protocol server
│   │   ├── pdf-wasm/      # WASM compilation target
│   │   ├── pdf-web/       # Web frontend (Yew)
│   │   ├── pdf-cli/       # CLI tool
│   │   ├── pdf-dashboard/ # Dashboard server
│   │   ├── pdf-macros/    # Derive macros
│   │   └── vlm-visual-gateway/ # VLM gateway
│   ├── ARCHITECTURE.md    # Architecture docs
│   └── CHANGELOG.md       # Changelog
├── scripts/               # Test/utility scripts
├── docs/                  # User documentation
│   ├── API_REFERENCE.md
│   ├── VLM_INTEGRATION.md
│   └── ...
├── plugins/               # Third-party MCP plugins
├── pdf-mcp-installer/     # Installer
├── deploy/                # Deployment configs
├── docker/                # Docker images
├── Dockerfile
└── docker-compose.yml
```

## 📄 License

[MIT](LICENSE)