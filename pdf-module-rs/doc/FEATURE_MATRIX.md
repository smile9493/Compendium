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

- `pdf_core::knowledge::{search, graph, rebuild_all}` for indexes
- `pdf_core::management::CompileStatusStore` for `.rsut_index/compile_status.json`

## Web UI

| Component | Status |
|-----------|--------|
| `pdf-mcp` + embedded `pdf-web-ui` (Vue 3) | **canonical** |
| `pdf-web` binary | **deprecated** — management API sidecar only |
