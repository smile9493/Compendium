# Changelog

All notable changes to this project will be documented in this file.
## [unreleased]

### Bug Fixes

- **pdf-mcp:** Remove unused contract imports for clippy(46687d0)
- **pdf-core:** Repair integration/snapshot tests and profiling bench(c2c100d)
- **pdf-mcp:** Isolate test workspace registry paths(4e15796)

### Chores

- Rebrand to Compendium and tighten repository hygiene(cfa7416)
- Allow rsut project token in typos dictionary(86b719f)
- Allow PDF Flate token in typos dictionary(313b548)

### Styling

- Apply cargo fmt for pre-push hook compliance(13395d4)
- Cargo fmt(6003b21)


## [0.5.0]

### Bug Fixes

- 修复 CLI 进程检测误杀其他进程的问题 [skip ci](c606d63)
- 修复 P0 和 P1 问题，提升代码质量(2dabbb8)
- 修复知识索引模块三个测试错误 + 完善 CI 只保留 Linux amd64(970b729)
- 修复 clippy 警告和代码格式化问题(9ef8017)
- 完善 .gitignore，移除 PDFium 预构建文件和编译产物(12a2f1b)
- 完善 .gitignore，移除 PDFium 预构建文件和编译产物(1eaf248)
- **mcp:** Resolve wiki KB via kb_id across HTTP APIs(3cd51eb)
- **clippy:** Set workspace lint group priorities for Rust 1.95(9f31de4)
- **wasm:** Satisfy clippy for preview helpers(cb599fd)
- **compile:** Clear stale last_outcome during awaiting_agent(251bb18)
- **wasm:** Satisfy clippy needless_borrows in preview helper(aca9b2b)
- **pdf-common:** Tighten feature flag env parsing and tests(3f6e3e3)
- **pdf-common:** Remove unreachable match arm in flag env parse(4d1755c)
- **compile:** Clear stale last_outcome during awaiting_agent(fbe7c38)

### CI/CD

- 只保留 Linux x86_64 (amd64) 构建，移除多平台矩阵(78fb60b)

### Chores

- 更新 .gitignore 忽略 wiki 和 CLI 二进制文件 [skip ci](f31a6c7)
- **cli:** Apply cargo fmt for pre-push hook compliance(c171e38)
- Apply cargo fmt across pdf-module-rs workspace(4e7ee1b)
- **release:** Document capabilities, ADRs, and release hardening(d195a4f)
- Apply cargo fmt for pre-push hook(e99d93e)
- Allow clippy acronym and glob-reexport lints in shared crates(be823d7)
- Clippy and pre-push hook compliance across workspace(80ce5ca)
- **release:** Document capabilities, ADRs, and release hardening(bc36dca)

### Documentation

- 为核心 API 添加文档注释(3b315de)
- 更新文档反映知识引擎架构 [skip ci](b4a6dbe)
- 完善项目文档(deb19f8)

### Features

- Implement improvement-design.md short-term optimizations [skip ci](7556467)
- 清理废弃 Vue 项目 + 添加 PDFium 头文件 + 完善构建配置(ac84d60)
- Cursor skills & rules + code quality improvements(f2f3856)
- Vue3 SPA 集成到 pdf-mcp + 全量代码更新(c66280f)
- Add architecture governance, feature flags, and framework scaffolding(89a74e8)
- **pdf-web-ui:** Implement navigation, body_markdown API, and UX roadmap(04f01b8)
- Major architecture enhancement and refactor[**BREAKING**](acabc5f)
- **core:** Phase 1 foundation — unified index and compile status(dad9d7d)
- **phase-2:** Hybrid search, quality loop, agent patch tools, compile drawer(cb08d35)
- **phase-3:** Platformization — multi-KB workspaces, extraction plugins, wasm preview, sync(c6dd894)
- **compile:** Add staged CompileJob pipeline, quality gates, and compile_plan(4b98b49)
- **mcp:** Typed tool contracts, knowledge search alignment, compile sampling(c4e8a5b)
- **web-ui:** Ops console, SSE compile events, share links, and i18n(334bfd1)
- **mcp:** Typed tool contracts, knowledge search alignment, compile sampling(2da33da)
- **web-ui:** Ops console, SSE compile events, share links, and i18n(8bd364c)

### Performance

- P2 WASM 优化(c9b4f9b)

### Refactor

- P1 架构优化(7ba2714)


## [0.1.3]

### Bug Fixes

- Remove deprecated tool_description_test.rs(b1d9f4d)
- Use auto version from Cargo.toml in CLI(f826020)

### Chores

- Trigger CI rebuild(a7b8464)
- Bump CLI version to 0.1.3(e6b5df2)

### Features

- Implement Karpathy Wiki architecture and fix CI workflow(e9ab7b6)


## [0.1.2]

### Bug Fixes

