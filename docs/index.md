# Compendium 文档

**AI 原生知识编译引擎** — 将 PDF 编译为结构化知识库，为 Claude、Cursor 等 MCP 客户端提供长期记忆与推理后端。

## 从这里开始

| 场景 | 文档 |
|------|------|
| 5 分钟跑通 Karpathy 编译流程 | [Karpathy 模式快速开始](KARPATHY_QUICKSTART.md) |
| 配置 Cursor / Claude MCP | [AI 客户端配置](AI_AGENT_SETUP_GUIDE.md) |
| 减少工具 Schema 开销 | [Code Mode](CODE_MODE.md) |
| 扫描版 PDF / 视觉理解 | [VLM 集成](VLM_INTEGRATION.md) |
| 全部 MCP 工具说明 | [API 参考](API_REFERENCE.md) |

## 核心能力

- **Karpathy 编译器模式** — PDF 预编译为结构化 Markdown，支持 L0→L3 知识金字塔
- **认知索引** — Tantivy 全文检索 + petgraph 知识图谱 + TF-IDF
- **纯 Rust** — 单二进制部署，`stdio` MCP + 可选 HTTP Wiki
- **增量编译** — Merkle 哈希检测变更

## 仓库链接

- [GitHub 源码](https://github.com/smile9493/Compendium)
- [架构说明 (ARCHITECTURE.md)](https://github.com/smile9493/Compendium/blob/main/pdf-module-rs/ARCHITECTURE.md)
- [贡献指南 (CONTRIBUTING.md)](https://github.com/smile9493/Compendium/blob/main/CONTRIBUTING.md)

本地预览文档站：`pip install -r requirements-docs.txt && mkdocs serve`
