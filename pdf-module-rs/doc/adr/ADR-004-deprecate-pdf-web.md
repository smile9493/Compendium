# ADR-004: pdf-mcp embeds pdf-web-ui; deprecate pdf-web binary

- **Status**: accepted
- **Date**: 2026-05-19
- **Deciders**: PDF MCP team

## Context

The wiki browser existed as a separate `pdf-web` service and as static assets. Maintaining two deployment paths duplicated routes, auth assumptions, and release verification.

## Decision

- **Canonical UI**: Vue 3 `pdf-web-ui` embedded in and served by `pdf-mcp` at `/app/`.
- **`pdf-web` binary**: deprecated; retained only as an optional management API sidecar if needed for legacy installs.
- New features (compile drawer, share links, MCP postMessage) ship only in the embedded UI.

## Consequences

- One Docker image and one release artifact for browser + MCP HTTP.
- Documentation and FEATURE_MATRIX must not imply `pdf-web` is required for wiki browsing.

## Alternatives considered

| Alternative | Pros | Cons |
|-------------|------|------|
| Standalone pdf-web only | Independent scaling | Duplicate stack |
| Two full UIs | Flexibility | Unsustainable maintenance |
