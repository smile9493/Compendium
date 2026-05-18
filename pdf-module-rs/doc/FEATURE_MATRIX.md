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

## Phase 3 (platformization)

| Capability | Status |
|------------|--------|
| Multi-KB `WorkspaceRegistry` (`~/.rsut/workspaces.toml`) | **stable** |
| `kb_id` on MCP / HTTP / Web UI | **stable** |
| `ExtractionRouter` + remote plugins (`extraction.plugins.toml`) | **stable** |
| `pdf-wasm` preview (`open`, thumbnails, page text) | **stable** |
| Local collab (audit, patch proposals, locks) | **stable** |
| Git-like sync (`file://` remote, Merkle manifest) | **stable** |

## HTTP / Web UI capabilities

| Capability | Status | Notes |
|------------|--------|-------|
| Compile SSE `GET /api/compile/events` | **stable** | Same JSON shape as compile status; MCP `mcp-compile-status` in [MCP_UI_EXTENSION.md](MCP_UI_EXTENSION.md) |
| `IndexCache` (in-process indexes) | **stable** | `pdf-mcp` HTTP + `pdf_core::knowledge::IndexCache` |
| Ops console (Settings + OpsBanner + index rebuild) | **stable** / **partial** | Health/rebuild **stable**; some header tooltips still Chinese-only |
| `body_markdown` SSOT | **stable** | Server does not emit `body_html`; UI errors when body missing |
| MCP UI v1 (postMessage) | **stable** | `mcp-ask-ai`, `mcp-compile-status` |
| Share read-only links | **stable** | `POST /api/v1/shares`, `#/share/:token/:path` |
| Collaboration proposal UI | **MCP-only** | `submit` / `apply` / `list_patch_proposals`; no Web UI approval panel |
| OpenAPI → TypeScript types | **partial** | CI checks management paths; wiki types in `types.d.ts` |
| UI i18n | **partial** | Settings, OpsBanner, compile status keys covered; drawer/rightbar progressive |

### Wiki write paths (collaboration)

| Path | Mechanism |
|------|-----------|
| Direct write | MCP `patch_wiki_entry`, `save_wiki_entry` |
| Proposal write | MCP `submit_patch_proposal` → `apply_patch_proposal`; discover via `list_patch_proposals` |

## Web UI

| Component | Status |
|-----------|--------|
| `pdf-mcp` + embedded `pdf-web-ui` (Vue 3) | **canonical** |
| KB workspace switcher in header | **stable** |
| `pdf-web` binary | **deprecated** — management API sidecar only |
