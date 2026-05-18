# ADR-002: Hybrid search with RRF and no silent FS fallback on API

- **Status**: accepted
- **Date**: 2026-05-19
- **Deciders**: PDF MCP team

## Context

Search must combine lexical (Tantivy CJK), vector (TF-IDF/jieba), and graph signals. Some deployments previously fell back to filesystem grep when indexes were empty, hiding operational problems.

## Decision

- Default mode is `SearchMode::Hybrid` (Tantivy + vectors + RRF fusion).
- HTTP and MCP use `SearchOptions::for_api()` with `allow_fs_fallback: false`.
- CLI may use `rebuild_if_empty`; FS fallback only when `RSUT_SEARCH_ALLOW_FS_FALLBACK=1`.
- Responses include `meta`: `mode`, `index_empty`, `used_fallback`.

## Consequences

- Empty indexes surface via OpsBanner / `index_empty` instead of silent grep.
- Operators must run index rebuild explicitly.
- Contract tests can assert `used_fallback == false` on API paths.

## Alternatives considered

| Alternative | Pros | Cons |
|-------------|------|------|
| Keyword-only default | Simpler | Weaker recall on Chinese |
| Always FS fallback | Works without index | Masks misconfiguration |
