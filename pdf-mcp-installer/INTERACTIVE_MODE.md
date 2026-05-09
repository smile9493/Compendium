# PDF Module MCP CLI 交互模式使用指南

## 🎯 快速开始

### 方式 1：直接运行（进入交互菜单）

```bash
# 直接运行 pdf-mcp，自动进入交互模式
./target/release/pdf-mcp
```

### 方式 2：使用 interactive 命令

```bash
# 显式调用 interactive 命令
./target/release/pdf-mcp interactive
```

---

## 📋 交互式菜单

运行后显示美观的菜单界面：

```
╔════════════════════════════════════════╗
║  📋  主菜单 ║
║═══════════════════════════════════════════╗
║  🔧  1. 初始化配置            ║
║  ⚙️  2. 配置 GLM API          ║
║  📊  3. 查看配置状态          ║
║  📤  4. 上传 PDF 文件           ║
║  📁  5. 查看 PDF 列表           ║
║  🧪  6. 测试 PDF 处理           ║
║  🔧  7. 管理系统服务          ║
║  📜  8. 查看日志              ║
║  🖥️  9. 启动 Dashboard        ║
║  🚪  0. 退出                  ║
║═══════════════════════════════════════════╝

→ 请选择操作 (0-9): 
```

---

## 🎮 菜单操作

### 数字选择

输入对应数字并回车：

- **1** - 初始化配置
- **2** - 配置 GLM API
- **3** - 查看配置状态
- **4** - 上传 PDF 文件
- **5** - 查看 PDF 列表
- **6** - 测试 PDF 处理
- **7** - 管理系统服务
- **8** - 查看日志
- **9** - 启动 Dashboard
- **0** - 退出

### 示例流程

#### 配置 GLM API Key

```
→ 请选择操作 (0-9): 2

>>> 配置 GLM API

获取 GLM API Key:
  1. 访问：https://open.bigmodel.cn/
  2. 注册/登录账号
  3. 进入控制台 -> API Keys
  4. 创建新的 API Key

请输入你的 GLM API Key: sk-xxxxxxxxxxxxxxxx

✓ VLM_API_KEY 已设置

✓ 配置已保存

配置信息:
  ✓ VLM_API_KEY: sk-xxxx****
  → VLM_MODEL: glm-4v-flash
  → VLM_ENDPOINT: https://open.bigmodel.cn/api/paas/v4/chat/completions
  → DASHBOARD_PORT: 8000
  → RUST_LOG: info

→ 按回车键继续...
```

#### 上传 PDF 文件

```
→ 请选择操作 (0-9): 4

>>> 上传 PDF 文件

请输入 PDF 文件路径：/path/to/document.pdf

→ 源文件：/path/to/document.pdf
→ 目标路径：/pdfs/document.pdf
✓ 上传成功
  文件路径：/pdfs/document.pdf

→ 按回车键继续...
```

#### 测试 PDF 处理

```
→ 请选择操作 (0-9): 6

>>> 测试 MCP 服务

请输入 PDF 文件路径 (直接回车使用默认): /pdfs/document.pdf

✓ 测试文件：/pdfs/document.pdf

→ 测试：获取页数
✓ 响应：{"jsonrpc":"2.0","id":1,"result":{"page_count":10}}

→ 按回车键继续...
```

#### 管理服务（子菜单）

```
→ 请选择操作 (0-9): 7

>>> 管理系统服务

服务管理:
  1. 启动服务 (start)
  2. 停止服务 (stop)
  3. 重启服务 (restart)
  4. 查看状态 (status)
  0. 返回主菜单

选择操作：4

>>> 管理系统服务
✓ 服务 status 成功

→ 按回车键继续...
```

---

## 🎨 界面特色

### 彩色输出

- **蓝色边框** - 菜单框架
- **黄色图标** - 功能标识
- **绿色成功** - ✓ 操作成功
- **红色错误** - ✗ 操作失败
- **蓝色提示** - → 提示信息

### Emoji 图标

- 📋 主菜单
- 🔧 配置/服务
- ⚙️ API 配置
- 📊 状态查看
- 📤 文件上传
- 📁 文件列表
- 🧪 测试功能
- 📜 日志查看
- 🖥️ Dashboard
- 🚪 退出

---

## 💡 使用技巧

### 1. 快速配置

```bash
# 第一次使用，进入交互模式
./target/release/pdf-mcp

# 选择 1 初始化
# 选择 2 配置 API Key
# 选择 3 查看状态确认
```

### 2. 日常使用

```bash
# 查看状态
./target/release/pdf-mcp status

# 上传文件
./target/release/pdf-mcp upload file.pdf

# 或直接进入交互模式
./target/release/pdf-mcp
```

### 3. 命令与交互结合

```bash
# 命令行快速配置
./target/release/pdf-mcp config --key YOUR_API_KEY

# 进入交互模式管理文件
./target/release/pdf-mcp
# 选择 4 上传
# 选择 5 查看
# 选择 6 测试
```

---

## 📊 对比

| 使用方式 | 优点 | 适用场景 |
|---------|------|---------|
| **交互模式** | 直观、友好、无需记命令 | 新手、日常管理 |
| **命令行** | 快速、可脚本化 | 自动化、批处理 |
| **混合使用** | 灵活、高效 | 高级用户 |

---

## 🚀 完整工作流示例

### 从零开始

```bash
# 1. 进入交互模式
./target/release/pdf-mcp

# 2. 选择 1 - 初始化配置
# → 自动创建所有目录和配置

# 3. 选择 2 - 配置 GLM API
# → 输入 API Key

# 4. 选择 3 - 查看状态
# → 确认配置正确

# 5. 选择 4 - 上传 PDF
# → 输入文件路径

# 6. 选择 6 - 测试处理
# → 验证功能正常

# 7. 选择 0 - 退出
```

### 日常使用

```bash
# 快速上传文件
./target/release/pdf-mcp upload document.pdf

# 查看列表
./target/release/pdf-mcp list

# 测试处理
./target/release/pdf-mcp test --file /pdfs/document.pdf

# 或进入交互模式一站式完成
./target/release/pdf-mcp
```

---

## 🛠️ 高级功能

### 服务管理子菜单

选择 `7` 进入服务管理子菜单：

```
服务管理:
  1. 启动服务 (start)
  2. 停止服务 (stop)
  3. 重启服务 (restart)
  4. 查看状态 (status)
  0. 返回主菜单
```

### 日志查看

选择 `8` 可以自定义查看行数：

```
查看最近多少行日志？(默认 20): 50
```

### Dashboard 启动

选择 `9` 可以自定义端口：

```
Dashboard 端口？(默认 8000): 9000
```

---

## ⌨️ 键盘操作

- **数字键 0-9** - 选择对应功能
- **回车键** - 确认输入/继续
- **Ctrl+C** - 强制退出

---

## 📝 提示

1. **首次使用建议走一遍交互流程**，熟悉所有功能
2. **常用命令可以记住命令行方式**，提高效率
3. **不确定时使用交互模式**，有详细提示
4. **每次操作后可以按回车返回菜单**，继续其他操作

---

## 🎉 总结

交互模式提供了：

- ✅ **零学习成本** - 数字选择，一看就懂
- ✅ **完整功能** - 所有操作都能完成
- ✅ **友好提示** - 每步都有详细说明
- ✅ **错误防护** - 输入验证，避免误操作
- ✅ **视觉美观** - 彩色界面，Emoji 图标

**让 PDF Module MCP 管理变得如此简单！** 🚀
