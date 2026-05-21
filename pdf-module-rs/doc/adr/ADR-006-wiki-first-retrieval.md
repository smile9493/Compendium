# ADR-006: Wiki-first retrieval mode

- **Status**: accepted
- **Date**: 2026-05-21
- **Related**: [ADR-002](ADR-002-hybrid-search-rrf.md)

## Context

Karpathy LLM Wiki uses `index.md` and linked pages for query at medium scale (~400k words), without vector RAG. Compendium already ships Tantivy + TF-IDF hybrid search (ADR-002).

## Decision

Add `SearchMode::WikiFirst` and config key `retrieval_mode=wiki_first`:

- Query flow: parse `wiki/index.md`, title/body match, optional graph context
- Does not open Tantivy or vector indexes
- Default remains `hybrid`; Karpathy-style users opt in via config or `search_knowledge` `mode`

## Consequences

- Complements hybrid search; does not replace it
- `lint_wiki` and `sync_nervous_system` keep `index.md` accurate for wiki-first quality
