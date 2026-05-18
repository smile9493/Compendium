# ADR-001: body_markdown as single source of truth

- **Status**: accepted
- **Date**: 2026-05-19
- **Deciders**: PDF MCP team

## Context

Wiki entries were historically rendered from server-generated HTML and client-side Markdown. Dual representations caused drift, larger payloads, and ambiguous contracts for MCP hosts embedding the browser UI.

## Decision

- Wiki entry bodies are stored and transmitted as **Markdown only** (`body_markdown`).
- The HTTP API does **not** generate `body_html` for clients.
- The Web UI renders via `MarkdownRenderer` on the client.
- Missing or empty `body_markdown` is a **client-visible error**, not a silent fallback.

## Consequences

- Smaller API responses and one rendering path.
- MCP `mcp-ask-ai` excerpts come from the same field as the UI.
- Clients must ship a Markdown renderer; legacy `body_html` consumers must migrate.

## Alternatives considered

| Alternative | Pros | Cons |
|-------------|------|------|
| Server HTML + Markdown | Rich server control | Two sources of truth |
| HTML-only | Simple old clients | Poor MCP/agent ergonomics |
