# PDF Module MCP 工具 API 参考

本文档详细描述 PDF Module 提供的全部 20 个 MCP 工具的参数、返回值和使用示例。

---

## 目录

- [PDF 提取工具](#pdf-提取工具)
  - [extract_text](#extract_text)
  - [extract_structured](#extract_structured)
  - [get_page_count](#get_page_count)
  - [search_keywords](#search_keywords)
  - [extrude_to_server_wiki](#extrude_to_server_wiki)
  - [extrude_to_agent_payload](#extrude_to_agent_payload)
- [知识编译工具](#知识编译工具)
  - [compile_to_wiki](#compile_to_wiki)
  - [incremental_compile](#incremental_compile)
  - [recompile_entry](#recompile_entry)
  - [aggregate_entries](#aggregate_entries)
  - [check_quality](#check_quality)
  - [micro_compile](#micro_compile)
  - [hypothesis_test](#hypothesis_test)
- [认知索引工具](#认知索引工具)
  - [search_knowledge](#search_knowledge)
  - [rebuild_index](#rebuild_index)
  - [get_entry_context](#get_entry_context)
  - [find_orphans](#find_orphans)
  - [suggest_links](#suggest_links)
  - [export_concept_map](#export_concept_map)

---

## PDF 提取工具

### extract_text

提取 PDF 文件的纯文本内容。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `file_path` | string | 是 | PDF 文件的绝对路径 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "提取的文本内容..."
    }
  ]
}
```

**错误**

| 错误码 | 描述 |
|--------|------|
| `invalid_params` | 缺少 file_path 参数 |
| `file_not_found` | 文件不存在 |
| `extraction_failed` | PDF 解析失败 |

**示例**

```json
{
  "name": "extract_text",
  "arguments": {
    "file_path": "/path/to/document.pdf"
  }
}
```

---

### extract_structured

提取 PDF 文件的结构化数据，包含每页文本和边界框信息。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `file_path` | string | 是 | PDF 文件的绝对路径 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"extracted_text\": \"完整文本...\",\n  \"page_count\": 10,\n  \"pages\": [\n    {\n      \"page_number\": 1,\n      \"text\": \"第1页文本...\",\n      \"bbox\": [0.0, 0.0, 612.0, 792.0]\n    }\n  ],\n  \"file_info\": {\n    \"size\": 1024000,\n    \"modified\": \"2026-05-04T00:00:00Z\"\n  }\n}"
    }
  ]
}
```

---

### get_page_count

获取 PDF 文件的页数。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `file_path` | string | 是 | PDF 文件的绝对路径 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "42"
    }
  ]
}
```

---

### search_keywords

在 PDF 文件中搜索关键词，返回匹配位置和上下文。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `file_path` | string | 是 | PDF 文件的绝对路径 |
| `keywords` | array[string] | 是 | 关键词列表 |
| `case_sensitive` | boolean | 否 | 是否区分大小写，默认 false |
| `context_length` | number | 否 | 匹配上下文长度，默认 50 字符 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"total_matches\": 15,\n  \"pages_with_matches\": 8,\n  \"matches\": [\n    {\n      \"keyword\": \"HTTP/2\",\n      \"page\": 3,\n      \"position\": 1234,\n      \"context\": \"...HTTP/2 多路复用允许...\"\n    }\n  ]\n}"
    }
  ]
}
```

---

### extrude_to_server_wiki

提取 PDF 内容到服务端 Wiki 的 raw/ 目录。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `file_path` | string | 是 | PDF 文件的绝对路径 |
| `wiki_base_path` | string | 否 | Wiki 基础路径，默认 `./wiki` |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"status\": \"success\",\n  \"raw_path\": \"/kb/raw/paper.md\",\n  \"index_path\": \"/kb/wiki/index.md\",\n  \"log_path\": \"/kb/wiki/log.md\",\n  \"page_count\": 45,\n  \"message\": \"PDF extracted to raw/. AI Agent should process and create wiki entries.\"\n}"
    }
  ]
}
```

---

### extrude_to_agent_payload

提取 PDF 内容并返回 Markdown 格式的编译提示。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `file_path` | string | 是 | PDF 文件的绝对路径 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "# PDF 提取完成\n\n## 任务说明\n\n你是一个专业的知识库管理员...\n\n## 元数据\n\n| 字段 | 值 |\n|------|-----|\n| 文档名称 | paper |\n| 页数 | 45 |\n...\n\n# 提取内容\n\n..."
    }
  ]
}
```

---

## 知识编译工具

### compile_to_wiki

将 PDF 编译到知识库，这是 Karpathy 编译器模式的核心入口。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `pdf_path` | string | 是 | PDF 文件的绝对路径 |
| `knowledge_base` | string | 是 | 知识库根目录的绝对路径 |
| `domain` | string | 否 | 领域分类，默认 `未分类` |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"raw_path\": \"/kb/raw/paper.md\",\n  \"entries\": [\n    {\n      \"title\": \"paper\",\n      \"domain\": \"IT\",\n      \"path\": \"/kb/raw/paper.compile_prompt.md\",\n      \"status\": \"pending\"\n    }\n  ],\n  \"source\": \"/path/to/paper.pdf\",\n  \"source_hash\": \"abc123def456...\",\n  \"page_count\": 45\n}"
    }
  ]
}
```

**工作流程**

1. 提取 PDF 文本
2. 保存到 `raw/` 目录
3. 生成编译提示文件
4. 更新哈希缓存

---

### incremental_compile

扫描 raw/ 目录，增量编译新增或变更的 PDF。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 是 | 知识库根目录的绝对路径 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"total_scanned\": 10,\n  \"compiled\": 3,\n  \"skipped\": 7,\n  \"results\": [\n    {\n      \"raw_path\": \"/kb/raw/new.pdf.md\",\n      \"entries\": [...],\n      \"source_hash\": \"...\",\n      \"page_count\": 20\n    }\n  ]\n}"
    }
  ]
}
```

**增量检测机制**

- 使用 SHA-256 哈希检测文件变更
- 缓存存储在 `.hash_cache`
- 只编译哈希变更的文件

---

### recompile_entry

重新编译单个知识条目，用于质量漂移修正。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 是 | 知识库根目录的绝对路径 |
| `entry_path` | string | 是 | 条目相对路径 (如 `it/concept.md`) |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"entry_path\": \"/kb/wiki/it/concept.md\",\n  \"version\": 2,\n  \"title\": \"概念名称\",\n  \"domain\": \"IT\",\n  \"source_changed\": true,\n  \"source_exists\": true,\n  \"backup_path\": \"/kb/wiki/.versions/concept_v1.md\",\n  \"recompile_prompt\": \"## 重编译指令\\n\\n请根据以下信息...\"\n}"
    }
  ]
}
```

**特性**

- 自动备份旧版本到 `.versions/`
- 版本号自动递增
- 检测源文件是否变更

---

### aggregate_entries

发现可聚合的 L1 条目簇，用于构建 L2 综述。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 是 | 知识库根目录的绝对路径 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"candidates\": [\n    {\n      \"domain\": \"IT\",\n      \"entry_paths\": [\"it/http2_multiplex.md\", \"it/http2_header.md\"],\n      \"suggested_title\": \"IT 领域综合: HTTP/2 协议\"\n    }\n  ],\n  \"total_clusters\": 1,\n  \"instructions\": \"For each cluster, create an L2 summary entry...\"\n}"
    }
  ]
}
```

**聚合算法**

- 基于标签共现 (Jaccard ≥ 0.3)
- 同领域内聚类
- 最小簇大小为 2

---

### check_quality

扫描知识库质量，检测问题条目。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 是 | 知识库根目录的绝对路径 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"total_entries\": 156,\n  \"avg_quality_score\": \"82.5%\",\n  \"domains\": [\"IT\", \"Math\", \"Network\"],\n  \"issues_count\": 12,\n  \"orphan_count\": 3,\n  \"broken_links_count\": 2,\n  \"report_markdown\": \"# Knowledge Quality Report\\n\\n...\",\n  \"has_errors\": false,\n  \"has_warnings\": true\n}"
    }
  ]
}
```

**检测项目**

- 缺失标题/领域/标签
- 质量分为 0
- 孤立条目
- 失效链接

---

### micro_compile

即时 PDF 提取，结果仅注入对话不写入知识库。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `pdf_path` | string | 是 | PDF 文件的绝对路径 |
| `page_range` | string | 否 | 页码范围 (如 `1-5` 或 `3,7,12`) |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "# 微编译结果: paper\n\n> 注意: 此内容仅用于当前对话上下文，不会保存到 wiki。\n\n- 页数: 45\n- 提取范围: 1-5\n\n---\n\n## Page 1\n\n第1页内容...\n\n## Page 2\n\n第2页内容...\n..."
    }
  ]
}
```

**使用场景**

- 快速查看 PDF 片段
- 跨领域临时查询
- 不污染知识库

---

### hypothesis_test

发现知识库中的矛盾观点对。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 是 | 知识库根目录的绝对路径 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"contradiction_pairs\": [\n    {\n      \"entry_a\": \"it/microservices.md\",\n      \"entry_b\": \"it/monolith.md\",\n      \"title_a\": \"微服务优势\",\n      \"title_b\": \"单体架构优势\"\n    }\n  ],\n  \"total\": 1,\n  \"instructions\": \"For each pair, read both entries and conduct a structured debate...\"\n}"
    }
  ]
}
```

**矛盾检测**

- 基于 `contradictions` 字段
- 双向关联去重
- 提供辩论框架

---

## 认知索引工具

### search_knowledge

Tantivy 全文搜索知识库。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 是 | 知识库根目录的绝对路径 |
| `query` | string | 是 | 搜索查询 |
| `limit` | number | 否 | 结果数量限制，默认 10 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "[\n  {\n    \"path\": \"it/http2_multiplex.md\",\n    \"title\": \"HTTP/2 多路复用\",\n    \"domain\": \"IT\",\n    \"score\": 0.95,\n    \"snippet\": \"...HTTP/2 多路复用允许...\"\n  }\n]"
    }
  ]
}
```

**搜索特性**

- CJK n-gram 分词
- 搜索 title/body/tags/domain
- 自动重建空索引

---

### rebuild_index

完全重建所有索引。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 是 | 知识库根目录的绝对路径 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"status\": \"success\",\n  \"fulltext_entries_indexed\": 156,\n  \"graph_nodes\": 156,\n  \"graph_edges\": 89,\n  \"message\": \"All indexes rebuilt from wiki/ files.\"\n}"
    }
  ]
}
```

**重建内容**

- Tantivy 全文索引
- petgraph 知识图谱
- 标签共现边

---

### get_entry_context

获取条目的 N 跳邻居。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 是 | 知识库根目录的绝对路径 |
| `entry_path` | string | 是 | 条目相对路径 |
| `hops` | number | 否 | 跳数，默认 2 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"entry\": \"it/http2_multiplex.md\",\n  \"hops\": 2,\n  \"neighbors\": [\n    {\n      \"path\": \"it/http2_header.md\",\n      \"title\": \"HTTP/2 头部压缩\",\n      \"domain\": \"IT\",\n      \"hops\": 1,\n      \"edge_kind\": \"related\"\n    },\n    {\n      \"path\": \"it/tcp.md\",\n      \"title\": \"TCP 连接\",\n      \"domain\": \"IT\",\n      \"hops\": 1,\n      \"edge_kind\": \"tag_cooccurrence\"\n    }\n  ],\n  \"total\": 2\n}"
    }
  ]
}
```

