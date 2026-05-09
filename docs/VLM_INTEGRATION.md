# VLM 集成指南

本文档说明如何配置和使用 PDF Module 的 VLM (Vision Language Model) 视觉增强功能。

---

## 一、概述

### 1.1 什么是 VLM 增强？

VLM 增强是一种可选的 PDF 处理升级方案，通过视觉语言模型对 PDF 页面进行布局分析，提升复杂文档的提取质量。

```
基础模式 (pdfium only):
PDF → 文本提取 → 输出

VLM 增强模式:
PDF → 文本提取 + 页面渲染 → VLM 布局分析 → 合并结果 → 输出
```

### 1.2 适用场景

| 场景 | 基础模式 | VLM 增强 |
|------|----------|----------|
| 纯文本文档 | ✅ 推荐 | 不必要 |
| 扫描件 PDF | ❌ 效果差 | ✅ 推荐 |
| 复杂布局 (多栏/表格) | ⚠️ 可能混乱 | ✅ 推荐 |
| 图文混排 | ❌ 丢失图片信息 | ✅ 推荐 |
| 手写批注 | ❌ 无法识别 | ✅ 推荐 |

---

## 二、支持的模型

### 2.1 模型列表

| 模型 | 提供商 | 特点 | 推荐场景 |
|------|--------|------|----------|
| `glm-4v-flash` | 智谱 AI | 免费、快速 | 日常使用 |
| `glm-4v-plus` | 智谱 AI | 高精度 | 复杂文档 |
| `gpt-4o` | OpenAI | 通用性强 | 国际文档 |
| `claude-3.5-sonnet` | Anthropic | 推理能力强 | 学术论文 |

### 2.2 模型选择建议

```
中文文档 → glm-4v-flash / glm-4v-plus
英文文档 → gpt-4o / claude-3.5-sonnet
学术论文 → claude-3.5-sonnet
快速预览 → glm-4v-flash
高精度需求 → glm-4v-plus / gpt-4o
```

---

## 三、配置方法

### 3.1 环境变量配置

**智谱 AI (推荐)**

```json
// ~/.cursor/mcp.json
{
  "mcpServers": {
    "pdf-module": {
      "command": "/path/to/pdf-mcp",
      "env": {
        "VLM_API_KEY": "your_zhipu_api_key",
        "VLM_MODEL": "glm-4v-flash",
        "VLM_ENDPOINT": "https://open.bigmodel.cn/api/paas/v4/chat/completions",
        "VLM_TIMEOUT_SECS": "30",
        "VLM_MAX_CONCURRENCY": "5"
      }
    }
  }
}
```

**OpenAI**

```json
{
  "mcpServers": {
    "pdf-module": {
      "command": "/path/to/pdf-mcp",
      "env": {
        "VLM_API_KEY": "sk-xxx",
        "VLM_MODEL": "gpt-4o",
        "VLM_ENDPOINT": "https://api.openai.com/v1/chat/completions",
        "VLM_TIMEOUT_SECS": "60"
      }
    }
  }
}
```

**Anthropic**

```json
{
  "mcpServers": {
    "pdf-module": {
      "command": "/path/to/pdf-mcp",
      "env": {
        "VLM_API_KEY": "sk-ant-xxx",
        "VLM_MODEL": "claude-3.5-sonnet",
        "VLM_ENDPOINT": "https://api.anthropic.com/v1/messages",
        "VLM_TIMEOUT_SECS": "60"
      }
    }
  }
}
```

### 3.2 环境变量说明

| 变量 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `VLM_API_KEY` | ✅ | - | API 密钥 |
| `VLM_MODEL` | ❌ | `glm-4v-flash` | 模型名称 |
| `VLM_ENDPOINT` | ❌ | 智谱 API | API 端点 |
| `VLM_TIMEOUT_SECS` | ❌ | `30` | 请求超时 (秒) |
| `VLM_MAX_CONCURRENCY` | ❌ | `5` | 最大并发数 |
| `VLM_MAX_RETRIES` | ❌ | `3` | 最大重试次数 |
| `VLM_ENABLE_THINKING` | ❌ | `false` | 启用思考模式 |
| `VLM_ENABLE_FUNCTION_CALL` | ❌ | `false` | 启用函数调用 |

---

## 四、工作原理

### 4.1 处理流程

