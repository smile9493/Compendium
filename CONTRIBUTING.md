# Contributing to Compendium

感谢您对 Compendium（AI 原生 PDF 知识编译引擎）的关注！本文档指导您如何参与项目开发。

---

## 目录

- [开发环境](#开发环境)
- [代码规范](#代码规范)
- [分支策略](#分支策略)
- [提交信息规范](#提交信息规范)
- [PR 流程](#pr-流程)
- [测试要求](#测试要求)
- [文档要求](#文档要求)

---

## 开发环境

### 系统要求

- **Rust**: 1.91.0+（Rust 2024 Edition）
- **系统**: Linux x86_64（推荐），macOS 也可用
- **可选**: Docker（容器化部署）

### 本地构建

```bash
# 克隆仓库
git clone https://github.com/smile9493/Compendium.git
cd Compendium

# 构建所有 crate
cargo build

# 运行测试
cargo test

# 运行 clippy
cargo clippy --workspace --all-targets -- -D warnings

# 格式化代码
cargo fmt --all
```

### 其他命令

项目使用 `just` 作为任务运行器：

```bash
# 查看可用命令
just --list

# 完整构建 + 测试 + clippy
just ci
```

---

## 代码规范

项目严格遵循以下编码标准：

### 执行模式

**standard 模式** — P0 Safety + P1 Maintainability 严格执行，P2 Performance 建议执行。

### Rust 编码规则

| 领域 | 规则 |
|------|------|
| **错误处理** | 库 crate 使用 `thiserror`；二进制入口使用 `anyhow` + `.context()` |
| **所有权** | 业务层使用 Owned + `.clone()`；热路径使用 `Cow`/`Bytes` 零拷贝 |
| **并发** | 仅使用有界 channel；`Mutex` 不得跨 `.await` 持有；优先 `parking_lot` |
| **API 演化** | 公共 struct/enum 使用 `#[non_exhaustive]` |
| **FFI 安全** | `extern` 函数必须包裹 `catch_unwind` |
| **内存布局** | FFI 类型使用 `#[repr(C)]` |

### 自动触发的技能

修改 Rust 代码时，Cursor Agent 会自动调用以下技能指南：

1. `rust-architecture-guide` — 所有 Rust 任务的宪法基础
2. `rust-wasm-frontend-infra-guide` — WASM 边界代码（`pdf-wasm` crate）
3. `rust-systems-cloud-infra-guide` — 服务端基础设施（`vlm-visual-gateway`、`pdf-mcp`）

### Cargo.toml 模板

新建 crate 时，请遵循：

```toml
[package]
name = "crate-name"
version.workspace = true
edition = "2024"
rust-version = "1.91.0"
license.workspace = true
```

### Rust Edition

项目使用 **Rust 2024 Edition**。所有新 crate 必须设置 `edition = "2024"`。

---

## 分支策略

| 分支 | 用途 | 说明 |
|------|------|------|
| `main` | 生产分支 | 保持稳定，仅通过 PR 合并 |
| `develop` | 开发分支 | 日常开发集成 |
| `feat/*` | 功能分支 | 从 develop 创建，完成后合并回 develop |
| `fix/*` | 修复分支 | Bug 修复 |
| `chore/*` | 杂项分支 | 构建配置、依赖更新、文档等 |
| `release/*` | 发布分支 | 从 develop 创建，合并到 main 和 develop |

---

## 提交信息规范

项目采用 **Conventional Commits** 规范：

```
<type>(<scope>): <description>

[optional body]
[optional footer]
```

### 类型

| 类型 | 含义 |
|------|------|
| `feat` | 新功能 |
| `fix` | Bug 修复 |
| `docs` | 文档变更 |
| `style` | 代码格式（不影响功能） |
| `refactor` | 重构（既不修复 bug 也不添加功能） |
| `perf` | 性能优化 |
| `test` | 测试相关 |
| `ci` | CI 配置变更 |
| `chore` | 构建、依赖、杂项 |
| `revert` | 回退提交 |

### 作用域

| 作用域 | 对应 crate |
|--------|-----------|
| `pdf-core` | `crates/pdf-core` |
| `pdf-mcp` | `crates/pdf-mcp` |
| `pdf-cli` | `crates/pdf-cli` |
| `pdf-wasm` | `crates/pdf-wasm` |
| `pdf-common` | `crates/pdf-common` |
| `pdf-macros` | `crates/pdf-macros` |
| `pdf-mcp-contracts` | `crates/pdf-mcp-contracts` |
| `vlm` | `crates/vlm-visual-gateway` |
| `web-ui` | Vue 3 前端 |
| `release` | 发布相关 |

### 示例

```
feat(pdf-core): add hybrid search RRF fusion
fix(pdf-mcp): isolate test workspace registry paths
docs: update API reference with management tools
chore: bump workspace edition to 2024
```

---

## PR 流程

1. 从 `develop` 创建功能分支：`git checkout -b feat/my-feature develop`
2. 在本地开发并提交（遵循 Conventional Commits）
3. 推送分支：`git push -u origin feat/my-feature`
4. 创建 Pull Request 到 `develop` 分支
5. PR 标题和描述遵循以下模板：

```markdown
## Summary

[1-3 句描述变更内容]

## Changes

- [具体变更 1]
- [具体变更 2]

## Test Plan

- [ ] cargo build --workspace 通过
- [ ] cargo test --workspace 通过
- [ ] cargo clippy --workspace --all-targets 通过
- [ ] 相关文档已更新
```

### PR 通过标准

- [x] `cargo build --workspace` 无错误
- [x] `cargo test --workspace` 全部通过
- [x] `cargo clippy --workspace --all-targets -- -D warnings` 无警告
- [x] `cargo fmt --all` 已执行
- [x] 相关文档已更新
- [x] 必要时添加了测试

---

## 测试要求

### 单元测试

- 新功能需添加对应的单元测试
- 测试文件放在 `src/` 中对应模块的 `#[cfg(test)] mod tests` 内
- 或放在 `tests/` 集成测试目录

### 测试命令

```bash
# 运行所有测试
cargo test --workspace

# 运行特定 crate 的测试
cargo test -p pdf-core

# 运行特定测试
cargo test test_name

# 运行集成测试
cargo test --test mcp_contracts
```

---

## 文档要求

### API 文档

MCP 工具的修改必须同步更新以下文件：

- `docs/API_REFERENCE.md` — 工具参数、返回值、使用示例

### 架构文档

架构变更需要更新：

- `ARCHITECTURE.md` — 架构概述
- `doc/adr/` — 新增或更新架构决策记录（ADR）

### 变更日志

`CHANGELOG.md` 由 `git cliff` 自动生成：

```bash
git cliff -o CHANGELOG.md
```

请在合并 PR 前确保 CHANGELOG 已更新。

### 行内文档

- 公共函数和类型必须有文档注释（`///` 或 `//!`）
- 内部函数可根据复杂度选择性添加
- 复杂算法需要添加算法说明注释