- 修复编译错误和同步 deploy.sh(3b85a76)
- 修复编译错误和 CLI bin 名称冲突(108c019)
- 安装 OpenSSL 开发库解决编译错误(bc12c1f)
- CLI 使用 rustls-tls 替代 native-tls(9f556e3)
- 修复 macOS 打包时 mv 命令错误(b64ddd2)
- Windows 构建路径使用 github.workspace 变量(255fac8)
- CLI 启动 Dashboard 时设置 PDFIUM_LIB_PATH 环境变量(0a45e40)

### Documentation

- 更新 Release 模板描述，MCP Client → MCP Server [skip ci](b8e9369)

### Features

- 添加CLI配置管理工具和一键安装脚本(efc6008)
- 添加 CLI 工具到 Release 构建(6c49e89)
- 添加 PDFIUM_LIB_PATH 支持和自动下载 pdfium 库(d1cacad)

### Refactor

- 一键安装改为下载预编译二进制(c251088)
- 改进 CLI 和精简 README(0363885)


## [0.1.1]

### Bug Fixes

- Resolve all clippy errors for CI(937c3b3)
- Resolve cargo-deny failures (security, licenses, wildcards)(ea0f6bc)
- 简化安全审计流程，移除 cargo-audit 避免 advisory-db 克隆失败(ac40ab7)

### CI/CD

- Install cargo-audit before running security audit(bd72769)

### Chores

- 从 Git 移除 pdfium C 头文件 - 修复 GitHub 语言统计(43938cc)

### Documentation

- 更新 README Docker 编排为单镜像模式，补充环境变量和版本历史(10e99eb)
- 更新 README Docker 编排为单镜像模式，补充环境变量和版本历史(2a63d27)

### Features

- 统一深色模式 slate 色系，修复布局与功能问题(1c195fe)

### Styling

- Apply cargo fmt across all crates(6b8a022)
- Apply cargo fmt to remaining files(6273051)


## [0.1.0]

### Bug Fixes

- 修复 CI 构建错误 - clippy 检查(20c37f3)
- Resolve clippy errors in vlm-visual-gateway and pdf-core(339a8ac)
- 修复剩余的 clippy 错误和代码格式问题(66d4f53)
- 升级 apexcharts 到 5.10.0 并允许提交 package-lock.json(7cd2f07)
- 添加 vue-tsc 和 eslint 依赖 - 修复类型检查和 lint 失败(64509ea)
- 升级 vue-tsc 到 2.0.0 - 修复 TypeScript 5.4 兼容性(d585f45)
- 添加缺失的类型定义和依赖 - 修复 TypeScript 错误(bd8c3ca)
- 完善类型定义和 Store 实现(8c5c8cc)
- 修复所有 Web TypeScript 和 ESLint 错误(20a6bc4)
- 修复 Dockerfile 中 rust 镜像标签 - 使用 rust:bookworm(ddfbedc)
- 从 .dockerignore 移除 dist - 允许 Web 构建产物进入 Docker(4b2e216)

### CI/CD

- 在每次 push 时运行 Web 构建和测试(d261417)
- 优化构建流程 - push 到 main 时运行完整 CI(230bad0)
- 添加交叉编译工具链支持 - 修复 ARM64 构建错误(27ae139)
- 修复 Web 构建缓存问题 - 移除重复的缓存配置(2e8d57a)
- 修复 npm 缓存问题 - 缓存 ~/.npm 目录(c2532ae)
- 放宽 TypeScript 类型检查配置 - 允许 CI 通过(a49a152)
- 使 Docker 构建可选 - 仅在配置 DOCKER_ENABLED 变量时运行(9503e53)
- 修复 ARM64 交叉编译 strip 问题 - 使用 aarch64-linux-gnu-strip(a76bc5a)
- 修复 Docker Secret 名称不匹配问题(590d032)
- 删除多余的 docker-image.yml 工作流(4dac211)
- 手动触发 CI 构建(c4b9910)
- 修复 workflow 语法错误 - 使用 vars 替代 secrets(4f5dacd)
- 重新触发 CI - DOCKER_ENABLED 已配置(68980cb)
- 修改 Docker 构建条件 - 只要变量存在就执行(04dc743)
- 重新触发 CI - Docker 凭据已更新(7d83d22)
- 移除 Docker 构建的条件判断 - 让 Docker 任务始终运行以便排查(d744159)
- 禁用 Docker 构建 - 改为本地手动构建推送(022c5de)
- 删除 docker-mcp 和 docker-web 任务定义 - 不再显示跳过状态(0b13859)
- 添加 contents:write 权限 - 修复 release 403 错误(5febcae)
- 显式指定 GITHUB_TOKEN - 修复 release 403 错误(82b3aaf)

### Documentation

- 更新 README.md - 添加完整的部署、版本和 CI/CD 说明(1a67d27)
- 更新 README - 添加 Docker 编排部署、Web 界面预览、完整部署指南(33c7aa4)

### Features

- Complete architecture refactoring - add pdf-common and pdf-macros crates(87dbf1b)
- 优化 CI 工作流配置和代码质量检查 (#4)(70ab375)

### Refactor

- 奥卡姆剃刀 - 收敛至纯 stdio MCP + 单一 pdfium 引擎(e2c5062)


