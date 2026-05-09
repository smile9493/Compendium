# PDF MCP Module

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.95%2B-orange.svg)](https://www.rust-lang.org/)
[![MCP](https://img.shields.io/badge/MCP-2024--11--05-blue.svg)](https://modelcontextprotocol.io/)

**AI 原生知识编译引擎** — 将 PDF 文档编译为结构化知识库，为 Claude、Cursor 等 AI 客户端提供长期记忆与推理后端。

[English](./README.en.md) | 简体中文

## ✨ 特性

- 🔥 **Karpathy 编译器模式** — PDF 预编译为结构化 Markdown，知识可累积、可解释，支持 L0→L1→L2→L3 知识金字塔
- 🧠 **认知索引层** — Tantivy 全文检索 + petgraph 知识图谱 + TF-IDF 向量嵌入，三路检索融合
- 🚀 **纯 Rust 实现** — 单二进制部署，零外部服务依赖，高性能 FFI 防波堤
- 🔄 **增量编译** — Merkle 哈希检测，只编译变更的 PDF
- 🖼️ **VLM 视觉理解** — 条件性 OCR 回退，用于扫描版/图片型 PDF
- 🌐 **双协议服务** — stdio（MCP）+ HTTP（Wiki 浏览），oneshot 信号启动
- 🎯 **23 个 MCP 工具** — 覆盖 PDF 提取、知识编译、认知索引、资源服务

## 📦 安装

### 一键安装

```bash
curl -fsSL https://raw.githubusercontent.com/smile9493/rsut_pdf_mcp/main/install.sh | bash
```

### Docker

```bash
docker pull smile9493/pdf-mcp:latest
```

### 从源码构建

```bash
git clone https://github.com/smile9493/rsut_pdf_mcp.git
cd rsut_pdf_mcp/pdf-module-rs
cargo build --release --bin pdf-mcp
```

## 🚀 快速开始

### 1. 配置 AI 客户端

**Cursor** (`~/.cursor/mcp.json`):

```json
{
  "mcpServers": {
    "pdf-mcp": {
      "command": "/opt/pdf-module/pdf-mcp",
      "env": {
        "PDFIUM_LIB_PATH": "/opt/pdf-module/lib/libpdfium.so"
      }
    }
  }
}
```

**Claude Desktop** (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "pdf-mcp": {
      "command": "/opt/pdf-module/pdf-mcp"
    }
  }
}
```

### 2. 编译 PDF 到知识库

```
用户: 帮我把 /path/to/paper.pdf 编译到知识库

AI: [调用 compile_to_wiki 工具]
已将 PDF 编译到知识库：
- 原始提取: raw/paper.md
- 编译提示: raw/paper.compile_prompt.md
请阅读提取内容，提炼核心概念，创建原子化词条...
```

### 3. 搜索知识库

```
用户: 搜索关于 HTTP/2 的知识

AI: [调用 search_knowledge 工具]
找到 3 条相关知识：
1. [IT] HTTP/2 多路复用 (score: 0.92)
2. [IT] HTTP/2 头部压缩 (score: 0.85)
3. [Network] HTTP/2 vs HTTP/1.1 对比 (score: 0.78)
```

## 🛠️ MCP 工具 (23 个)

### PDF 提取 (6)

| 工具 | 说明 |
|------|------|
| `extract_text` | 提取 PDF 纯文本 |
| `extract_structured` | 提取结构化数据（每页文本 + bbox） |
| `get_page_count` | 获取 PDF 页数 |
| `search_keywords` | PDF 内关键词搜索（支持正则） |
| `extrude_to_server_wiki` | 提取到服务端 Wiki |
| `extrude_to_agent_payload` | 返回 Markdown payload 到对话 |

### 知识编译 (7)

| 工具 | 说明 |
|------|------|
| `compile_to_wiki` | PDF → 知识库编译入口 |
| `incremental_compile` | 增量编译（Merkle 哈希检测） |
| `recompile_entry` | 单条目重编译 + 版本备份 |
| `aggregate_entries` | L1→L2 聚合候选发现 |
| `check_quality` | Wiki 质量扫描（漂移/矛盾检测） |
| `micro_compile` | 即时提取（不持久化） |
| `hypothesis_test` | 矛盾对发现 + 辩论框架生成 |

### 认知索引 (6)

| 工具 | 说明 |
|------|------|
| `search_knowledge` | Tantivy 全文搜索（CJK 分词） |
| `rebuild_index` | 重建所有索引 |
| `get_entry_context` | N 跳邻居发现（知识图谱遍历） |
| `find_orphans` | 孤立条目检测 |
| `suggest_links` | 链接建议（Jaccard 相似度） |
| `export_concept_map` | Mermaid.js 概念图导出 |

### 管理 (5)

| 工具 | 说明 |
|------|------|
| `get_config` | 获取当前配置 |
| `set_config` | 更新配置项 |
| `get_health_report` | 系统健康报告（引擎/索引/缓存状态） |
| `trigger_incremental_compile` | 触发批量增量编译 |
| `get_compile_status` | 查询编译任务状态 |
| `show_wiki_browser` | 显示 Wiki 浏览器入口 |

### 资源 (2)

| 资源 | 说明 |
|------|------|
| `rust-pdf://dashboard` | 嵌入式系统仪表板（rust_embed 编译） |
| `rust-pdf://wiki-browser` | 嵌入式知识库浏览器 |

## 🏗️ 架构

```
┌──────────────────────────────────────────────────┐
│            AI Client (Claude / Cursor)            │
│               23 MCP tools via JSON-RPC           │
└──────────────┬───────────────┬───────────────────┘
               │ stdio         │ HTTP
               ▼               ▼
┌──────────────────────┐ ┌──────────────────┐
│   pdf-mcp (server)   │ │  Wiki HTTP       │
│   JSON-RPC dispatch  │ │  axum + embed    │
├──────────────────────┤ └──────────────────┘
│  Extraction │ Compile│
│  Indexing  │ Manage  │
└──────────────┬───────┘
               │
    ┌──────────┴───────────┐
    ▼                      ▼
┌──────────────┐  ┌──────────────────┐
│ PdfiumEngine │  │ VlmVisualGateway │
│ (FFI levee)  │  │ (条件性 OCR)      │
└──────────────┘  └──────────────────┘
         │                │
         ▼                ▼
┌──────────────────────────────────────┐
│         Knowledge Engine             │
│  Tantivy │ petgraph │ TF-IDF Vector  │
│  hash_cache │ cache_db │ bincode     │
└──────────────────────────────────────┘
```

### 断水堤分层

```
Facade Layer:  pdf-mcp (MCP protocol), vlm-visual-gateway (HTTP facade)
Core Layer:    pdf-core (extraction, knowledge, parallel), pdf-common (shared)
Infra Layer:   pdf-macros (derive macros), pdf-wasm (WASM target)
```

## 📁 知识库结构

```
knowledge_base/
├── raw/                   # 原始 PDF 提取 (YAML)
├── wiki/                  # 编译后的知识库
│   ├── index.md           # 全局导航
│   ├── log.md             # 操作日志
│   ├── .versions/         # 重编译备份
│   └── <domain>/          # 领域词条
├── schema/                # 编译指令
├── .hash_cache/           # Merkle 哈希缓存 (JSON)
├── .rsut_index/           # 可重建索引
│   ├── tantivy/           # 全文检索索引
│   └── graph.bin          # 知识图谱持久化 (bincode)
└── .cache_db/             # 条目缓存
```

## 📝 条目格式

```yaml
---
title: "HTTP/2 多路复用"
domain: "IT"
source: "raw/rfc7540.pdf"
page: 12
tags: ["http", "networking", "protocol"]
level: L1
status: compiled
quality_score: 0.85
version: 1
contradictions: []
related: ["wiki/it/http1.md"]
created: 2026-05-04T00:00:00Z
updated: 2026-05-04T00:00:00Z
---
# HTTP/2 多路复用
正文内容...
```

## 🗺️ 知识金字塔

```
L3  Domain Map      (领域导航，每个领域一份)
    ↑ aggregated from
L2  Aggregation      (综述，同子主题多 L1 合并)
    ↑ aggregated from
L1  Atomic Concept   (原子概念，核心知识单元)
    ↑ compiled from
L0  Raw Extraction   (原始提取，PDF → text)
```

## ⚙️ 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `PDFIUM_LIB_PATH` | PDFium 动态库路径 | 自动检测 |
| `VLM_API_KEY` | VLM API 密钥 | - |
| `VLM_MODEL` | 模型名称 | `glm-4v-flash` |
| `VLM_ENDPOINT` | API 端点 | 智谱 API |
| `MCP_HTTP_PORT` | HTTP Wiki 服务端口 | - (禁用) |
| `KB_PATH` | 知识库根路径 | `./knowledge_base` |

## 📁 项目结构

```
pdf-module/
├── pdf-module-rs/         # Rust workspace (核心引擎)
│   ├── crates/
│   │   ├── pdf-common/    # 共享类型/错误/DTO
│   │   ├── pdf-core/      # 提取/知识/并行引擎
│   │   ├── pdf-mcp/       # MCP 协议服务器
│   │   ├── pdf-wasm/      # WASM 编译目标
│   │   ├── pdf-web/       # Web 前端 (Yew)
│   │   ├── pdf-cli/       # CLI 工具
│   │   ├── pdf-dashboard/ # 仪表板服务
│   │   ├── pdf-macros/    # 派生宏
│   │   └── vlm-visual-gateway/ # VLM 网关
│   ├── ARCHITECTURE.md    # 架构文档
│   └── CHANGELOG.md       # 变更日志
├── scripts/               # 测试/工具脚本
├── docs/                  # 用户文档
│   ├── API_REFERENCE.md
│   ├── VLM_INTEGRATION.md
│   └── ...
├── plugins/               # 第三方 MCP 插件
├── pdf-mcp-installer/     # 安装器
├── deploy/                # 部署配置
├── docker/                # Docker 镜像
├── Dockerfile
└── docker-compose.yml
```

## 📄 License

[MIT](LICENSE)