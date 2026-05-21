# PDF Module MCP 工具 API 参考

本文档详细描述 PDF Module 提供的全部 MCP 工具的参数、返回值和使用示例。

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
  - [init_knowledge_base](#init_knowledge_base)
  - [compile_to_wiki](#compile_to_wiki)
  - [incremental_compile](#incremental_compile)
  - [save_wiki_entry](#save_wiki_entry)
  - [complete_compile_job](#complete_compile_job)
  - [recompile_entry](#recompile_entry)
  - [aggregate_entries](#aggregate_entries)
  - [lint_wiki](#lint_wiki)
  - [archive_answer](#archive_answer)
  - [check_quality](#check_quality)
  - [micro_compile](#micro_compile)
  - [hypothesis_test](#hypothesis_test)
- [认知索引工具](#认知索引工具)
  - [search_knowledge](#search_knowledge)
  - [rebuild_index](#rebuild_index)
  - [get_entry_context](#get_entry_context)
  - [get_agent_context](#get_agent_context)
  - [get_compilation_context](#get_compilation_context)
  - [preview_wiki_patch](#preview_wiki_patch)
  - [patch_wiki_entry](#patch_wiki_entry)
  - [find_orphans](#find_orphans)
  - [suggest_links](#suggest_links)
  - [export_concept_map](#export_concept_map)
- [管理工具](#管理工具)
  - [get_config](#get_config)
  - [set_config](#set_config)
  - [get_health_report](#get_health_report)
  - [trigger_incremental_compile](#trigger_incremental_compile)
  - [get_compile_status](#get_compile_status)
  - [list_quality_issues](#list_quality_issues)
  - [fix_suggest](#fix_suggest)
  - [apply_quality_gate](#apply_quality_gate)
  - [show_wiki_browser](#show_wiki_browser)
  - [compile_uploaded_pdf](#compile_uploaded_pdf)

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

### init_knowledge_base

初始化一个空的 Karpathy 风格知识库。创建 `schema/`、`wiki/`、`raw/` 目录结构并填充模板文件。

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
      "text": "{\n  \"knowledge_base\": \"/kb\",\n  \"created_files\": [\n    \"schema/AGENTS.md\",\n    \"schema/CLAUDE.md\",\n    \"wiki/index.md\",\n    \"wiki/log.md\",\n    \"raw/.gitkeep\"\n  ],\n  \"skipped_files\": []\n}"
    }
  ]
}
```

**工作流程**

1. 检查目标路径是否为空（非空跳过）
2. 创建 `schema/`、`wiki/`、`raw/` 目录
3. 写入模板文件
4. 初始 `index.md` 和空的 `log.md`

---

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

### save_wiki_entry

创建或更新 wiki 知识条目，支持 YAML front matter（含 `entry_type` / `confidence`）。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 是 | 知识库根目录的绝对路径 |
| `entry_path` | string | 是 | 条目相对路径 (如 `it/concept.md`) |
| `body` | string | 是 | Markdown 正文 |
| `entry_type` | string | 否 | 条目类型：concept / entity / source-summary / comparison / overview |
| `confidence` | string | 否 | 置信度：high / medium / low |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"path\": \"kb/wiki/it/concept.md\",\n  \"entry_type\": \"concept\",\n  \"confidence\": \"high\"\n}"
    }
  ]
}
```

**自动操作**

- 自动附加 YAML front matter
- 调用 `sync_nervous_system` 更新 `index.md` + `log.md`

---

### complete_compile_job

完成编译 job：重建索引、施放质量门禁、生成人类可读综述。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 是 | 知识库根目录的绝对路径 |
| `stage` | string | 是 | 当前阶段：compile / quality / index / done |
| `gate_decision` | string | 否 | quality gate 决策：pass / warn / block |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"stage\": \"done\",\n  \"human_review_summary\": \"编译完成。本 job 处理了 3 个 PDF、生成了 12 个 L1 条目和 1 个 L2 聚合。索引已重建。\",\n  \"quality_issues\": 0,\n  \"index_built\": true\n}"
    }
  ]
}
```

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

### lint_wiki

Karpathy 聚合 lint：同时对知识库进行多项质量检查并返回统一报告。

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
      "text": "{\n  \"orphans\": [\"wiki/unlinked_concept.md\"],\n  \"broken_wikilinks\": [{\"source\": \"wiki/it/nginx.md\", \"target\": \"wiki/unknown.md\"}],\n  \"contradictions\": [{\"entry_a\": \"wiki/it/a.md\", \"entry_b\": \"wiki/it/b.md\"}],\n  \"drift_hints\": [],\n  \"missing_concept_hints\": [{\"entry\": \"wiki/it/http3.md\", \"missing\": [\"QUIC\"]}],\n  \"recommended_research\": [\"QUIC\"]\n}"
    }
  ]
}
```

**检查项目**

- **孤儿条目**: 没有任何入边链接的条目
- **断链**: 引用了不存在的 `[[wikilink]]` 目标
- **矛盾**: 声明相悖的条目对
- **漂移**: 内容一致性显著偏差的条目
- **缺页概念**: 被高频引用但自身不存在的概念

---

### archive_answer

将 AI Agent 的问答对话结果回写为知识库的 overview 页面。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 是 | 知识库根目录的绝对路径 |
| `question` | string | 是 | 用户原始提问 |
| `answer` | string | 是 | AI 回答正文 |
| `references` | string[] | 否 | 引用的 wiki 条目路径列表 |
| `domain` | string | 否 | 领域分类 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"path\": \"kb/wiki/tmp/qa_2026-05-21_http2-explained.md\",\n  \"entry_type\": \"overview\",\n  \"confidence\": \"medium\"\n}"
    }
  ]
}
```

**使用场景**

- LLM 在对话中回答了复杂问题
- 用户希望将优质 QA 持久化到知识库
- 从 `search_knowledge` 结果中生成综述

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

多模态搜索知识库，支持 keyword / semantic / hybrid / wiki_first 四种模式。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 是 | 知识库根目录的绝对路径 |
| `query` | string | 是 | 搜索查询 |
| `mode` | string | 否 | 搜索模式：keyword / semantic / hybrid / wiki_first |
| `limit` | number | 否 | 结果数量限制，默认 10 |

**搜索模式**

| 模式 | 说明 |
|------|------|
| `keyword` | Tantivy 全文检索，CJK n-gram 分词 |
| `semantic` | TF-IDF 向量嵌入相似度检索 |
| `hybrid` | 混合检索（keyword + semantic RRF 融合），默认模式 |
| `wiki_first` | 读取 `index.md` + 图遍历，优先走 wiki 内部链接 |

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

- 四种模式：keyword / semantic / hybrid / wiki_first
- CJK n-gram 分词
- 搜索 title/body/tags/domain
- wiki_first 模式：解析 `index.md` 符号表 + 图邻居排序
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

## 管理工具

### get_config

获取知识库的运行时配置。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 否 | 知识库根目录的绝对路径 |
| `kb_id` | string | 否 | 知识库 ID（与 knowledge_base 二选一，优先使用）|

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"config\": {\n    \"max_entry_size\": \"10485760\",\n    \"language\": \"zh-CN\"\n  },\n  \"total_keys\": 2,\n  \"config_path\": \"/path/to/kb/.rsut_index/config.json\"\n}"
    }
  ]
}
```

---

### set_config

设置知识库的运行时配置值。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 否 | 知识库根目录的绝对路径 |
| `kb_id` | string | 否 | 知识库 ID |
| `key` | string | 是 | 配置键名 |
| `value` | string | 是 | 配置值 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"status\": \"success\",\n  \"key\": \"language\",\n  \"value\": \"en\",\n  \"message\": \"Configuration 'language' updated successfully.\"\n}"
    }
  ]
}
```

---

### get_health_report

获取知识库全面健康报告，包括条目、图谱、索引、质量快照和提取栈状态。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 否 | 知识库根目录的绝对路径 |
| `kb_id` | string | 否 | 知识库 ID |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"total_entries\": 150,\n  \"orphan_count\": 3,\n  \"contradiction_count\": 1,\n  \"broken_link_count\": 0,\n  \"index_size_mb\": 12,\n  \"graph_nodes\": 1200,\n  \"graph_edges\": 3500,\n  \"avg_quality_score\": \"85.2%\",\n  \"domains\": [\"backend\", \"frontend\"],\n  \"last_compile\": \"2026-01-15T10:30:00+00:00\",\n  \"generated_at\": \"2026-01-15T12:00:00+00:00\",\n  \"extraction\": {}\n}"
    }
  ]
}
```

---

### trigger_incremental_compile

手动触发增量编译，检测并编译新的或变更的 PDF 文件。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 否 | 知识库根目录的绝对路径 |
| `kb_id` | string | 否 | 知识库 ID |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"result\": {\n    \"job_id\": \"ef145c16-3ca1-4215-957a-8a7b3c57f88f\",\n    \"pipeline_status\": \"awaiting_agent\",\n    \"new_entries\": 2,\n    \"updated_entries\": 1,\n    \"unchanged_entries\": 100,\n    \"errors\": []\n  }\n}"
    }
  ]
}
```

---

### get_compile_status

获取当前或上次编译任务的状态，包含阶段进度和质量快照。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 否 | 知识库根目录的绝对路径 |
| `kb_id` | string | 否 | 知识库 ID |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"running\": false,\n  \"last_started\": \"2026-01-15T10:30:00+00:00\",\n  \"last_finished\": \"2026-01-15T10:35:00+00:00\",\n  \"progress\": 1.0,\n  \"stats\": {\n    \"new\": 2,\n    \"updated\": 1,\n    \"errors\": 0\n  },\n  \"history\": []\n}"
    }
  ]
}
```

---

### list_quality_issues

列出知识库中的质量问题，支持按严重程度筛选。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 否 | 知识库根目录的绝对路径 |
| `kb_id` | string | 否 | 知识库 ID |
| `severity` | string | 否 | 严重程度过滤（如 "error", "warning"）|
| `limit` | number | 否 | 返回条数上限（默认 50）|

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"issues\": [\n    {\n      \"issue_id\": \"q-001\",\n      \"severity\": \"error\",\n      \"message\": \"Broken internal link: [[missing-page]]\",\n      \"file\": \"wiki/backend/architecture.md\",\n      \"line\": 42\n    }\n  ],\n  \"count\": 1\n}"
    }
  ]
}
```

---

### fix_suggest

针对特定质量问题，建议 MCP 修复操作。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 否 | 知识库根目录的绝对路径 |
| `kb_id` | string | 否 | 知识库 ID |
| `issue_id` | string | 是 | 要修复的问题 ID |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"suggestions\": [],\n  \"issue_id\": \"q-001\"\n}"
    }
  ]
}
```

---

### apply_quality_gate

对所有 Wiki 条目运行发布质量门禁，检查并反馈阻塞条目。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `knowledge_base` | string | 否 | 知识库根目录的绝对路径 |
| `kb_id` | string | 否 | 知识库 ID |
| `job_id` | string | 否 | 关联的编译任务 ID |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"blocked_count\": 0,\n  \"total_entries\": 100,\n  \"passed\": true\n}"
    }
  ]
}
```

---

### show_wiki_browser

打开交互式 Wiki 浏览器 MCP App 资源。

**参数**

无。

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"type\": \"resource\",\n  \"uri\": \"ui://wiki/browser\",\n  \"message\": \"Wiki browser opened. The client should render ui://wiki/browser as an MCP App iframe.\"\n}"
    }
  ]
}
```

---

### compile_uploaded_pdf

编译通过上传 API 提交的 PDF 文件到知识库。

**参数**

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `upload_id` | string | 是 | 上传 API 返回的文件 ID |
| `knowledge_base` | string | 是 | 知识库根目录的绝对路径 |
| `domain` | string | 否 | 领域分类 |

**返回值**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"raw_path\": \"/kb/raw/uploaded_paper.md\",\n  \"entries\": [...],\n  \"source_hash\": \"...\",\n  \"page_count\": 30\n}"
    }
  ]
}
```

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
- **工具总数**: 28
