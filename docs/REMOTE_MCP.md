# Remote MCP：无需宿主机文件系统的远程读写

Compendium 的设计使得运行在**另一台机器**（局域网或公网）上的 AI Agent **不需要** `docker exec`、卷挂载或直接访问宿主文件系统。所有 Wiki 读写都通过服务端的 MCP 工具完成。

## 传输方式

| 传输方式 | 适用场景 | 端点 / 命令 |
|-----------|-------------|-------------------|
| **HTTP JSON-RPC** | 远程 `pdf-cli`、自定义 Agent、反向代理 | `POST http://<host>:8001/mcp` |
| **stdio** | Cursor/Claude Desktop 与 Docker 运行在同一台主机 | `docker exec -i pdf-mcp pdf-mcp`（纯 stdio：`env -u HTTP_PORT pdf-mcp`） |

HTTP 请求体为标准 MCP JSON-RPC（`tools/call`、`initialize`、`tools/list`），与 stdio 协议一致。

## 读写工具对应关系

| 目标 | 工具 | 备注 |
|------|------|--------|
| 读取**完整** Markdown | `get_wiki_entry` | `entry_path` 相对于 `wiki/`（如 `CS/软件架构设计.md`） |
| 读取**预算内**上下文 | `get_agent_context` | 截断正文 + 图谱邻居 + 关联搜索命中 |
| 写入 / 移动 / 创建 | `save_wiki_entry` | 完整 `content` 字符串（YAML front matter + 正文） |
| 部分编辑 | `patch_wiki_entry` | 结构化操作；可用 `preview_wiki_patch` 预览 |

批量写入后请调用 `rebuild_index` 或 `complete_compile_job` 重建索引。

## 示例：远程 `pdf-cli`

```bash
compendium query search "架构" --remote http://192.168.1.10:8001
# 底层原理：POST /mcp → tools/call → search_knowledge
```

`pdf-cli` 的 `RemoteClient::call_tool` 目标地址为 `{server}/mcp`。

## 示例：curl

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

## 笔记本上的 Cursor，服务器上的知识库

部署步骤：

1. 在服务端运行 Compendium（设置 `HTTP_PORT=8001`、`KNOWLEDGE_BASE`）。
2. 暴露 8001 端口（公网建议配置 TLS / 反向代理）。
3. 将 Agent 指向 HTTP MCP（如客户端支持），或在本地运行一个 stdio 代理，将请求转发到 `POST /mcp`（参见 `pdf-cli` 代理模式：`pdf-module-rs/crates/pdf-cli/src/proxy.rs`）。

> **注意**：不要使用 `GET /api/wiki/entries/*` 作为 Agent 写入路径——该 API 仅供浏览器界面提供只读 HTML/JSON。

## Wiki 重组工作流（仅 MCP）

1. `lint` — 健康报告  
2. `get_wiki_entry` — 读取待移动的每个页面  
3. `save_wiki_entry` — 写入新路径，旧路径可写一个归档 stub  
4. `save_wiki_entry` — 更新 `index.md`  
5. `rebuild_index` + `lint`
