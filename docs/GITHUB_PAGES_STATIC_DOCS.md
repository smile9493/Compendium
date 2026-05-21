# 使用静态文档生成器将文档部署到 GitHub Pages

本篇详细说明如何使用静态文档生成器（如 MkDocs、Docsify、VuePress 等）将项目文档构建为 HTML 网站，并托管到 GitHub Pages。整个过程涵盖环境准备、本地生成、手动部署及自动化部署方案。

---

## 1. 工具选择

静态文档生成器可将 Markdown 文件渲染为带导航、搜索、主题的完整静态站点。常用工具对比：

| 工具 | 语言/生态 | 特点 |
|------|-----------|------|
| **MkDocs** | Python | 配置简单，主题丰富（Material for MkDocs），适合技术文档 |
| **Docsify** | JavaScript | 运行时渲染，无需构建，单页面应用，快速启动 |
| **VuePress** | JavaScript (Vue) | Vue 生态，支持自定义组件，适合大型文档站 |
| **Docusaurus** | JavaScript (React) | Meta 开源，功能强大，支持版本化 |
| **Sphinx** | Python | 传统文档工具，支持 reStructuredText，适合 Python 项目 |

下文以 **MkDocs** 作为示例进行详细演示，其他工具的部署流程类似。

---

## 2. 准备工作

- 拥有一个 GitHub 仓库（用于存放文档源文件及托管站点）
- 本地安装 Git，并配置好与 GitHub 仓库的连接
- 本地安装相应生成器所需的运行环境（Python 或 Node.js）

---

## 3. MkDocs 详细操作步骤

### 3.1 安装 MkDocs

确保 Python（3.6+）和 pip 已安装，然后执行：

```bash
pip install mkdocs
```

若要使用更丰富的 Material 主题，可额外安装：

```bash
pip install mkdocs-material
```

### 3.2 初始化文档项目

在本地仓库根目录（或子目录）执行：

```bash
mkdocs new docs-project
cd docs-project
```

此时目录结构为：

```
docs-project/
├── mkdocs.yml          # 配置文件
└── docs/
    └── index.md        # 文档首页
```

### 3.3 编写与组织文档

所有 Markdown 文件放入 `docs/` 文件夹。例如：

```
docs/
├── index.md
├── guide/
│   ├── installation.md
│   └── usage.md
└── api/
    └── reference.md
```

编辑 `docs/index.md` 作为站点首页。

### 3.4 配置站点

编辑 `mkdocs.yml`，设置站点名称、导航栏、主题等：

```yaml
site_name: 我的项目文档
site_url: https://用户名.github.io/仓库名/
theme:
  name: material          # 使用 material 主题，若安装 mkdocs-material
  language: zh
nav:
  - 首页: index.md
  - 使用指南:
    - 安装: guide/installation.md
    - 基本用法: guide/usage.md
  - API 参考: api/reference.md
```

若使用默认主题，将 `name` 设为 `mkdocs` 或 `readthedocs`。

### 3.5 本地预览

在 `mkdocs.yml` 所在目录执行：

```bash
mkdocs serve
```

浏览器访问 `http://127.0.0.1:8000` 即可实时预览文档，修改内容会自动刷新。

### 3.6 构建静态文件

预览无误后，构建站点：

```bash
mkdocs build
```

生成的静态文件位于 `site/` 目录，可直接用浏览器打开 `site/index.html` 查看最终效果。

---

## 4. 部署到 GitHub Pages

### 4.1 手动部署（使用命令行）

MkDocs 提供一键部署命令，会将 `site/` 内容推送到远程仓库的 `gh-pages` 分支。

首先确保本地仓库与 GitHub 远程关联，且当前所在分支已提交所有更改。然后执行：

```bash
mkdocs gh-deploy
```

该命令会：

1. 在本地构建站点到 `site/`
2. 将 `site/` 内容复制到临时目录并初始化为一个 Git 仓库
3. 强制推送至远程 `gh-pages` 分支

部署成功后，前往 GitHub 仓库的 **Settings → Pages**，确保 “Source” 选择的是 `Deploy from a branch`，分支选 `gh-pages`，目录选 `/ (root)`，保存。站点将在 **https://用户名.github.io/仓库名/** 上线。

