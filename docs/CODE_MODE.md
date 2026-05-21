# Compendium MCP Code Mode

Code Mode replaces 53 per-tool JSON Schemas with **two MCP tools** plus an optional TypeScript API reference resource. Agents discover methods via search or `compendium.d.ts`, then run batches through `execute_compendium`.

## Enable

Set in MCP server env (default is `full` for backward compatibility):

```json
{
  "mcpServers": {
    "pdf-mcp": {
      "command": "/path/to/pdf-mcp",
      "env": {
        "COMPENDIUM_MCP_MODE": "code",
        "KNOWLEDGE_BASE_PATH": "/home/user/my-kb"
      }
    }
  }
}
```

## MCP tools (code mode)

| Tool | Purpose |
|------|---------|
| `search_compendium_api` | Keyword search over method names and descriptions |
| `execute_compendium` | Run `calls: [{ method, args }]` in-process (whitelist = full API catalog) |

## Resource

| URI | Content |
|-----|---------|
| `compendium://sdk/typescript` | Generated `compendium.d.ts` (all 53 methods) |

## Example batch

```json
{
  "calls": [
    {
      "method": "search_knowledge",
      "args": {
        "knowledge_base": "/home/user/my-kb",
        "query": "HTTP/2 multiplexing",
        "limit": 5
      }
    },
    {
      "method": "get_agent_context",
      "args": {
        "knowledge_base": "/home/user/my-kb",
        "entry_path": "it/http2_multiplex.md"
      }
    }
  ],
  "stop_on_error": false,
  "max_calls": 10,
  "max_result_chars": 8192
}
```

Each result: `{ "method", "ok", "data" | "error", "truncated"? }`.

## Regenerate SDK artifacts

After changing tool contracts:

```bash
cd pdf-module-rs
cargo run -p pdf-mcp-contracts --bin generate-sdk
```

Writes:

- `pdf-module-rs/templates/sdk/compendium.d.ts`
- `pdf-module-rs/templates/sdk/compendium-api-index.json`

## When to use full mode

Use `COMPENDIUM_MCP_MODE=full` (or unset) when the client does not support Code Mode workflows or you want native per-tool MCP calls without batching.

## Wiki UI (HTTP)

When the HTTP server is running (`HTTP_PORT` set), open **Settings → About** in the embedded wiki browser. It shows the **current MCP mode** (read-only), tool counts, and buttons to **copy Cursor `mcp.json` snippets** for Code or Full mode. Changing mode still requires editing env and reloading MCP in Cursor.
