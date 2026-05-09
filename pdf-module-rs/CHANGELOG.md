# Changelog

## [0.5.0] - 2026-05-09

### 重大变更：架构强化 + 仓库整理

P0-P1 级工程加固，提升安全性、可维护性和文档完整性。

### Changed - 架构强化

- **错误类型统一**: `pdf-core::PdfModuleError` 合并为 `pdf_common::PdfError` 类型别名，消除 900+ 行重复维护
- **`#[tracing::instrument]` 全覆盖**: 所有 26 个 MCP 工具 handler 函数 + `dispatch_tool` 均已添加结构化日志
- **Async 隔离**: `fs::write()` → `tokio::fs::write()`，`std::fs::read_to_string()` → `tokio::fs::read_to_string()`
- **启动竞态消除**: HTTP 服务器启动检测从 `sleep(500ms)` 改为 `oneshot` 信号 + `timeout(3s)`
- **轮询反模式消除**: `server.rs` 中的 `sleep(100ms)` polling 替换为 `CancellationToken::cancelled()` 事件驱动
- **正则 panic 消除**: `Regex::new().expect()` → `.map_err()` 错误传播

### Changed - 工程配置

- **`rust-toolchain.toml`**: 锁定 Rust 1.95.0 + `wasm32-unknown-unknown` target
- **`panic='abort'`**: `[profile.release]` 启用 abort 策略
- **Clippy 零警告**: 修复 10+ 警告 (collapsible_match, unnecessary_sort_by, manual_checked_ops 等)

### Added - 新功能

- **MCP Resources 协议**: `resources/list` + `resources/read`，通过 `rust_embed` 编译时嵌入 dashboard/wiki-browser
- **图索引持久化**: `graph.bin` bincode 序列化，启动时反序列化或重建
- **TF-IDF 向量嵌入**: `batch_embed_all()`, `embed_entry()`, `VectorIndex`
- **知识库快照**: `CollectContext`, `AggregationCandidate`, `scan_wiki_files()`
- **版本备份**: `.versions/v{N}.md` 重编译前自动备份

### Changed - 仓库结构

- **前端移除**: `web/` Vue.js 目录完整移除 (-14,000 行)，简化至 Pure Rust monorepo
- **`.gitignore` 完善**: 新增 50+ 模式覆盖构建产物、运行时数据、IDE/OS 文件
- **根目录清理**: 测试脚本迁移至 `scripts/`，审查报告已删除
- **文档更新**: README.md/README.en.md 反映 23 tools，ARCHITECTURE.md 扩展至 280+ 行含断水堤分层

### Fixed

- 拼写错误: `"rsut-pdf-mcp"` → `"rust-pdf-mcp"`
- 死代码: 移除未使用 `NaiveDate` 导入，添加 `#[allow(dead_code)]` 用于 serde 引用函数
- 文档警告: 2 个 cargo doc 链接/HTML 标签问题已修复

---

## [0.4.0] - 2026-05-04

### 重大变更：AI 原生知识编译引擎

从"PDF 提取工具"升级为"**AI 原生知识编译与推理引擎**"，完整实现 Karpathy 编译器模式。

### Added - 知识引擎核心

- **KnowledgeEngine**: 知识编译调度核心
  - `compile_to_wiki`: PDF → raw/ + AI 编译提示
  - `incremental_compile`: Merkle 哈希增量编译
  - `recompile_entry`: 单条目重编译 + 版本备份
  - `aggregate_entries`: L1→L2 聚合候选发现
  - `check_quality`: Wiki 质量扫描
  - `micro_compile`: 即时 PDF 提取（不污染 wiki）
  - `hypothesis_test`: 矛盾对发现 + 辩论框架

- **KnowledgeEntry**: 标准化 YAML front matter
  - 15 个元数据字段（title, domain, tags, level, status, quality_score 等）
  - 支持 L0/L1/L2/L3 知识金字塔
  - 支持 contradictions/related/aggregated_from 链接

- **HashCache**: Merkle 风格增量变更检测
  - SHA-256 文件哈希
  - 自动跳过未变更 PDF

### Added - 认知索引层

- **FulltextIndex** (Tantivy 0.22): 全文检索
  - CJK n-gram 分词器（中文字符 unigram + bigram）
  - 索引 title/body/tags/domain
  - 存储于 `.rsut_index/tantivy/`

- **GraphIndex** (petgraph 0.7): 知识图谱
  - `related`/`contradictions` 有向边
  - 标签共现弱关系边（Jaccard ≥ 0.3）
  - N 跳邻居发现
  - 孤立条目检测
  - 链接建议（Jaccard 相似度）
  - Mermaid.js 概念图导出

### Added - MCP 工具 (14 新增)

