# @smile9493/pdf-mcp

AI-native PDF knowledge compilation engine - MCP server for Claude, Cursor, and other AI clients.

## Installation

```bash
npx @smile9493/pdf-mcp
```

## Usage with MCP Clients

### Trae IDE

Add to your MCP configuration:

```json
{
  "mcpServers": {
    "pdf-mcp": {
      "command": "npx",
      "args": ["-y", "@smile9493/pdf-mcp"]
    }
  }
}
```

### Cursor

Add to `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "pdf-mcp": {
      "command": "npx",
      "args": ["-y", "@smile9493/pdf-mcp"]
    }
  }
}
```

### Claude Desktop

Add to your Claude Desktop config:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
**Linux**: `~/.config/claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "pdf-mcp": {
      "command": "npx",
      "args": ["-y", "@smile9493/pdf-mcp"]
    }
  }
}
```

## Features

- 🔥 **Karpathy Compiler Mode** - PDF pre-compiled to structured Markdown
- 🧠 **Cognitive Index Layer** - Tantivy + petgraph + TF-IDF
- 🚀 **Pure Rust** - Single binary, zero external dependencies
- 🔄 **Incremental Compilation** - Merkle hash detection
- 🖼️ **VLM Visual Understanding** - OCR fallback for scanned PDFs
- 🎯 **25 MCP Tools** - PDF extraction, knowledge compilation, cognitive index

## Available Tools

### PDF Extraction (6)
- `extract_text` - Extract plain text from PDF
- `extract_structured` - Extract structured data (per-page + bbox)
- `get_page_count` - Get PDF page count
- `search_keywords` - Search keywords in PDF
- `extrude_to_server_wiki` - Extract to server-side wiki
- `extrude_to_agent_payload` - Return Markdown payload

### Knowledge Compilation (6)
- `compile_to_wiki` - PDF → knowledge base compilation
- `incremental_compile` - Incremental compilation (hash detection)
- `micro_compile` - On-demand extraction (not persisted)
- `aggregate_entries` - L1→L2 aggregation candidates
- `hypothesis_test` - Contradiction detection
- `recompile_entry` - Single entry recompilation

### Cognitive Index (7)
- `search_knowledge` - Tantivy full-text search
- `rebuild_index` - Rebuild all indexes
- `get_entry_context` - N-hop neighbor discovery
- `find_orphans` - Orphan entry detection
- `suggest_links` - Link suggestions
- `export_concept_map` - Mermaid concept map
- `check_quality` - Wiki quality analysis

### Management (6)
- `get_config` - Get runtime configuration
- `set_config` - Set configuration key
- `get_health_report` - Health report
- `trigger_incremental_compile` - Trigger compilation
- `get_compile_status` - Compile status
- `show_wiki_browser` - Open wiki browser

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `PDFIUM_LIB_PATH` | Path to pdfium library | Bundled |
| `VLM_API_KEY` | VLM API key | - |
| `VLM_ENDPOINT` | VLM API endpoint | - |
| `VLM_MODEL` | VLM model name | `gpt-4o` |
| `RUST_LOG` | Log level | `info` |

## License

MIT
