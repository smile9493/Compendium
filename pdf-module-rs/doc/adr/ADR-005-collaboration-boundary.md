# ADR-005: Collaboration boundary (MCP write, UI read)

- **Status**: accepted
- **Date**: 2026-05-19
- **Deciders**: PDF MCP team

## Context

`pdf-core` implements local collaboration: audit log, patch proposals, entry locks. The Web UI gained share links for read-only access. Product scope needed a clear split to avoid a half-built approval UI.

## Decision

**Web UI responsibilities**: browse, search, compile operations, settings/ops banner, read-only share viewing.

**MCP responsibilities**: all wiki mutations and collaboration workflows.

### Write paths

1. **Direct write** — `patch_wiki_entry`, `save_wiki_entry` (immediate apply + reindex).
2. **Proposal write** — `submit_patch_proposal` → review in agent/host → `apply_patch_proposal`; pending items listed via `list_patch_proposals`.

**Out of scope for Web UI v1**: proposal list, apply/reject buttons, audit viewer.

Share links (`POST /api/v1/shares`) provide time-limited read access without write.

## Consequences

- Agents orchestrate governance; humans use the browser for inspection and compile ops.
- `list_patch_proposals` closes the discovery gap without building a full collab panel.

## Alternatives considered

| Alternative | Pros | Cons |
|-------------|------|------|
| Full collab panel in UI | Human-friendly review | Large UX surface; duplicates MCP |
| No proposals at all | Simpler | Loses audit-friendly workflow |
