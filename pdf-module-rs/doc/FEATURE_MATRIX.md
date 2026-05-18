# Feature Matrix

Cargo features and their production status in the PDF MCP workspace.

| Feature | Crate | Default | Status | Consumers |
|---------|-------|---------|--------|-----------|
| `knowledge` | pdf-core | yes | **stable** | pdf-mcp, pdf-cli, pdf-web |
| `vlm` | pdf-core | yes | **stable** | pdf-core extractor (optional VLM path) |
| `dhat-heap` | pdf-core | no | dev | profiling only |
| `diagnostics` | pdf-common | no | scaffold | none |
| `config-loader` | pdf-common | no | scaffold | none |
| `i18n` | pdf-common | no | scaffold | none |

## Removed (Phase 1)

The following `pdf-common` features were removed as unused scaffolding:

- `auth` (JWT)
- `crypto` (password encryption)
- `database` (sqlx migrations)

## Index and compile status (unified)

All entry points use:

- `pdf_core::knowledge::{search, search_with_mode, graph, rebuild_all, reindex_entry}` for indexes
  - **Hybrid search** (default): Tantivy CJK + TF-IDF/jieba vectors + RRF (`SearchMode::Hybrid`)
- `pdf_core::management::CompileStatusStore` for `.rsut_index/compile_status.json`
- `pdf_core::management::QualitySnapshotStore` for `.rsut_index/quality_snapshot.json`

## Agent tools (Phase 2)

| Tool | Purpose |
|------|---------|
| `get_agent_context` | Token-budget context bundle (center + neighbors + related) |
| `preview_wiki_patch` | Diff preview without write |
| `patch_wiki_entry` | Structured patch + `reindex_entry` |

## Web UI

| Component | Status |
|-----------|--------|
| `pdf-mcp` + embedded `pdf-web-ui` (Vue 3) | **canonical** |
| `pdf-web` binary | **deprecated** — management API sidecar only |
