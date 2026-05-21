# Wiki Agent Schema (Karpathy LLM Wiki)

You maintain this knowledge base. Read this file before any ingest, query, or lint operation.

## Three commands (natural language → MCP tools)

Prefer the **meta tools** `ingest`, `query`, and `lint` (single call each). Use atomic tools when you need fine-grained control.

### ingest — compile raw material into wiki

When the user says **ingest** or asks to compile new material:

1. **`ingest`** (recommended) or `compile_to_wiki` / `incremental_compile` — PDFs and `raw/*.md` land in `raw/`
2. `get_compilation_context` — read prompts and existing wiki context
3. Create or update wiki pages via `save_wiki_entry` (one page per atomic concept)
4. For each new raw file, add a **source-summary** page (`entry_type: source-summary`)
5. `complete_compile_job` — rebuild indexes and run quality gate

Ripple rule: update every related concept/entity page; set `related` bidirectionally; mark contradictions in `contradictions`.

### query — answer from the wiki

When the user asks a question:

1. **`query`** (recommended) — index excerpt + `search_knowledge` + top-hit context
2. Or read `wiki/index.md` and `search_knowledge` with `mode: wiki_first` when configured
3. For **full page text** (remote/LAN — no host filesystem), use **`get_wiki_entry`**; use **`get_agent_context`** when you only need a token-budget excerpt + neighbors
4. Synthesize an answer with `[[wikilink]]` citations to entry paths
5. If the answer is durable, call **`archive_answer`** to write an `overview` page

Do not answer from memory alone when the wiki already contains the topic.

### lint — health check

When the user says **lint**:

1. **`lint`** (recommended) — `lint_wiki` + `check_quality` + `find_orphans` + `detect_stale_entries` + load-bearing report
2. Or **`lint_wiki`** alone for a lighter pass
2. Fix high-severity issues via `patch_wiki_entry` / `save_wiki_entry` (read bodies with `get_wiki_entry` when not on the same machine as the KB)
3. Append summary is written to `wiki/log.md` automatically

## Page types (`entry_type`)

| type | use |
|------|-----|
| `concept` | atomic idea (default L1) |
| `entity` | person, org, paper, product |
| `source-summary` | one page per `raw/` source |
| `comparison` | trade-offs between approaches |
| `overview` | cross-source synthesis (L2/L3) |

## YAML front matter (required)

```yaml
---
title: "Concept name"
domain: "IT"
entry_type: concept
confidence: medium
source: "raw/paper.md"
tags: ["tag1"]
level: L1
status: compiled
related: ["it/other_concept.md"]
contradictions: []
last_validated: 2026-05-21
created: 2026-05-21
updated: 2026-05-21
---
```

- **confidence**: `high` | `medium` | `low` — strength of claims in this page (not PDF extraction quality)
- **related**: paths relative to `wiki/` (e.g. `it/foo.md`)

## Naming

- Prefer `[Domain] Concept_Name.md` under `wiki/<domain>/`
- Never name pages by chapter numbers alone

## Raw layer

- `raw/` is read-only for you: do not overwrite sources; new extractions are versioned by the engine
- Cite sources in `source` and source-summary pages
