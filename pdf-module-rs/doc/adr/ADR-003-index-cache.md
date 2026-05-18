# ADR-003: IndexCache for per-KB lazy indexes

- **Status**: accepted
- **Date**: 2026-05-19
- **Deciders**: PDF MCP team

## Context

Opening Tantivy, graph, and vector indexes on every HTTP search request added latency and file handle churn. Multiple KBs (`WorkspaceRegistry`) require isolated index state per knowledge base path.

## Decision

- `pdf_core::knowledge::IndexCache` holds per-KB `KbIndexes` (fulltext, graph, vectors) loaded lazily.
- `pdf-mcp` HTTP handlers search via `HttpState.index_cache`.
- `POST /api/index/rebuild` and `rebuild_all` invalidate or refresh cached indexes for that KB.
- MCP tools share the same cache instance in `ToolContext`.

## Consequences

- Faster repeat searches within a process lifetime.
- Memory grows with number of active KBs; acceptable for typical desktop/single-tenant deployments.
- Rebuild must go through management APIs to avoid stale cache.

## Alternatives considered

| Alternative | Pros | Cons |
|-------------|------|------|
| Open index per request | Always fresh | Slow, heavy I/O |
| Global single index | Simple | Breaks multi-KB |