**边类型**

- `related`: 显式关联
- `contradiction`: 矛盾关系
- `tag_cooccurrence`: 标签共现

---

### find_orphans

检测孤立条目。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 是 | 知识库根目录的绝对路径 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"orphan_count\": 3,\n  \"entries\": [\n    \"it/legacy_protocol.md\",\n    \"math/old_theorem.md\"\n  ],\n  \"message\": \"3 entries have no links. Consider integrating them.\"\n}"
    }
  ]
}
```

---

### suggest_links

为条目推荐潜在链接。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 是 | 知识库根目录的绝对路径 |
| `entry_path` | string | 是 | 条目相对路径 |
| `top_k` | number | 否 | 返回数量，默认 10 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"entry\": \"it/http2_multiplex.md\",\n  \"suggestions\": [\n    {\n      \"from\": \"it/http2_multiplex.md\",\n      \"to\": \"it/quic.md\",\n      \"score\": 0.65,\n      \"reason\": \"Shared tags: http, protocol, networking\"\n    }\n  ],\n  \"total\": 1\n}"
    }
  ]
}
```

**推荐算法**

- Jaccard 相似度
- 基于标签计算
- 过滤已存在链接

---

### export_concept_map

导出 Mermaid.js 格式的概念图。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 是 | 知识库根目录的绝对路径 |
| `entry_path` | string | 是 | 中心条目相对路径 |
| `depth` | number | 否 | 深度，默认 2 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"entry\": \"it/http2_multiplex.md\",\n  \"depth\": 2,\n  \"mermaid\": \"graph LR\\n    n0[\\\"HTTP/2 多路复用\\\"]:::center\\n    n1[\\\"HTTP/2 头部压缩\\\"]\\n    n0 -->|relates| n1\\n    classDef center fill:#f96,stroke:#333,stroke-width:2px\",\n  \"usage\": \"Paste the mermaid field into any Mermaid.js renderer\"\n}"
    }
  ]
}
```

**渲染方式**

- Obsidian 代码块
- GitHub Markdown
- mermaid.live

---

## 错误码参考

| 错误码 | 描述 |
|--------|------|
| `parse_error` | JSON 解析失败 |
| `invalid_params` | 参数缺失或无效 |
| `method_not_found` | 未知工具名称 |
| `internal_error` | 内部错误 |

---

## 版本信息

- **协议版本**: MCP 2024-11-05
- **服务器版本**: 0.6.0
- **工具总数**: 20
