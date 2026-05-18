# Release checklist (manual)

Run before tagging a release. Record **pass/fail**, `kb_id`, browser, and commit SHA.

## Environment

- [ ] `pdf-mcp` built from target commit (not stale `main`)
- [ ] At least one KB registered in `~/.rsut/workspaces.toml`
- [ ] PDFium / extraction stack healthy (`GET /api/health`)

## Paths

1. **Multi-KB** — Header workspace switcher changes tree/search scope for the active `kb_id`.
2. **Open entry** — Tree click → URL `#/wiki/<path>` matches store `currentPath` → browser refresh keeps the same entry.
3. **Hybrid search** — With a built index, search returns hits; with empty index, OpsBanner shows and response `meta.index_empty` is true; `meta.used_fallback` is false for HTTP API.
4. **Compile** — Upload or incremental compile → compile drawer shows stages → SSE or poll matches `GET /api/compile/status`.
5. **Share** — Copy share link from entry → open in private window `#/share/<token>/<path>` → read-only body renders.
6. **MCP iframe** — Embed or `?mcp=1` → “Ask AI” sends `mcp-ask-ai` postMessage (verify in host devtools).

## Sign-off

| Role | Name | Date | Result |
|------|------|------|--------|
| | | | |
