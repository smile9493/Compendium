# PDF MCP Docker 部署指南

## 快速开始

### 1. 构建镜像

```bash
cd /opt/pdf-module/pdf-module-rs

# 先编译二进制文件
cargo build --release --bin pdf-mcp --bin pdf-web

# 构建Docker镜像
cd target/release
docker build -f /opt/pdf-module/docker/Dockerfile.prebuilt-full -t pdf-mcp-full:latest .
```

### 2. 使用 Docker Compose 启动

```bash
cd /opt/pdf-module/docker
docker compose -f docker-compose.full.yml up -d
```

### 3. 验证服务

```bash
# 检查容器状态
docker ps

# 测试 Web API
curl http://localhost:8000/api/health

# 测试 MCP HTTP API
curl http://localhost:8001/api/health
```

## 服务说明

| 服务 | 端口 | 说明 |
|------|------|------|
| pdf-mcp | 8001 | **推荐**：MCP + 统一 HTTP + 内嵌 Wiki UI (`pdf-web-ui`) |
| pdf-web | 8000 | **已弃用**：仅管理 API sidecar，与 pdf-mcp 重复 |

## API 端点

### Web API (端口 8000)

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/health` | GET | 健康报告 |
| `/api/config` | GET/POST | 配置管理 |
| `/api/compile/status` | GET | 编译状态 |
| `/api/compile` | POST | 触发增量编译 |
| `/api/index/rebuild` | POST | 重建索引 |

### MCP HTTP API (端口 8001)

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/wiki/tree` | GET | Wiki目录树 |
| `/api/wiki/entries/*path` | GET | 单个条目 |
| `/api/wiki/search?q=...` | GET | 全文搜索 |
| `/api/wiki/graph/*path` | GET | 概念图 |
| `/api/wiki/stats` | GET | 知识库统计 |
| `/api/health` | GET | 健康报告 |

## 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `RUST_LOG` | `info` | 日志级别 |
| `PDFIUM_LIB_PATH` | `/usr/local/lib/libpdfium.so` | pdfium库路径 |
| `KNOWLEDGE_BASE` | `/app/kb` | 知识库路径 |
| `STORAGE_TYPE` | `local` | 存储类型 |
| `STORAGE_LOCAL_DIR` | `/app/data` | 本地存储目录 |

## 数据持久化

Docker Compose 配置了以下卷：

- `pdf_kb`: 知识库数据
- `pdf_data`: PDF文件存储
- `pdf_logs`: 日志文件

## 配置 AI 客户端

### Cursor

编辑 `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "pdf-mcp": {
      "command": "docker",
      "args": ["exec", "-i", "pdf-mcp", "pdf-mcp"]
    }
  }
}
```

### Claude Desktop

编辑配置文件：

```json
{
  "mcpServers": {
    "pdf-mcp": {
      "command": "docker",
      "args": ["exec", "-i", "pdf-mcp", "pdf-mcp"]
    }
  }
}
```

## 健康检查

```bash
# 检查容器健康状态
docker inspect --format='{{.State.Health.Status}}' pdf-web

# 查看日志
docker logs pdf-web
docker logs pdf-mcp
```

## 故障排查

### 端口冲突

如果端口被占用，修改 `docker-compose.full.yml` 中的端口映射：

```yaml
ports:
  - "8080:8000"  # 修改为可用端口
```

### pdfium 加载失败

确保容器内 pdfium 库存在：

```bash
docker exec pdf-mcp ls -la /usr/local/lib/libpdfium.so
```

### 知识库为空

首次使用需要编译PDF：

```bash
# 通过API触发编译
curl -X POST http://localhost:8000/api/compile
```

## 生产部署建议

1. **使用 HTTPS**: 配置反向代理（nginx/traefik）
2. **资源限制**: 添加 CPU/内存限制
3. **日志收集**: 配置日志驱动
4. **备份策略**: 定期备份知识库卷

```yaml
services:
  pdf-web:
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
```
