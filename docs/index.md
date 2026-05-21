---
hide:
  - navigation
  - toc
---

<!--
  Compendium — Landing Page
  Uses Material for MkDocs grid cards pattern (Pydantic / FastAPI style)
-->

<p align="center">
  <img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT">
  <img src="https://img.shields.io/badge/Rust-1.91%2B-orange.svg" alt="Rust 1.91+">
  <img src="https://img.shields.io/badge/MCP-2024--11--05-blue.svg" alt="MCP">
  <img src="https://img.shields.io/badge/Edition-2024-ff69b4.svg" alt="Rust 2024 Edition">
  <img src="https://img.shields.io/github/stars/smile9493/Compendium?style=social" alt="GitHub stars">
</p>

<h1 align="center" style="margin-top: 1.5rem;">
  Compendium
  <br>
  <small>AI 原生 PDF 知识编译引擎</small>
</h1>

<p align="center" style="font-size: 1.1rem; max-width: 640px; margin: 1rem auto 2rem auto; color: var(--md-default-fg-color--light);">
  将 PDF 文档编译为结构化、可累积、可推理的知识库——为 <strong>Cursor</strong>、<strong>Claude</strong> 及其他 MCP 客户端提供长期记忆与推理后端。53 个 MCP 工具，纯 Rust 实现，单二进制部署。
</p>

<p align="center">
  <a href="KARPATHY_QUICKSTART/" class="md-button md-button--primary">5 分钟快速开始</a>
  <a href="AI_AGENT_SETUP_GUIDE/" class="md-button">安装指南</a>
  <a href="https://github.com/smile9493/Compendium" class="md-button">:fontawesome-brands-github: GitHub</a>
</p>

---

## 核心能力

<div class="grid cards" markdown>

-   :material-lightning-bolt:{ .lg .middle } **Karpathy 编译器模式**

    ---

    PDF → 结构化 Markdown，支持 L0 → L1 → L2 → L3 知识金字塔。知识可累积、可解释、可演进，告别一次性 RAG。

    [:octicons-arrow-right-24: 了解更多](KARPATHY_QUICKSTART.md)

-   :material-magnify:{ .lg .middle } **三路认知索引**

    ---

    Tantivy 全文检索 + petgraph 知识图谱 + TF-IDF 向量嵌入，三路检索融合。不光找到内容，更理解上下文关系。

    [:octicons-arrow-right-24: 查看 API](API_REFERENCE.md)

-   :material-ferris:{ .lg .middle } **纯 Rust 实现**

    ---

    单二进制部署，零外部服务依赖。高性能 FFI 防波堤模式（Breakwater Pattern）——PdfiumEngine → Knowledge Engine 隔离。

    [:octicons-arrow-right-24: 阅读架构](https://github.com/smile9493/Compendium/blob/main/pdf-module-rs/ARCHITECTURE.md)

-   :material-source-branch:{ .lg .middle } **增量编译**

    ---

    Merkle 哈希检测变更，只编译有改动的 PDF。大规模知识库中保持高性能。

    [:octicons-arrow-right-24: 知识编译指南](KNOWLEDGE_COMPILATION_GUIDE.md)

-   :material-eye:{ .lg .middle } **VLM 视觉理解**

    ---

    条件性从 Pdfium 本地解析升档到远程 VLM 布局理解。扫描版和图片型 PDF 的 OCR 回退。

    [:octicons-arrow-right-24: VLM 集成](VLM_INTEGRATION.md)

-   :material-code-braces:{ .lg .middle } **Code Mode**

    ---

    将 53 个工具的 JSON Schema 开销压缩为 2 个 MCP 工具 + TypeScript API 资源。适合多步 ingest/query 任务流。

    [:octicons-arrow-right-24: Code Mode](CODE_MODE.md)

</div>

---

## 一分钟安装

=== "一键安装"

    ```bash
    curl -fsSL https://raw.githubusercontent.com/smile9493/Compendium/main/install.sh | bash
    ```

=== "Docker"

    ```bash
    docker pull smile9493/pdf-mcp:latest
    ```

=== "从源码构建"

    ```bash
    git clone https://github.com/smile9493/Compendium.git
    cd Compendium/pdf-module-rs
    cargo build --release --bin pdf-mcp
    ```

---

## 快速体验

<div class="grid" markdown>

<div markdown>

### :material-play:{ .middle } 初始化知识库

```bash
compendium kb init ~/my-kb
```

创建 Karpathy 模板：`schema/AGENTS.md`（三口令）、`wiki/index.md`、`raw/` 素材目录。

</div>

<div markdown>

### :material-upload:{ .middle } 编译第一篇 PDF

```
用户: 帮我把 paper.pdf 编译到知识库
AI:  [调用 compile_to_wiki 工具]
     → raw/paper.md
     → raw/paper.compile_prompt.md
请阅读提取内容，提炼核心概念...
```

</div>

<div markdown>

### :material-database-search:{ .middle } 搜索知识库

```
用户: 搜索关于 HTTP/2 的知识
AI:  [调用 search_knowledge 工具]
     找到 3 条相关知识：
     1. HTTP/2 多路复用 (score: 0.92)
     2. HTTP/2 头部压缩 (score: 0.85)
     3. HTTP/2 vs HTTP/1.1 对比 (score: 0.78)
```

</div>

</div>

---

## 社区与资源

<div class="grid cards" markdown>

-   :material-github:{ .lg .middle } **GitHub**

    ---

    源码、Issue 追踪、PR 贡献。

    [:octicons-arrow-right-24: smile9493/Compendium](https://github.com/smile9493/Compendium)

-   :material-file-document:{ .lg .middle } **贡献指南**

    ---

    如何报告 Bug、提交 PR、以及开发环境搭建。

    [:octicons-arrow-right-24: CONTRIBUTING.md](https://github.com/smile9493/Compendium/blob/main/CONTRIBUTING.md)

-   :material-history:{ .lg .middle } **变更日志**

    ---

    各版本新增功能、修复与破坏性变更。

    [:octicons-arrow-right-24: CHANGELOG.md](https://github.com/smile9493/Compendium/blob/main/pdf-module-rs/CHANGELOG.md)

-   :material-file-code:{ .lg .middle } **架构文档**

    ---

    Breakwater Pattern、Workspace 划分、知识引擎内部结构。

    [:octicons-arrow-right-24: ARCHITECTURE.md](https://github.com/smile9493/Compendium/blob/main/pdf-module-rs/ARCHITECTURE.md)

</div>
