# CLAUDE.md

> AI-native PDF knowledge compilation engine. Version 0.5, Rust 2024 Edition workspace.

## Project Identity

PDF MCP Module is an MCP (Model Context Protocol) server that compiles PDF documents into structured AI knowledge bases — a long-term memory & reasoning backend for Claude, Cursor, and other AI clients.

**Core pipeline**: PDF → extraction (Pdfium/VLM/Hybrid) → structured Markdown → Tantivy + petgraph + TF-IDF indexing → AI queryable knowledge.

**Architecture**: [Breakwater Pattern](ARCHITECTURE.md) — Facade (pdf-mcp, vlm-visual-gateway) / Core (pdf-core) / Infra (pdf-wasm, pdf-macros) layered isolation.

---

## Mandatory Skills — Auto-Invoke on Every Rust Task

Before generating, modifying, or reviewing **any Rust code** in this project, you MUST invoke the following skills. These are the constitutional foundation of all engineering decisions.

### Primary: `rust-architecture-guide` (Universal Constitution)

**Invoke immediately when:**
- Starting any Rust coding task, no matter how small
- Making trade-off decisions (P0 Safety vs P3 Performance)
- Designing new modules, types, or APIs
- Refactoring existing code
- Choosing between `thiserror`/`anyhow`, `Box<dyn Trait>`/generics, `clone()`/lifetimes

**Key rules always active for this project:**
- **Execution mode**: `standard` (P0+P1 enforced, P2 warned)
- **Error handling**: Library crates → `thiserror`; binary entry → `anyhow` + `.context()`
- **Ownership**: Business layer → Owned + `.clone()` unbundled; hotpath → `Cow`/`Bytes` zero-copy
- **Concurrency**: Bounded channels only; `Mutex` never held across `.await`; `parking_lot` preferred
- **API evolution**: `#[non_exhaustive]` on all public structs/enums; Sealed Trait for internal stability
- **Memory layout**: `#[repr(C)]` on FFI types; 64-byte cache-line alignment for shared hot data
- **Physical audit**: I/O budget tracked; memory ceiling with 20% margin; concurrency contention <20%

### Secondary: `rust-wasm-frontend-infra-guide` (WASM Boundary)

**Invoke when working on:**
- `crates/pdf-wasm/` — WASM compilation target
- Any `wasm32-unknown-unknown` target code
- FFI boundaries between Rust and JavaScript

**Key constraints:**
- **IRON-01**: Binary size paramount — `opt-level="z"`, `lto=true`, `wasm-opt -Oz`
- **IRON-02**: Zero-copy at boundary — `WasmSlice` pattern, no serialization on hot paths
- **IRON-03**: Memory partitioning — global static residency + per-frame Arena (`bumpalo` + `reset()`)
- **IRON-04**: Cross-origin isolation documented for `SharedArrayBuffer`

### Tertiary: `rust-systems-cloud-infra-guide` (Server Infra)

**Invoke when working on:**
- `crates/vlm-visual-gateway/` — HTTP gateway with Prometheus metrics
- `crates/pdf-mcp/` — long-running MCP server
- Graceful shutdown, backpressure, observability

---

## Module → Skill Mapping

| Crate | Target | Primary Skill | Key Concerns |
|-------|--------|---------------|--------------|
| `pdf-common` | lib | rust-architecture-guide | Shared DTOs, Error types, Traits |
| `pdf-macros` | proc-macro | rust-architecture-guide | Derive macros, Span diagnostics |
| `pdf-core` | lib | rust-architecture-guide | Extraction pipeline, knowledge engine, FFI |
| `pdf-mcp` | bin (server) | rust-architecture-guide + cloud-infra | MCP JSON-RPC, sampling, tools, observability |
| `pdf-cli` | bin (CLI) | rust-architecture-guide | Health check, config, compile commands |
| `vlm-visual-gateway` | lib | rust-architecture-guide + cloud-infra | VLM HTTP API, OCR, Prometheus metrics |
| `pdf-wasm` | lib (WASM) | rust-architecture-guide + wasm-frontend | WASM exports, Arena, slice encoding |

---

## Project-Specific Conventions

### Lint Level (inherited from each crate's `lib.rs`)
```rust
#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(clippy::all)]
#![deny(clippy::await_holding_lock)]
#![deny(clippy::large_stack_frames)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(clippy::todo)]
#![deny(clippy::dbg_macro)]
```

### Naming & Structure
- Module files: `kebab-case` for multi-word (e.g., `quality_probe.rs`, `hash_cache.rs`)
- Type names: `PascalCase`, descriptive (e.g., `McpPdfPipeline`, `KnowledgeEngine`)
- Every `pub` type has a doc comment referencing Python origin: `/// Corresponds to Python: X2TextAdapter`
- Module doc comments describe architecture role: `//! # Knowledge Engine`
- Comments only where non-obvious; no redundant comments on self-documenting code

### Error Handling
- All errors go through `PdfModuleError` enum in `pdf-common`
- `ErrorCategory` for classification (Io, Extraction, Knowledge, etc.)
- `PdfResult<T>` alias = `Result<T, PdfModuleError>`

### Testing
- Unit tests in same file as implementation (`#[cfg(test)] mod tests`)
- Test naming: `test_<function>_<scenario>` or `test_<behavior>`
- Integration tests in `tests/` directory

### Commit & Versioning
- Version in `VERSION` file at repo root
- Semantic commits preferred
- No compiled binaries in repo (ELF, WASM artifacts)

---

## Output Contract — Mandatory After Every Code Change

After generating or modifying Rust code, append a **Decision Summary** block:

```markdown
## Decision Summary
- **Mode**: standard
- **Edition**: Rust 2024
- **Rules Applied**: [P0-P3 rules used]
- **Conflicts Resolved**: [priority judgments or "None"]
- **Deviations**: [`// DEVIATION: reason` or "None"]
- **Trade-offs**: [key decisions]
```

---

## Quick Reference: When to Use Which Pattern

| Situation | Pattern | Reference |
|-----------|---------|-----------|
| Multiple boolean flags accumulating | Enum state machine | rust-architecture-guide §Typical Refactoring Paths |
| `String` storage on hot path | `Cow<'a, str>` or `Arc<str>` | rust-architecture-guide §Ownership Strategy |
| Manual `for` loop over collection | Iterator adapter chain | rust-architecture-guide §Idiomatic Style |
| Library returning errors | `thiserror` derive + context | rust-architecture-guide §Error Handling |
| Binary propagating errors | `anyhow::Result` + `.context()` | rust-architecture-guide §Error Handling |
| PDF FFI boundary (pdfium-render) | Opaque handles + `catch_unwind` | rust-architecture-guide §FFI Safety |
| WASM JS↔Rust data transfer | `WasmSlice` zero-copy | rust-wasm-frontend-infra-guide §FFI Boundary |
| Long-running async server method | `#[tracing::instrument]` + bounded channels | rust-architecture-guide §Observability |
| Struct with cross-crate stability | `#[non_exhaustive]` | rust-architecture-guide §API Evolution |