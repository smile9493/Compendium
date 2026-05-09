# PDF Module MCP CLI 工具

## 项目概述

PDF Module MCP CLI 是一个**命令行配置管理工具**，类似于 Web Dashboard 的命令行版本，用于管理 PDF Module MCP 的服务端配置和客户端连接。

**核心原则**：CLI 只负责配置管理，不涉及 PDF 处理的实际功能。

## 核心功能

### 1. 初始化配置 (`init`)

从零开始配置整个系统：

```bash
pdf-mcp init
```

**功能**:
- 创建安装目录结构
- 创建 Wiki 目录（raw/wiki/scheme）
- 创建配置文件模板
- 生成客户端配置示例
- 显示快速开始指南

### 2. 配置 GLM API (`config`)

配置智谱 AI 的 API Key 和参数：

```bash
# 交互式配置
pdf-mcp config

# 命令行配置
pdf-mcp config --key YOUR_API_KEY
pdf-mcp config --model glm-4v-flash
pdf-mcp config --endpoint https://open.bigmodel.cn/api/paas/v4/chat/completions
```

**功能**:
- 设置 VLM API Key
- 配置 VLM 模型
- 配置 API 端点
- 配置日志级别
- 配置 Dashboard 端口

### 3. 查看配置状态 (`status`)

查看当前配置和服务状态：

```bash
pdf-mcp status
```

**显示内容**:
- API Key 配置状态（密钥会被部分隐藏）
- VLM 模型和端点配置
- Dashboard 端口配置
- 服务运行状态
- 日志级别

### 4. 生成客户端配置 (`generate-config`)

为 MCP 客户端生成配置示例：

```bash
# 生成到标准输出
pdf-mcp generate-config

# 生成到文件
pdf-mcp generate-config --output ~/.config/pdf-mcp.json

# 指定服务端地址
pdf-mcp generate-config --server-url http://192.168.1.100:3000
```

**输出示例**:
```json
{
  "mcpServers": {
    "pdf-module": {
      "command": "npx",
      "args": ["-y", "@pdf-module/mcp"],
      "env": {
        "PDF_MCP_SERVER": "http://localhost:3000"
      }
    }
  }
}
```

### 5. 配置说明 (`info`)

查看详细的配置说明和使用指南：

```bash
# 显示完整说明
pdf-mcp info

# 显示特定部分
pdf-mcp info --section server
pdf-mcp info --section client
pdf-mcp info --section security
```

**内容包括**:
- 服务端-客户端架构说明
- 安全最佳实践
- API Key 配置位置推荐
- 常用命令参考

### 6. 管理系统服务 (`service`)

管理 PDF Module MCP 服务：

```bash
pdf-mcp service start      # 启动服务
pdf-mcp service stop       # 停止服务
pdf-mcp service restart    # 重启服务
pdf-mcp service status     # 查看状态
```

### 7. 查看日志 (`logs`)

查看服务日志：

```bash
# 查看最近 20 行
pdf-mcp logs

# 查看最近 50 行
pdf-mcp logs -n 50

# 实时跟踪日志
pdf-mcp logs -f
pdf-mcp logs -f -n 100
```

### 8. 启动 Dashboard (`dashboard`)

启动 Web 管理界面：

```bash
# 使用默认端口 8000
pdf-mcp dashboard

# 指定端口
pdf-mcp dashboard --port 8080
```

### 9. 交互式菜单 (`interactive`)

进入交互式菜单模式，类似 sing-box 的体验：

```bash
pdf-mcp interactive
```

**菜单选项**:
```
╔════════════════════════════════════════════════════════════════╗
║                  PDF Module MCP 配置管理器                     ║
╠════════════════════════════════════════════════════════════════╣
║  1. 初始化配置                                                   ║
║  2. 配置 GLM API                                                ║
║  3. 查看配置状态                                                 ║
║  4. 生成客户端配置                                               ║
║  5. 配置说明                                                     ║
║  6. 管理系统服务                                                 ║
║  7. 查看日志                                                     ║
║  8. 启动 Dashboard                                              ║
║  0. 退出                                                        ║
╚════════════════════════════════════════════════════════════════╝
```

