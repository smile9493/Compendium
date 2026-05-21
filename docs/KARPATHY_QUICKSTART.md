# Karpathy 模式快速开始（5 分钟）

本指南使用 **stdio MCP + 空知识库**，不启动 HTTP Wiki 或 VLM。

## 1. 初始化知识库

```bash
compendium kb init ~/my-kb
```

或 MCP 工具：`init_knowledge_base`（`knowledge_base` 指向空目录）。

将创建：

- `schema/AGENTS.md` — ingest / query / lint 三口令与 MCP 映射
- `wiki/index.md`、`wiki/log.md` — 中枢神经系统
- `raw/` — 原始素材（只读约定）

> 仓库根目录 [`CLAUDE.md`](../CLAUDE.md) 用于 **Rust 开发**；Wiki 维护规范在 `knowledge_base/schema/`。

## 2. 配置 Cursor MCP

指向 `pdf-mcp` 二进制；知识库路径设为 `~/my-kb`。

## 3. 三口令示例

### ingest

```
帮我把 /path/to/paper.pdf ingest 到知识库，领域 IT
```

Agent 应：`compile_to_wiki` → 阅读 `get_compilation_context` → `save_wiki_entry` → `complete_compile_job`。

### query

```
根据 wiki 回答：HTTP/2 多路复用和 HTTP/1.1 有何区别？
```

Agent 应：先读 `schema/AGENTS.md` 与 `wiki/index.md`，再 `search_knowledge`（可选 `mode: wiki_first`）或 `get_agent_context`。

### lint

```
lint 知识库
```

Agent 应：调用 `lint_wiki`，按报告修复后可选 `patch_wiki_entry`。

## 4. 查询回写

若回答值得保留：

```
把刚才的回答 archive 到 wiki，标题「HTTP/2 综述」
```

调用 `archive_answer` 写入 `overview` 类页面。

## 5. 轻量检索（可选）

在知识库 `.rsut_index/config.json` 中设置：

```json
{ "retrieval_mode": "wiki_first" }
```

或单次查询：`search_knowledge` + `"mode": "wiki_first"`（不经过 Tantivy/向量）。

## 环境变量（极简部署）

- 不设置 VLM 相关变量即可仅用 Pdfium 提取
- 不需要启动 HTTP Wiki 服务

详见 [知识编译指南](./KNOWLEDGE_COMPILATION_GUIDE.md) 与 [AI Agent 工作流](./AI_AGENT_SETUP_GUIDE.md)。
