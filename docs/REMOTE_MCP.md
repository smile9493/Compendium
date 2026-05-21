# Remote MCP: read/write without host filesystem

Compendium is designed so AI agents on **another machine** (LAN or public internet) never need `docker exec`, volume mounts, or direct file access. All wiki I/O goes through MCP tools on the server.

## Transport options

| Transport | When to use | Endpoint / command |
|-----------|-------------|-------------------|
| **HTTP JSON-RPC** | Remote `pdf-cli`, custom agents, reverse proxy | `POST http://<host>:8001/mcp` |
| **stdio** | Cursor/Claude Desktop on the same host as Docker | `docker exec -i pdf-mcp pdf-mcp` (stdio-only: `env -u HTTP_PORT pdf-mcp`) |

The HTTP body is standard MCP JSON-RPC (`tools/call`, `initialize`, `tools/list`), same as stdio.

## Read / write pair

| Goal | Tool | Notes |
|------|------|--------|
| Read **full** Markdown | `get_wiki_entry` | `entry_path` relative to `wiki/` (e.g. `CS/软件架构设计.md`) |
| Read **budget** context | `get_agent_context` | Truncated body + graph neighbors + related search hits |
| Write / move / create | `save_wiki_entry` | Full `content` string (YAML front matter + body) |
| Partial edit | `patch_wiki_entry` | Structured ops; preview with `preview_wiki_patch` |

After bulk writes, call `rebuild_index` or `complete_compile_job`.

## Example: remote `pdf-cli`

```bash
compendium query search "架构" --remote http://192.168.1.10:8001
# Under the hood: POST /mcp with tools/call → search_knowledge
```

`pdf-cli` `RemoteClient::call_tool` targets `{server}/mcp`.

## Example: curl

```bash
curl -s -X POST http://localhost:8001/mcp \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "get_wiki_entry",
      "arguments": { "entry_path": "index.md" }
    }
  }'
```

## Cursor on a laptop, KB on a server

1. Run Compendium on the server (`HTTP_PORT=8001`, `KNOWLEDGE_BASE` set).
2. Expose 8001 (TLS/reverse proxy recommended for public internet).
3. Point the agent at HTTP MCP (when supported) or run a local stdio proxy that forwards to `POST /mcp` (see `pdf-cli` proxy mode in `pdf-module-rs/crates/pdf-cli/src/proxy.rs`).

Do **not** rely on `GET /api/wiki/entries/*` for agent write paths — that API is read-only HTML/JSON for the browser UI.

## Wiki reorganize workflow (MCP-only)

1. `lint` — health report  
2. `get_wiki_entry` — read each page to move  
3. `save_wiki_entry` — write new path + optional archived stub at old path  
4. `save_wiki_entry` — update `index.md`  
5. `rebuild_index` + `lint`