直接运行 `pdf-mcp` 不带参数也会进入交互模式。

## 架构说明

PDF Module MCP 采用 **服务端-客户端** 架构：

### 服务端 (Server)
- **位置**: `/opt/pdf-module`
- **职责**:
  - 管理 API Key 和敏感配置（.env.local）
  - 处理 PDF 文件和 VLM 调用
  - 提供 REST API 接口
- **配置文件**: `/opt/pdf-module/.env.local`

### 客户端 (Client)
- **类型**: 任何 MCP 兼容客户端（Trae IDE、Cursor、Claude Desktop 等）
- **职责**: 通过 MCP 协议与服务端通信
- **配置**: 只需要服务端地址，不需要 API Key

## 安全最佳实践

### 推荐：API Key 配置在服务端 ✅

```bash
# 服务端配置文件：/opt/pdf-module/.env.local
VLM_API_KEY=your_api_key_here
```

**优点**:
- 敏感信息集中管理
- 不会暴露给客户端
- 更新只需改一处
- 可以加入 .gitignore

### 客户端只需要服务地址 ✅

```json
{
  "mcpServers": {
    "pdf-module": {
      "command": "npx",
      "args": ["-y", "@pdf-module/mcp"],
      "env": {
        "PDF_MCP_SERVER": "http://服务器IP:3000"
      }
    }
  }
}
```

### 避免：客户端配置中包含 API Key ❌

不要在客户端 MCP JSON 中包含 API Key，避免：
- 泄露给其他人
- 提交到 Git 仓库
- 需要在多个客户端重复更新

## 安装

### 从源码编译

```bash
# 克隆项目
git clone https://github.com/smile9493/rsut_pdf_mcp.git
cd rsut_pdf_mcp/pdf-mcp-installer

# 编译
cargo build --release

# 安装
sudo cp target/release/pdf-mcp /usr/local/bin/
sudo chmod +x /usr/local/bin/pdf-mcp
```

### 下载预编译二进制

从 GitHub Releases 页面下载对应平台的二进制文件：

```bash
# Linux x86_64
wget https://github.com/smile9493/rsut_pdf_mcp/releases/latest/download/pdf-mcp-linux-x86_64
chmod +x pdf-mcp-linux-x86_64
sudo mv pdf-mcp-linux-x86_64 /usr/local/bin/pdf-mcp
```

## 快速开始

1. **初始化配置**:
   ```bash
   sudo pdf-mcp init
   ```

2. **配置 API Key**:
   ```bash
   pdf-mcp config --key YOUR_API_KEY
   ```

3. **查看状态**:
   ```bash
   pdf-mcp status
   ```

4. **启动服务**:
   ```bash
   sudo pdf-mcp service start
   ```

5. **生成客户端配置**:
   ```bash
   pdf-mcp generate-config > ~/.config/trae/mcp.json
   ```

6. **查看配置说明**:
   ```bash
   pdf-mcp info
   ```

## 常用命令速查

| 功能 | 命令 |
|------|------|
| 初始化 | `sudo pdf-mcp init` |
| 配置 | `pdf-mcp config --key KEY` |
| 状态 | `pdf-mcp status` |
| 客户端配置 | `pdf-mcp generate-config` |
| 配置说明 | `pdf-mcp info` |
| 启动服务 | `sudo pdf-mcp service start` |
| 停止服务 | `sudo pdf-mcp service stop` |
| 查看日志 | `pdf-mcp logs -n 50` |
| Dashboard | `pdf-mcp dashboard` |
| 交互模式 | `pdf-mcp` 或 `pdf-mcp interactive` |

## 文件结构

```
/opt/pdf-module/
├── .env.local              # 环境配置（API Key等）
├── pdf-mcp                 # MCP 服务二进制
├── pdf-dashboard           # Dashboard 二进制（可选）
├── web/                    # Web 界面
├── wiki/
│   ├── raw/               # 原始文件
│   ├── wiki/              # 处理后文件
│   └── scheme/            # 数据结构
└── mcp-config-example.json  # 客户端配置示例
```

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！