### 4.2 使用 GitHub Actions 自动部署

在仓库根目录创建 `.github/workflows/deploy-docs.yml` 文件，写入以下内容：

```yaml
name: Deploy MkDocs to Pages

on:
  push:
    branches: [ main ]   # 根据默认分支名称调整
  workflow_dispatch:     # 允许手动触发

permissions:
  contents: write

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.x'
      - name: Install dependencies
        run: pip install mkdocs mkdocs-material
      - name: Deploy
        run: mkdocs gh-deploy --force
```

提交并推送此文件后，每次向 `main` 分支推送更新，GitHub Actions 将自动构建并部署文档，无需手动执行 `mkdocs gh-deploy`。

**注意**：若 `mkdocs.yml` 不在仓库根目录，需在 Action 中指定工作目录，或通过 `-f` 参数指定配置文件路径，例如：

```yaml
run: cd docs-project && mkdocs gh-deploy --force
```

### 4.3 部署到 `/docs` 文件夹而非独立分支（备选）

如果不希望使用 `gh-pages` 分支，可将 MkDocs 的 `site_dir` 修改为 `docs`，直接放在源码分支。编辑 `mkdocs.yml`：

```yaml
site_dir: docs
```

执行 `mkdocs build` 后，静态文件会输出到仓库根目录的 `docs/` 文件夹。然后在 GitHub 仓库的 Pages 设置中选择对应分支（如 `main`）和 `/docs` 目录即可。这种方式会将构建产物混入源码仓库，通常建议使用独立分支。

---

## 5. 其他生成器的部署要点

### Docsify

- 特点：无需构建，只需在 `docs/` 目录下放好 Markdown 和一个 `index.html` 入口文件。
- 初始化：全局安装 `docsify-cli`，执行 `docsify init ./docs` 生成基础文件。
- 部署：直接将整个仓库（或 `docs/` 目录）推送到 GitHub，在 Pages 设置中选择包含 `index.html` 的分支和目录。
- 也可使用 `gh-pages` 分支，手动复制文件或编写 Actions 脚本。

### VuePress / Docusaurus

- 生成静态文件的命令一般为 `npm run build`（或 `yarn build`），输出目录默认 `dist/`（VuePress）或 `build/`（Docusaurus）。
- 自动部署推荐使用 GitHub Actions，可选用社区现成的 Actions（如 `peaceiris/actions-gh-pages`）将指定文件夹推送到 `gh-pages` 分支。示例：

```yaml
- name: Deploy
  uses: peaceiris/actions-gh-pages@v3
  with:
    github_token: ${{ secrets.GITHUB_TOKEN }}
    publish_dir: ./dist
```

- 同样需要在仓库 Settings 中启用 GitHub Pages 并指向 `gh-pages` 分支。

---

## 6. 常见问题

### 部署后样式丢失或 404 页面
- 检查 `mkdocs.yml` 中的 `site_url` 是否正确设置，应与最终访问地址一致。
- 如果站点不在域名根路径（即 `https://用户名.github.io/仓库名/`），确保静态文件中的资源路径正确。对于 MkDocs，设置了正确的 `site_url` 会自动处理；VuePress 等需配置 `base` 字段（如 `base: '/仓库名/'`）。

### 自定义域名
在 GitHub Pages 设置中添加自定义域名，并在 DNS 服务商处添加相应的 CNAME 记录。同时可以在 `docs/` 目录（或站点根目录）放入一个名为 `CNAME` 的文件，内容为自定义域名，防止被部署覆盖。

### 构建失败
- 检查依赖是否完整安装（`mkdocs-material`、插件等）。
- 查看 GitHub Actions 日志定位错误，常见问题为 Python 版本、pip 安装权限、网络问题。

---

## 7. 总结

使用静态文档生成器构建文档并托管到 GitHub Pages 的标准流程为：

1. 选择合适的生成器并初始化项目
2. 编写 Markdown 文档，配置导航和主题
3. 本地预览并构建静态站点
4. 通过 `gh-deploy` 命令或 GitHub Actions 将静态文件推送到 `gh-pages` 分支
5. 在仓库设置中启用 Pages 指向该分支

整个过程可实现文档即代码，更新推送后自动发布，轻松维护项目公开文档。
