# MCP Wiki Browser UI Extension Protocol

Version **1** — postMessage bridge between the embedded wiki SPA (`ui://wiki/browser`) and the MCP host.

## Envelope

All messages MUST include:

| Field | Type | Description |
|-------|------|-------------|
| `v` | number | Protocol version (currently `1`) |
| `type` | string | Message type (see below) |
| `source` | string | Sender id, e.g. `wiki-browser` |

## Host → SPA

No host-initiated types are required in v1.

## SPA → Host

### `mcp-ask-ai`

User invoked “Ask AI” on the current wiki entry.

```json
{
  "v": 1,
  "type": "mcp-ask-ai",
  "source": "wiki-browser",
  "title": "Entry title",
  "path": "IT/concept.md",
  "excerpt": "First 2000 chars of body markdown…"
}
```

## Origin policy

The SPA MUST call `postMessage(payload, targetOrigin)` where `targetOrigin` is derived from `document.referrer` when embedded, else `window.location.origin`. The host SHOULD ignore messages when `event.origin` does not match the expected parent origin.

## Future types (reserved)

- `mcp-navigate-entry` — deep-link to another entry from host
- `mcp-compile-status` — push compile job updates into the UI

## Compile pipeline sampling (server-side)

When the MCP **stdio** server runs with a host that supports `sampling/createMessage`, set:

```bash
export RSUT_COMPILE_SAMPLING=1
```

After `compile_to_wiki`, `incremental_compile`, `compile_uploaded_pdf`, or `trigger_incremental_compile` reaches `awaiting_agent`, the server may call the host LLM to summarize the generated compile prompt. On success:

- The compile job `message` field stores a truncated `sampling_summary: …` hint.
- The tool JSON gains a `sampling_summary` object (`model`, `summary`, `prompt_path`).

Failures are logged and do not fail the compile. HTTP-only mode without stdio sampling has no effect.
