# MiniMax MCP Plugin

MiniMax MCP插件为PDF Module提供网络搜索和图片理解能力。

## 功能特性

### 1. 网络搜索 (web_search)
- 根据查询词进行网络搜索
- 返回搜索结果和相关建议
- 支持多种搜索引擎

### 2. 图片理解 (understand_image)
- 图片内容理解和分析
- 支持多种图片格式(JPEG、PNG、GIF、WebP)
- 支持HTTP/HTTPS URL和本地文件路径
- 最大支持20MB图片

## 安装配置

### 1. 获取API Key

访问 [Token Plan订阅页面](https://www.minimax.com),订阅套餐并获取专属API Key。

### 2. 安装依赖

#### macOS / Linux
```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

#### Windows
```powershell
powershell -c "irm https://astral.sh/uv/install.ps1 | iex"
```

### 3. 配置插件

在PDF Module的设置页面配置:
- API Key: 输入MiniMax API密钥
- 启用网络搜索: 开启/关闭
- 启用图片理解: 开启/关闭

## 使用方式

### MCP工具调用

#### 网络搜索
```json
{
  "tool": "web_search",
  "parameters": {
    "query": "PDF文本提取最佳实践"
  }
}
```

#### 图片理解
```json
{
  "tool": "understand_image",
  "parameters": {
    "prompt": "请描述这张图片的内容",
    "image_url": "https://example.com/image.png"
  }
}
```

### 本地图片
```json
{
  "tool": "understand_image",
  "parameters": {
    "prompt": "分析这个PDF页面的布局",
    "image_url": "/path/to/local/image.png"
  }
}
```

## 配置文件

### MCP配置示例

```json
{
  "mcpServers": {
    "minimax": {
      "command": "uvx",
      "args": ["minimax-mcp"],
      "env": {
        "MINIMAX_API_KEY": "your-api-key"
      }
    }
  }
}
```

### Docker Compose集成

```yaml
services:
  pdf-api:
    environment:
      - MINIMAX_API_KEY=${MINIMAX_API_KEY}
      - MINIMAX_WEB_SEARCH_ENABLED=true
      - MINIMAX_IMAGE_UNDERSTAND_ENABLED=true
```

## API参考

### web_search

**参数:**
| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| query | string | ✓ | 搜索查询词 |

**返回:**
```json
{
  "results": [
    {
      "title": "结果标题",
      "url": "https://...",
      "snippet": "结果摘要"
    }
  ],
  "related_searches": ["相关搜索建议"]
}
```

### understand_image

**参数:**
| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| prompt | string | ✓ | 对图片的提问或分析要求 |
| image_url | string | ✓ | 图片URL或本地路径 |

**返回:**
```json
{
  "analysis": "图片分析结果",
  "metadata": {
    "format": "PNG",
    "size": "1920x1080",
    "file_size": "2.5MB"
  }
}
```

## 与PDF Module集成

### 1. PDF文档搜索增强

结合PDF文本提取和网络搜索:
```
1. 提取PDF关键词
2. 使用web_search搜索相关资料
3. 整合PDF内容和搜索结果
```

### 2. PDF页面视觉分析

使用图片理解分析PDF页面:
```
1. 将PDF页面转换为图片
2. 使用understand_image分析布局
3. 提取表格、图表等结构化信息
```

### 3. 智能文档处理

结合多个工具:
```
1. extract_text - 提取PDF文本
2. web_search - 搜索背景资料
3. understand_image - 分析图表图片
4. 生成综合报告
```

## 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| MINIMAX_API_KEY | - | MiniMax API密钥 |
| MINIMAX_BASE_URL | https://api.minimax.chat | API基础URL |
| MINIMAX_WEB_SEARCH_ENABLED | true | 启用网络搜索 |
| MINIMAX_IMAGE_UNDERSTAND_ENABLED | true | 启用图片理解 |
| MINIMAX_TIMEOUT | 30 | 请求超时(秒) |
| MINIMAX_MAX_IMAGE_SIZE | 20 | 最大图片大小(MB) |

## 错误处理

### 常见错误

1. **API Key无效**
   - 检查API Key是否正确
   - 确认订阅是否有效

2. **图片大小超限**
   - 压缩图片到20MB以下
   - 使用支持的格式

3. **网络请求失败**
   - 检查网络连接
   - 增加超时时间

## 最佳实践

1. **批量处理**
   - 使用队列处理多个图片
   - 避免并发请求过多

2. **缓存优化**
   - 缓存搜索结果
   - 缓存图片分析结果

3. **错误重试**
   - 实现指数退避重试
   - 记录失败请求

## 许可证

MIT License

## 支持

- 官方文档: https://www.minimax.com/docs
- GitHub Issues: https://github.com/minimax/mcp/issues
