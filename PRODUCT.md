# PDF MCP Module — 产品定位

| 字段 | 内容 |
|------|------|
| **名称** | `rust-pdf-mcp`（PDF MCP Module） |
| **一句话简介** | AI 原生知识编译引擎 |
| **标签** | `AI-native`, `Knowledge Compiler`, `Karpathy Mode` |
| **License** | MIT |
| **入口** | pdf-mcp (stdio MCP server) |

## 产品目标

1. **AI Agent 的长期外脑** — 提供可累积、可搜索的结构化知识库
2. **学习工具的刚需基础设施** — 将 PDF → wiki 的研发知识编译管道
3. **从"被动搜索"到"知识编译"的范式升级** — MCP 生态的独特定位

## 核心定位

- **AI 原生**: 服务对象是 AI Agent，不是人类用户
- **编译器模式**: PDF 预编译为结构化 Markdown (Karpathy 模式)
- **单二进制部署**: 零外部依赖（Rust 编译为静态链接二进制）
- **独立 agent 能力**: PDF 文本提取 + VLM 视觉理解 + 增量编译 + 全文搜索

## 用户群体

| 用户 | 场景 |
|------|------|
| **AI Agent 用户** (Claude/Cursor) | 构建个人/团队知识库，将 PDF 教材转为结构化知识 |
| **研究人员** | 论文编译 + 概念关联发现 |
| **开发者** | 技术文档编译 + API 参考构建 |
| **教育领域** | PDF 教材 → 知识库，搭配 AI 导师系统 |

## 技术竞品对比

| 维度 | MCP PDF Module | 纯 LLM 方案 (Claude Projects) | RAG (如 Pinecone + LangChain) |
|------|---------------|------|------|
| **知识组织** | 原子化词条，手动精选 | 段落切片，自动 | 向量切片，自动 |
| **准确性** | 高（人工审核） | 中（生成偏差） | 中（搜索噪声） |
| **增量维护** | 哈希增量，精准定位 | 全局重建 | 向量重新嵌入 |
| **可解释性** | 完全透明，文件即索引 | 黑盒，依赖 LLM 内部状态 | 半透明，向量不可读 |
| **存储格式** | 纯文本 Markdown，Git 可追踪 | 闭源 blob | 专有向量数据库 |
| **部署复杂度** | 单二进制，零依赖 | 云端闭源 | 多组件（embedder + vector db + orchestrator） |
| **长期所有权** | 100% 用户持有 | 锁定在平台 | 锁定在基础设施 |

## MCP 工具能力矩阵

- **full 模式**（默认）：53 个独立 MCP 工具，各带 input/output JSON Schema
- **Code Mode**（`COMPENDIUM_MCP_MODE=code`）：`search_compendium_api` + `execute_compendium`；53 个 API 通过批次 `calls` 调用。见 [docs/CODE_MODE.md](../docs/CODE_MODE.md)

### full 模式工具分组

### PDF 提取层

| 工具 | 输出 | 适用场景 |
|------|------|----------|
| `extract_text` | String | 简单文本 PDF |
| `extract_structured` | `[PageParagraph]` | 保留段落/bbox |
| `get_page_count` | u32 | 预览前检查 |
| `search_keywords` | `[KeywordMatch]` | 正则 + 二分定位 |
| `extrude_to_server_wiki` | String | 提取到 server wiki |
| `extrude_to_agent_payload` | Markdown payload | 直接注入对话 |

### 知识编译层

| 工具 | 编译产物 | 适用场景 |
|------|----------|----------|
| `compile_to_wiki` | raw/*.md + compile_prompt | **主入口** |
| `incremental_compile` | Merkle hash 变更检测 | 批量增量 |
| `recompile_entry` | 新 entry.md + v{N}.md backup | 修正知识 |
| `aggregate_entries` | `[AggregationCandidate]` | L1→L2 聚合 |
| `micro_compile` | 即时 Markdown（不持久化） | 零污染 preview |
| `hypothesis_test` | `[ContradictionPair]` | 辩证推理 |

### 认知检索层

| 工具 | 能力 | 索引 |
|------|------|------|
| `search_knowledge` | 全文检索 (CJK-aware) | Tantivy |
| `get_entry_context` | N-hop 邻居遍历 | petgraph |
| `suggest_links` | 新链接建议 (Jaccard) | petgraph |
| `find_orphans` | 孤立条目发现 | petgraph |
| `export_concept_map` | Mermaid.js graph | petgraph |
| `check_quality` | 质量扫描（漂移/矛盾） | 全 wiki scan |

### 运维管理

| 工具 | 能力 |
|------|------|
| `get_config` | 查看配置 |
| `set_config` | 修改配置 |
| `get_health_report` | 引擎/索引/缓存健康 |
| `trigger_incremental_compile` | 手动触发 |
| `get_compile_status` | 进度查询 |
| `show_wiki_browser` | 浏览器入口 |

### 资源服务

| 资源 | 实现 |
|------|------|
| `rust-pdf://dashboard` | 编译时嵌入仪表板 (rust_embed) |
| `rust-pdf://wiki-browser` | 编译时嵌入知识浏览器 (rust_embed) |

## 技术规格

| 指标 | 数值 |
|------|------|
| Rust 版本 | 1.95.0 |
| Crates 数量 | 9 (8 lib + 1 bin) |
| MCP 工具总数 | 53（full）/ 2（code） |
| MCP 资源 | 2 embedded |
| 单二进制大小 | ~30MB (release, stripped) |
| WASM 引擎大小 | ~800KB (gzipped) |
| 支持 PDF 页数上限 | 伪无界 (mmap + lazy) |
| 知识库格式 | YAML front matter + Markdown body |
| 文本提取引擎 | pdfium (chromium) |
| VLM 引擎 | GPT-4o / Claude 3.5 / GLM-4.6v |

## 竞争优势

1. **知识所有权**: 100% 用户持有，纯文本 Markdown，Git 可追踪
2. **编译模式 > 搜索模式**: 预处理精度远超实时 RAG
3. **单一二进制**: 零外部服务，点击即用
4. **开源规则**: MIT 许可证，社区可共建

## 生态站位

在 MCP 生态中，定位为**知识层的编译强引擎**：

```
MCP 生态中的 PDF 模块
├── LLM Client (Cursor/Claude Desktop)     ← 调用方
├── MCP PDF Module (本产品)                 ← 知识编译
├── Server Toolkit (Things/Resources)       ← 在对话中激活
└── VLM Gateway (GPT-4o/Claude/GLM-4.6v)  ← 视觉理解
```

## 商业模式

- **开源免费**: MIT License，社区驱动
- **付费功能 (计划中)**:
  - VLM 高级集成 (复杂表/非标准 OCR)
  - 团队协作知识库
  - 托管式 SaaS (无需本地部署)
  - 企业级合规审计日志

## 目标里程碑

| 里程碑 | 状态 | 说明 |
|--------|------|------|
| v0.1: 基础 PDF 提取 | ✅ | 2024-04, pdfium MCP server |
| v0.2: 插件化架构 | ✅ | 2024-04, ToolRegistry + CircuitBreaker |
| v0.3: 奥卡姆剃刀重构 | ✅ | 2024-04, 极简架构 + VLM fallback |
| v0.4: AI 知识编译器 | ✅ | 2026-05, 20 tools + Karpathy 模式 |
| v0.5: 架构强化 | ✅ | 2026-05, P0/P1 加固 + 文档完善 |
| v0.6: 视觉理解增强 | 🔲 | 复杂表 OCR + 智能化降级 |
| v1.0: 生产发布 | 🔲 | 云端部署 + 开发者文档 |