| 工具 | 说明 |
|------|------|
| `compile_to_wiki` | PDF → 知识库编译入口 |
| `incremental_compile` | 增量编译（哈希检测） |
| `recompile_entry` | 单条目重编译 |
| `aggregate_entries` | L1→L2 聚合候选 |
| `check_quality` | Wiki 质量扫描 |
| `micro_compile` | 即时提取（不持久化） |
| `hypothesis_test` | 矛盾推理 |
| `search_knowledge` | Tantivy 全文搜索 |
| `rebuild_index` | 重建所有索引 |
| `get_entry_context` | N 跳邻居发现 |
| `find_orphans` | 孤立条目检测 |
| `suggest_links` | 链接建议 |
| `export_concept_map` | Mermaid 概念图 |
| `search_keywords` | PDF 内关键词搜索 |

### Added - Sampling 协议

- **MCP Sampling**: Server-initiated LLM 调用
  - `SamplingClient`: 异步请求管理
  - `SamplingManager`: 超时/重试控制
  - 支持 text/image 消息类型

### Dependencies

- `tantivy` 0.22 - 全文检索
- `petgraph` 0.7 - 图索引

### 架构

```
┌──────────────────────────────────────────────────┐
│            AI Client (Claude / Cursor)            │
│            20 MCP tools via JSON-RPC              │
└──────────────────────┬───────────────────────────┘
                       │ stdio
                       ▼
┌──────────────────────────────────────────────────┐
│                 pdf-mcp (server)                  │
├──────────────────────────────────────────────────┤
│  ┌─── PDF Extraction (6 tools) ─────────────────┐│
│  └──────────────────────────────────────────────┘│
│  ┌─── Knowledge Engine (7 tools) ───────────────┐│
│  └──────────────────────────────────────────────┘│
│  ┌─── Cognitive Index (6 tools) ────────────────┐│
│  └──────────────────────────────────────────────┘│
└──────────────────────┬───────────────────────────┘
                       │
        ┌──────────────┴──────────────┐
        ▼                             ▼
┌───────────────┐         ┌───────────────────┐
│  PdfiumEngine │         │  VlmGateway       │
│  (FFI levee)  │         │  (conditional)    │
└───────────────┘         └───────────────────┘
```

---

## [0.3.0] - 2026-04-29

### 重大变更：奥卡姆剃刀重构

移除所有冗余模块，收敛至最小可运行架构：

### Removed

- **REST API**: MCP stdio 是最终契约，旁路冗余
- **Python SDK**: 官方 MCP SDK 足矣
- **多引擎抽象**: 单一 pdfium 胜任所有场景
- **缓存模块**: 大模型自带 Prompt Caching
- **熔断器**: 本地 I/O 无需网络熔断
- **SSE 传输**: stdio 是 MCP 标准
- **智能路由**: 无路由 = 无分支预测惩罚
- **插件系统**: 过度工程化
- **审计日志**: 简化为日志输出
- **存储抽象**: 无状态设计

### Added

- **Web Dashboard**: Vue 3 监控面板
  - MCP 配置页面 (服务器 + VLM API Key 配置)
  - MCP 监控仪表板 (连接状态、工具调用、日志)
  - MCP 工具测试页面
- **VLM 条件升级**: GPT-4o / Claude 3.5 Sonnet 集成
  - 扫描件检测
  - 混沌布局降级
  - `catch_unwind` FFI 安全
- **GitHub CI/CD**:
  - 多平台构建 (Windows/Linux/macOS)
  - Docker 镜像 (mcp + web)
  - Release 自动发布

### Changed

- **MCP 工具收敛至 3 个**:
  - `extract_text`: 提取纯文本
  - `extract_structured`: 提取结构化数据
  - `get_page_count`: 获取页数
- **前端类型与后端 DTO 完全对齐**
- **移除所有无用页面**: Outbox, Reconciliation, RBAC, VectorRoutes, AuditLogs

### 架构

```
AI Agent (Cursor/Claude)
    │
    └──► pdf-mcp (stdio) ──► PdfiumEngine ──► PDF

Web Dashboard (Vue 3)
    │
    ├── MCP 配置
    ├── MCP 监控
    └── 工具测试
```

---

## [0.2.0] - 2026-04-25

### Added - 插件化架构

- **Plugin Registry**: `ToolRegistry` 实现，支持分类索引和能力索引
- **ToolHandler trait**: 扩展生命周期钩子
- **PluginRegistry trait**: 统一注册/查询接口
- **ToolDispatcher trait**: 工具调度/批量执行/健康检查

### Added - 控制平面

- **AuditLogger**: 审计日志记录
- **RateLimiter**: 令牌桶限流
- **CircuitBreaker**: 熔断器状态机
- **MetricsCollector**: 执行指标收集

---

## [0.1.0] - 2024-04-22

### Added

- PDF 文本提取 (pdfium 引擎)
- MCP Server (stdio)
- 文件验证
