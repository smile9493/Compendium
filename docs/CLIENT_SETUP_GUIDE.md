# PDF Module 客户端配置指南

## 概述

本指南说明如何配置 AI Agent (Cursor/Claude Desktop) 连接 PDF Module MCP 服务。

## 架构

```
┌─────────────────────────────────────────────────────────────┐
│                  AI Agent (Cursor/Claude Desktop)           │
└──────────────────────────┬──────────────────────────────────┘
                           │ JSON-RPC over stdio
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                     pdf-mcp (MCP 服务端)                    │
│  Windows: pdf-mcp.exe  │  Linux/macOS: pdf-mcp             │
├─────────────────────────────────────────────────────────────┤
│  MCP Tools (20):                                            │
│  ┌─ PDF Extraction (6) ───────────────────────────────────┐│
│  │ extract_text / extract_structured / get_page_count     ││
│  │ search_keywords / extrude_to_server_wiki               ││
│  │ extrude_to_agent_payload                               ││
│  └─────────────────────────────────────────────────────────┘│
│  ┌─ Knowledge Compilation (7) ─────────────────────────────┐│
│  │ compile_to_wiki / incremental_compile / recompile_entry││
│  │ aggregate_entries / check_quality / micro_compile      ││
│  │ hypothesis_test                                        ││
│  └─────────────────────────────────────────────────────────┘│
│  ┌─ Cognitive Index (6) ───────────────────────────────────┐│
│  │ search_knowledge / rebuild_index / get_entry_context   ││
│  │ find_orphans / suggest_links / export_concept_map      ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

---

## 方式一: 下载预编译二进制 (推荐)

### 1. 下载

从 [GitHub Releases](https://github.com/smile9493/rsut_pdf_mcp/releases) 下载对应平台版本：

| 平台 | 文件 | 解压后 |
|------|------|--------|
| Windows x64 | `pdf-mcp-windows-x64.zip` | `pdf-mcp.exe` |
| Linux x64 | `pdf-mcp-linux-x64.tar.gz` | `pdf-mcp` |
| macOS x64 | `pdf-mcp-macos-x64.tar.gz` | `pdf-mcp` |
| macOS ARM64 | `pdf-mcp-macos-arm64.tar.gz` | `pdf-mcp` |

### 2. 解压

```bash
# Linux/macOS
tar -xzf pdf-mcp-linux-x64.tar.gz
chmod +x pdf-mcp

# Windows: 解压 zip 文件
```

### 3. 配置 Agent

---

## 方式二: 从源码编译

```bash
git clone https://github.com/smile9493/rsut_pdf_mcp.git
cd rsut_pdf_mcp/pdf-module-rs
cargo build --release
```

二进制位于: `target/release/pdf-mcp`

---

## Cursor 配置

配置文件: `~/.cursor/mcp.json`

### 基础配置

```json
{
  "mcpServers": {
    "pdf-module": {
      "command": "/path/to/pdf-mcp"
    }
  }
}
```

### 带 VLM 配置

```json
{
  "mcpServers": {
    "pdf-module": {
      "command": "/path/to/pdf-mcp",
      "env": {
        "VLM_API_KEY": "sk-xxx",
        "VLM_ENDPOINT": "https://api.openai.com/v1/chat/completions",
        "VLM_MODEL": "gpt-4o",
        "VLM_TIMEOUT_SECS": "30",
        "VLM_MAX_CONCURRENCY": "5"
      }
    }
  }
}
```

---

## Claude Desktop 配置

配置文件位置:
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
- **Linux**: `~/.config/claude/claude_desktop_config.json`

### 基础配置

```json
{
  "mcpServers": {
    "pdf-module": {
      "command": "/path/to/pdf-mcp"
    }
  }
}
```

### 带 VLM 配置

```json
{
  "mcpServers": {
    "pdf-module": {
      "command": "/path/to/pdf-mcp",
      "env": {
        "VLM_API_KEY": "sk-ant-xxx",
        "VLM_ENDPOINT": "https://api.anthropic.com/v1/messages",
        "VLM_MODEL": "claude-3.5-sonnet"
      }
    }
  }
}
```

---

## VLM 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `VLM_API_KEY` | API 密钥 | - |
| `VLM_ENDPOINT` | API 端点 | - |
| `VLM_MODEL` | 模型 (`gpt-4o` / `claude-3.5-sonnet`) | `gpt-4o` |
| `VLM_TIMEOUT_SECS` | 请求超时 (秒) | `30` |
| `VLM_MAX_CONCURRENCY` | 最大并发数 | `5` |

---

## 可用工具 (20 个)

### PDF 提取 (6)

| 工具 | 说明 | 参数 |
|------|------|------|
| `extract_text` | 提取 PDF 纯文本 | `file_path` |
| `extract_structured` | 提取结构化数据 (per-page + bbox) | `file_path` |
| `get_page_count` | 获取 PDF 页数 | `file_path` |
| `search_keywords` | PDF 内关键词搜索 | `file_path`, `keywords` |
| `extrude_to_server_wiki` | 提取到服务端 Wiki | `file_path`, `wiki_base_path` |
| `extrude_to_agent_payload` | 返回 Markdown payload | `file_path` |

### 知识编译 (7)

| 工具 | 说明 | 参数 |
|------|------|------|
| `compile_to_wiki` | PDF → 知识库编译入口 | `pdf_path`, `knowledge_base`, `domain` |
| `incremental_compile` | 增量编译（哈希检测） | `knowledge_base` |
| `recompile_entry` | 单条目重编译 | `knowledge_base`, `entry_path` |
| `aggregate_entries` | L1→L2 聚合候选 | `knowledge_base` |
| `check_quality` | Wiki 质量扫描 | `knowledge_base` |
| `micro_compile` | 即时提取（不持久化） | `pdf_path`, `page_range` |
| `hypothesis_test` | 矛盾对发现 | `knowledge_base` |

### 认知索引 (6)

| 工具 | 说明 | 参数 |
|------|------|------|
| `search_knowledge` | Tantivy 全文搜索 | `knowledge_base`, `query`, `limit` |
| `rebuild_index` | 重建所有索引 | `knowledge_base` |
| `get_entry_context` | N 跳邻居发现 | `knowledge_base`, `entry_path`, `hops` |
| `find_orphans` | 孤立条目检测 | `knowledge_base` |
| `suggest_links` | 链接建议 | `knowledge_base`, `entry_path`, `top_k` |
| `export_concept_map` | Mermaid 概念图 | `knowledge_base`, `entry_path`, `depth` |

---

## Web Dashboard

访问 http://localhost:5173 使用图形界面：

- **MCP 配置** (`/mcp-config`): 可视化配置 MCP + VLM
- **MCP 监控** (`/mcp-monitor`): 实时监控工具调用
- **MCP 工具** (`/mcp-tools`): 测试工具调用

---

## 故障排查

### 测试 MCP 连接

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' | ./pdf-mcp
```

预期输出包含 `serverInfo`。

### 常见问题

| 问题 | 解决 |
|------|------|
| `permission denied` | `chmod +x pdf-mcp` |
| `VLM_API_KEY not set` | 检查 env 配置 |
| 工具不显示 | 重启 Agent |
