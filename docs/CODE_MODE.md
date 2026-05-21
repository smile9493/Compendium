# Compendium MCP Code Mode

Code Mode 将 53 个 MCP 工具的 JSON Schema 开销压缩为 **2 个工具** + TypeScript API 参考资源。Agent 通过搜索或 `compendium.d.ts` 发现方法，然后通过 `execute_compendium` 批量执行。

## 启用

在 MCP server 环境变量中设置（默认为 `full` 以保持向后兼容）：

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

## MCP 工具（Code Mode 下仅 2 个）

| 工具 | 用途 |
|------|------|
| `search_compendium_api` | 按关键词搜索方法名和描述 |
| `execute_compendium` | 进程内批量执行 `calls: [{ method, args }]`（白名单 = 完整 API 目录） |

## 资源

| URI | 内容 |
|-----|------|
| `compendium://sdk/typescript` | 自动生成的 `compendium.d.ts`（包含全部 53 个方法） |

## 批量调用示例

```json
{
  "calls": [
    {
      "method": "search_knowledge",
      "args": {
        "knowledge_base": "/home/user/my-kb",
        "query": "HTTP/2 多路复用",
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

每条结果格式：`{ "method", "ok", "data" | "error", "truncated"? }`。

## 重新生成 SDK 产物

修改工具契约后：

```bash
cd pdf-module-rs
cargo run -p pdf-mcp-contracts --bin generate-sdk
```

写入文件：

- `pdf-module-rs/templates/sdk/compendium.d.ts`
- `pdf-module-rs/templates/sdk/compendium-api-index.json`

## 何时使用 Full 模式

当客户端不支持 Code Mode 工作流，或希望使用原生按工具调用的 MCP 方式（不批处理）时，请使用 `COMPENDIUM_MCP_MODE=full`（或不设置该环境变量）。

## Wiki 界面（HTTP）

当 HTTP 服务启动后（设置了 `HTTP_PORT`），打开嵌入式 Wiki 浏览器中的 **Settings → About**。页面会显示**当前 MCP 模式**（只读）、工具数量统计，以及**复制 Cursor `mcp.json` 片段**的按钮（Code / Full 模式均可一键复制）。切换模式仍需修改环境变量并在 Cursor 中重新加载 MCP。
