# 知识编译指南

本文档详细说明 PDF Module 的知识编译流程、最佳实践和高级用法。

---

## 一、核心概念

### 1.1 什么是知识编译？

知识编译是将非结构化的 PDF 文档转换为结构化、可搜索、可关联的知识条目的过程。这是 **Karpathy 编译器模式** 的核心实现。

```
传统 RAG 模式:
PDF → 向量索引 → 相似度检索 → 生成答案
(知识不可累积，每次都是一次性检索)

编译器模式:
PDF → Markdown 知识库 → 索引 → 搜索/关联/推理
(知识可累积、可解释、可推理)
```

### 1.2 知识金字塔

```
┌─────────────────────────────────────┐
│  L3 Domain Map (领域导航)            │  ← 每个领域 1 个
│  例: IT领域知识地图.md               │
├─────────────────────────────────────┤
│  L2 Aggregation (综述条目)           │  ← 同主题 L1 聚合
│  例: HTTP_2协议详解.md               │
├─────────────────────────────────────┤
│  L1 Atomic Concept (原子概念)        │  ← 核心知识单元
│  例: HTTP_2多路复用.md               │
├─────────────────────────────────────┤
│  L0 Raw Extraction (原始提取)        │  ← PDF 直接提取
│  例: raw/rfc7540.md                 │
└─────────────────────────────────────┘
```

---

## 二、编译流程

### 2.1 基本流程

```
1. compile_to_wiki     → 提取 PDF，生成 raw/ 和编译提示
2. AI Agent 处理       → 阅读提示，创建 L1 原子条目
3. search_knowledge    → 验证条目可搜索
4. aggregate_entries   → 发现 L2 聚合候选
5. AI Agent 聚合       → 创建 L2 综述条目
```

### 2.2 目录结构

```
knowledge_base/
├── raw/                        # 原始提取
│   ├── paper.md               # PDF 提取内容
│   └── paper.compile_prompt.md # AI 编译提示
│
├── wiki/                       # 编译后知识
│   ├── index.md               # 全局导航 (自动生成)
│   ├── log.md                 # 操作日志
│   ├── .versions/             # 重编译备份
│   │   └── concept_v1.md
│   └── it/                    # 领域目录
│       ├── http2_multiplex.md
│       └── http2_header.md
│
├── schema/                     # 编译指令 (可选)
│   └── CLAUDE.md              # AI Agent 规范
│
├── .hash_cache                 # 增量编译缓存
│
└── .rsut_index/               # 可重建索引
    └── tantivy/               # 全文搜索索引
```

---

## 三、条目规范

### 3.1 YAML Front Matter

每个 wiki 条目必须包含以下元数据：

```yaml
---
title: "HTTP/2 多路复用"
domain: "IT"
category: "networking/protocols"
source: "raw/rfc7540.pdf"
page: 12
source_hash: "abc123def456..."
tags: ["http", "networking", "protocol", "multiplexing"]
level: L1
status: compiled
quality_score: 0.85
version: 1
contradictions: []
related: ["wiki/it/http1_pipelining.md", "wiki/it/quic.md"]
aggregated_from: []
created: 2026-05-04T10:00:00Z
updated: 2026-05-04T10:00:00Z
---

# HTTP/2 多路复用

正文内容...
```

### 3.2 字段说明

| 字段 | 必填 | 说明 |
|------|------|------|
| `title` | ✅ | 条目标题 |
| `domain` | ✅ | 领域分类 (IT/Math/Network 等) |
| `category` | ❌ | 层级分类 (如 networking/protocols) |
| `source` | ❌ | 来源 PDF 相对路径 |
| `page` | ❌ | 来源页码 |
| `source_hash` | ❌ | 来源文件 SHA-256 |
| `tags` | ✅ | 标签列表 (至少 1 个) |
| `level` | ✅ | 层级 (L0/L1/L2/L3) |
| `status` | ✅ | 状态 (pending/compiled/needs_recompile) |
| `quality_score` | ❌ | 质量分 (0.0-1.0) |
| `version` | ❌ | 版本号 |
| `contradictions` | ❌ | 矛盾条目路径列表 |
| `related` | ❌ | 相关条目路径列表 |
| `aggregated_from` | ❌ | L2 条目来源 L1 列表 |

### 3.3 命名规范

```
文件名格式: [领域] 概念名称.md

✅ 推荐:
[IT] HTTP_2_多路复用.md
[Math] 贝叶斯定理.md
[Network] TCP_三次握手.md

❌ 避免:
第3章_概述.md
paper_summary.md
2024-05-04_note.md
```

---

## 四、编译工作流

### 4.1 首次编译

**步骤 1: 调用 compile_to_wiki**

```
用户: 帮我把 /path/to/paper.pdf 编译到知识库，领域是 IT

AI: [调用 compile_to_wiki]
    pdf_path: /path/to/paper.pdf
    knowledge_base: /kb
    domain: IT

返回:
    raw_path: /kb/raw/paper.md
    compile_prompt: /kb/raw/paper.compile_prompt.md
```

**步骤 2: AI Agent 处理编译提示**

```
AI 阅读 paper.compile_prompt.md，执行:

1. 深度通读提取内容
2. 提炼 10-15 个核心概念
3. 检查 wiki/ 是否已存在相关条目
4. 创建/更新原子条目
5. 更新 wiki/index.md 和 wiki/log.md
```

**步骤 3: 验证编译结果**

```
用户: 搜索 HTTP/2 相关知识

AI: [调用 search_knowledge]
    query: HTTP/2
    limit: 10

返回匹配条目，验证编译成功
```