```
┌─────────────────────────────────────────────────────────────┐
│                      PDF 输入                                │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│              PdfiumEngine (本地 FFI)                         │
│  ┌─────────────────┐    ┌─────────────────────────────────┐ │
│  │  文本提取        │    │  页面渲染 (RGBA 像素)           │ │
│  │  (所有页面)      │    │  (按需，DPI=150)               │ │
│  └────────┬────────┘    └────────────────┬────────────────┘ │
└───────────┼──────────────────────────────┼──────────────────┘
            │                              │
            │                              ▼
            │              ┌───────────────────────────────────┐
            │              │        VlmGateway                  │
            │              │  ┌─────────────────────────────┐   │
            │              │  │  Base64 编码                │   │
            │              │  │  构建 API 请求              │   │
            │              │  │  并发控制 (Semaphore)       │   │
            │              │  │  重试 + 指数退避            │   │
            │              │  └─────────────────────────────┘   │
            │              └────────────────┬──────────────────┘
            │                               │
            │                               ▼
            │              ┌───────────────────────────────────┐
            │              │     VLM API (远程)                 │
            │              │  返回: 区域类型 + 坐标 + 内容      │
            │              └────────────────┬──────────────────┘
            │                               │
            ▼                               ▼
┌─────────────────────────────────────────────────────────────┐
│                    结果合并                                  │
│  - 文本内容合并                                              │
│  - 区域标注                                                  │
│  - 阅读顺序推断                                              │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    结构化输出                                │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 降级策略

当 VLM 不可用时，系统自动降级到基础模式：

```
VLM 不可用原因:
- API Key 未配置
- 网络超时
- API 限流
- 模型不可用

降级行为:
1. 记录降级事件到日志
2. 使用 pdfium 结果继续
3. 返回降级标记
```

---

## 五、使用示例

### 5.1 自动 VLM 增强

当配置了 VLM 环境变量后，系统会自动检测扫描件并启用 VLM：

```
用户: 提取 /path/to/scanned.pdf

AI: [调用 extract_structured]
    
系统自动:
1. 检测到扫描件 (文本提取量 < 阈值)
2. 启用 VLM 布局分析
3. 合并结果返回
```

### 5.2 强制 VLM 模式

```
用户: 用 VLM 模式提取这个 PDF

AI: [调用 extract_structured]
    (系统检测到 VLM 配置，自动启用)
```

### 5.3 查看处理状态

VLM 处理结果会包含元数据：

```json
{
  "extracted_text": "...",
  "page_count": 10,
  "extraction_metadata": {
    "vlm_used": true,
    "vlm_model": "glm-4v-flash",
    "vlm_pages_processed": 10,
    "vlm_degraded": false
  }
}
```

---

## 六、性能优化

### 6.1 并发控制

```bash
# 低配置机器
VLM_MAX_CONCURRENCY=2

# 高配置机器
VLM_MAX_CONCURRENCY=10
```

### 6.2 超时设置

```bash
# 快速模型
VLM_TIMEOUT_SECS=30

# 慢速模型 / 大文档
VLM_TIMEOUT_SECS=120
```

### 6.3 重试策略

```bash
# 默认重试
VLM_MAX_RETRIES=3

# 不重试 (快速失败)
VLM_MAX_RETRIES=0
```

---

## 七、成本控制

### 7.1 模型成本对比

| 模型 | 输入价格 | 输出价格 | 100页估算 |
|------|----------|----------|-----------|
| glm-4v-flash | 免费 | 免费 | ¥0 |
| glm-4v-plus | ¥0.01/千token | ¥0.01/千token | ~¥2 |
| gpt-4o | $2.5/百万token | $10/百万token | ~$0.5 |
| claude-3.5-sonnet | $3/百万token | $15/百万token | ~$0.8 |

### 7.2 成本优化建议

1. **使用免费模型**: `glm-4v-flash` 适合大多数场景
2. **选择性启用**: 只对扫描件启用 VLM
3. **批量处理**: 减少重复请求
4. **缓存结果**: 避免重复处理同一 PDF

---

## 八、故障排查

### 8.1 常见错误

| 错误 | 原因 | 解决 |
|------|------|------|
| `VLM_API_KEY not set` | 未配置 API Key | 设置环境变量 |
| `Timeout` | 请求超时 | 增加 `VLM_TIMEOUT_SECS` |
| `Rate limit` | API 限流 | 降低 `VLM_MAX_CONCURRENCY` |
| `Model not found` | 模型名称错误 | 检查 `VLM_MODEL` |
| `Network error` | 网络问题 | 检查网络连接 |

### 8.2 调试模式

启用详细日志：

```bash
RUST_LOG=pdf_module=debug,vlm_gateway=trace
```

### 8.3 测试 VLM 连接

```bash
# 测试 API 连通性
curl -X POST https://open.bigmodel.cn/api/paas/v4/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model": "glm-4v-flash", "messages": [{"role": "user", "content": "test"}]}'
```

---

## 九、安全建议

### 9.1 API Key 管理

```
✅ 推荐:
- 使用环境变量
- 配置在服务端
- 定期轮换

❌ 避免:
- 硬编码在代码中
- 提交到 Git 仓库
- 在客户端配置
```

### 9.2 数据隐私

- PDF 内容会发送到 VLM API
- 敏感文档建议使用本地模型
- 了解各提供商的隐私政策

---

## 十、相关文档

- [客户端配置指南](./CLIENT_SETUP_GUIDE.md)
- [API 参考](./API_REFERENCE.md)
- [架构设计](../pdf-module-rs/ARCHITECTURE.md)
