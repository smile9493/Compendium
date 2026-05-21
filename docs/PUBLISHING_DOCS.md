# 本仓库：MkDocs 发布说明

通用教程（MkDocs / Docsify / VuePress 等对比与完整步骤）见 **[GitHub Pages 静态文档部署教程](GITHUB_PAGES_STATIC_DOCS.md)**。

下文仅说明 **Compendium** 仓库已落地的配置。

本仓库使用 [MkDocs](https://www.mkdocs.org/) + [Material for MkDocs](https://squidfunk.github.io/mkdocs-material/) 将 `docs/` 目录构建为静态站点，并通过 GitHub Actions（官方 Pages 部署）发布。

## 在线地址

部署成功后访问：**https://smile9493.github.io/Compendium/**

### 首次启用（仓库维护者）

1. 打开 **Settings → Pages**
2. **Build and deployment → Source** 选择 **GitHub Actions**（不要选 `gh-pages` 分支；本仓库已改用 artifact 部署）
3. 合并文档相关改动到 `main` 后，在 **Actions** 查看 **Docs** workflow 是否通过

若此前用过 `mkdocs gh-deploy` 生成的 `gh-pages` 分支，可保留或删除；以 Actions 部署为准即可。

## CI 行为（`.github/workflows/deploy-docs.yml`）

| 事件 | 行为 |
|------|------|
| 向 `main` 推送且变更命中 `docs/**`、`mkdocs.yml` 等 | `build` → `deploy` |
| 针对 `main` 的 PR（同上路径） | 仅 `build`（`mkdocs build --strict`），不发布 |
| `workflow_dispatch` | 在 `main` 上手动触发时构建并部署 |

路径过滤避免仅改 Rust/Docker 时重复跑文档流水线。Python 依赖使用 `setup-python` 的 pip 缓存。

## 本地开发

```bash
pip install -r requirements-docs.txt
mkdocs serve
```

浏览器打开 `http://127.0.0.1:8000`，修改 Markdown 后会自动刷新。

构建静态文件（输出到 `site/`，已加入 `.gitignore`）：

```bash
mkdocs build --strict
```

`--strict` 与 CI 一致：导航引用了不存在的页面时会失败。

## 手动部署（可选，旧方式）

仍可使用 `mkdocs gh-deploy` 推送到 `gh-pages` 分支，但需将 Pages Source 改回分支部署，与当前 Actions 方案二选一：

```bash
mkdocs gh-deploy --force
```

推荐以 **Docs** workflow 为准，避免两套部署方式冲突。

## 配置说明

| 文件 | 作用 |
|------|------|
| `mkdocs.yml` | 站点名、`site_url`、导航、Material 主题 |
| `requirements-docs.txt` | Python 依赖（mkdocs、mkdocs-material） |
| `docs/` | 所有 Markdown 源文件 |
| `.github/workflows/deploy-docs.yml` | PR 构建校验 + `main` 自动发布 |

### 子路径与样式

项目站 URL 为 `https://<user>.github.io/<repo>/`，必须在 `mkdocs.yml` 中设置正确的 `site_url`（当前为 `https://smile9493.github.io/Compendium/`），否则资源路径可能 404。

### 自定义域名

在 **Settings → Pages** 填写自定义域名，并在构建产物根目录提供 `CNAME`（可通过 MkDocs `extra` 或 `docs/CNAME` 插件复制到 `site/`）。

## 常见问题

**部署后样式丢失**

检查 `site_url` 是否与最终访问地址一致（含仓库名与尾部斜杠）。

**Actions 构建失败**

查看 **Docs → Build MkDocs** 日志；常见原因为依赖未安装、`mkdocs.yml` 导航指向缺失文件，或 strict 模式下存在警告。

**PR 未跑文档 CI**

仅当 PR 修改了 `docs/**`、`mkdocs.yml`、`requirements-docs.txt` 或本 workflow 文件时才会触发；改其他目录不会跑文档构建。

**Pages 显示 404**

确认 Pages Source 为 **GitHub Actions**，且最近一次 **Deploy to GitHub Pages** job 成功。