### 4.2 增量编译

```
用户: 我添加了 3 个新 PDF，帮我增量编译

AI: [调用 incremental_compile]
    knowledge_base: /kb

返回:
    total_scanned: 3
    compiled: 2      # 新文件
    skipped: 1       # 哈希未变更
```

### 4.3 重编译单条目

```
用户: 重新编译 it/http2_multiplex.md，源文件有更新

AI: [调用 recompile_entry]
    knowledge_base: /kb
    entry_path: it/http2_multiplex.md

返回:
    version: 2
    source_changed: true
    backup_path: .versions/http2_multiplex_v1.md
```

---

## 五、知识关联

### 5.1 显式关联

在 front matter 中手动指定：

```yaml
related:
  - wiki/it/http1_pipelining.md
  - wiki/it/quic.md
contradictions:
  - wiki/it/spdy_protocol.md
```

### 5.2 自动发现关联

```
用户: 分析 HTTP/2 多路复用的相关知识

AI: [调用 get_entry_context]
    entry_path: it/http2_multiplex.md
    hops: 2

返回 2 跳内的所有关联条目
```

### 5.3 链接建议

```
用户: 为 HTTP/2 多路复用推荐相关链接

AI: [调用 suggest_links]
    entry_path: it/http2_multiplex.md
    top_k: 10

返回基于标签相似度的链接建议
```

---

## 六、质量监控

### 6.1 质量检查

```
用户: 检查知识库质量

AI: [调用 check_quality]
    knowledge_base: /kb

返回:
    total_entries: 156
    avg_quality_score: 82.5%
    issues_count: 12
    orphan_count: 3
    broken_links_count: 2
```

### 6.2 质量问题类型

| 类型 | 严重度 | 说明 |
|------|--------|------|
| 缺失标题 | ERROR | title 为空 |
| 缺失领域 | ERROR | domain 为空 |
| 无标签 | WARN | tags 为空 |
| 质量分为 0 | INFO | quality_score 未评估 |
| 孤立条目 | WARN | 无任何链接 |
| 失效链接 | ERROR | 引用的路径不存在 |

### 6.3 质量分评估标准

| 分数 | 等级 | 标准 |
|------|------|------|
| 0.9-1.0 | 优秀 | 内容完整、标签丰富、链接完善 |
| 0.7-0.9 | 良好 | 内容完整、有标签、部分链接 |
| 0.5-0.7 | 一般 | 内容基本完整、标签较少 |
| 0-0.5 | 待改进 | 内容不完整或缺少元数据 |

---

## 七、高级用法

### 7.1 L1 → L2 聚合

```
用户: 发现可以聚合的原子概念

AI: [调用 aggregate_entries]
    knowledge_base: /kb

返回:
    candidates: [
        {
            domain: "IT",
            entry_paths: ["it/http2_multiplex.md", "it/http2_header.md"],
            suggested_title: "IT 领域综合: HTTP/2 协议"
        }
    ]
```

AI Agent 根据候选创建 L2 综述条目：

```yaml
---
title: "HTTP/2 协议详解"
domain: "IT"
level: L2
aggregated_from:
  - wiki/it/http2_multiplex.md
  - wiki/it/http2_header.md
  - wiki/it/http2_stream.md
---
```

### 7.2 矛盾推理

```
用户: 检查知识库中的矛盾观点

AI: [调用 hypothesis_test]
    knowledge_base: /kb

返回:
    contradiction_pairs: [
        {
            entry_a: "it/microservices_benefits.md",
            entry_b: "it/monolith_advantages.md"
        }
    ]
```

### 7.3 概念图导出

```
用户: 导出 HTTP/2 相关的概念图

AI: [调用 export_concept_map]
    entry_path: it/http2_multiplex.md
    depth: 2

返回 Mermaid.js 格式的概念图
```

---

## 八、最佳实践

### 8.1 条目粒度

```
✅ 原子化概念 (推荐):
- [IT] HTTP_2_多路复用.md
- [IT] HTTP_2_头部压缩.md
- [IT] HTTP_2_流控制.md

❌ 过于宽泛:
- [IT] HTTP_2.md (包含所有内容)

❌ 过于细碎:
- [IT] HTTP_2_帧类型_DATA.md
- [IT] HTTP_2_帧类型_HEADERS.md
```

### 8.2 标签策略

```yaml
tags:
  - 核心概念    # http, tcp, algorithm
  - 技术栈      # nginx, rust, python
  - 关系类型    # protocol, architecture, pattern
  - 领域细分    # frontend, backend, devops
```

### 8.3 版本管理

- 每次重编译自动备份到 `.versions/`
- 版本号自动递增
- 保留历史版本用于对比

---

## 九、故障排查

### 9.1 常见问题

| 问题 | 原因 | 解决 |
|------|------|------|
| 搜索无结果 | 索引未构建 | 调用 `rebuild_index` |
| 增量编译跳过所有 | 缓存问题 | 删除 `.hash_cache` |
| 条目显示孤立 | 缺少 related | 调用 `suggest_links` |
| 质量分为 0 | 未评估 | 手动设置或重编译 |

### 9.2 索引重建

```
用户: 重建所有索引

AI: [调用 rebuild_index]
    knowledge_base: /kb

返回:
    fulltext_entries_indexed: 156
    graph_nodes: 156
    graph_edges: 89
```

---

## 十、相关文档

- [API 参考](./API_REFERENCE.md)
- [AI Agent 工作流指南](./AI_AGENT_SETUP_GUIDE.md)
- [架构设计](../pdf-module-rs/ARCHITECTURE.md)
