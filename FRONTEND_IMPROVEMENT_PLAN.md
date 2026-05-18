# 前端架构改进方案与实施计划

> **范围**：仅前端（`pdf-module-rs/crates/pdf-mcp/pdf-web-ui/`）
> **基线**：2026-05-14 已完成的布局重构 + Composables 抽取 + API 层增强
> **目标**：安全加固 → 质量提升 → 性能优化 → 可维护性增强

---

## 一、现状分析

### 1.1 文件清单与依赖关系图

```
pdf-web-ui/src/
│
├── main.js                          # 入口：createApp → Pinia → Router → mount
│
├── App.vue                          # 根布局：Grid (Header / Sidebar / Main / RightBar)
│   ├── AppHeader.vue                #   顶部栏：logo + 搜索 + 功能按钮组
│   │   ├── useSearchStore           #     搜索输入 → triggerSearch → SearchOverlay
│   │   ├── useWikiStore             #     toggleTheme, navigateTo
│   │   └── defineExpose → searchInputRef (供 useKeyboard)
│   ├── AppSidebar.vue               #   侧边栏：domain chips + tree
│   │   ├── useWikiStore             #     domainsFromTree, domainFilter, tree
│   │   └── TreeNode.vue             #      递归节点：caret + icon + active 状态
│   │       └── useWikiStore         #        navigateTo, currentPath
│   ├── main → router-view           #   主内容区（page transition）
│   │   ├── WikiBrowser.vue          #     空状态页
│   │   ├── EntryDetail.vue          #     条目详情：进度条 + 面包屑 + meta + prose + relations
│   │   │   ├── useReadingProgress   #      滚动进度 composable
│   │   │   ├── MarkdownRenderer.vue #        marked → v-html（❗XSS 风险）
│   │   │   │   ├── marked.parse()   #         GFM + code highlight
│   │   │   │   ├── [[wiki-link]]    #         regex 替换 → a.wikilink
│   │   │   │   └── getCurrentInstance() #     ❗反模式
│   │   │   └── useWikiStore         #        currentEntry, navigateTo
│   │   └── SearchResults.vue        #     路由搜索结果页
│   │       └── api.searchWiki()     #       直接调用（❓功能与 SearchOverlay 重复）
│   ├── RightBar.vue                 #   信息栏：TOC + mermaid 预览
│   │   ├── useScrollSpy             #     IntersectionObserver composable
│   │   └── useWikiStore             #     currentEntry
│   ├── SearchOverlay.vue            #   搜索浮层：domain facets + results
│   │   ├── useSearchStore           #     results, loading, domainFacets
│   │   └── useWikiStore             #     navigateTo
│   ├── dialogs/                     #   模态框组件
│   │   ├── StatsDialog.vue          #     统计：loadStats → wikiStore.stats
│   │   │   └── watch(open) → loadStats()  # ❗prop 引用不明确
│   │   ├── DomainDialog.vue         #     领域：domainsFromTree
│   │   │   └── wikiStore.domainFilter = x  # ❗直接 mutation
│   │   ├── GraphDialog.vue          #     图谱：mermaid (via CDN ❗)
│   │   │   ├── api.getWikiGraph()   #      获取图谱数据
│   │   │   ├── mermaid.js           #      CDN 动态加载 ❗（无降级）
│   │   │   ├── buildGlobalMermaid() #      实时计算（可缓存）
│   │   │   └── renderMermaid()      #      异步渲染（可能竞态）
│   │   └── SettingsModal.vue        #     设置：4 tab (config / health / compile / about)
│   │       ├── useConfigStore       #       loadConfig/loadHealth/loadCompileStatus ❗串行
│   │       └── api.uploadPdf()      #      PDF 上传
│   └── MarkdownRenderer.vue         #   （同 EntryDetail 引用）
│
├── composables/                     # 已抽取（上一轮改进成果）
│   ├── useKeyboard.js               #   / Esc 全局快捷键
│   ├── useScrollSpy.js              #   IntersectionObserver 滚动监听
│   └── useReadingProgress.js        #   阅读进度条计算
│
├── stores/                          # Pinia 状态管理
│   ├── wiki.js                      #   核心：tree / currentEntry / domainFilter / theme
│   │   ├── api.getWikiTree()        #     ✅ 使用 api 层
│   │   ├── api.getWikiEntry()       #     ✅ 使用 api 层
│   │   └── api.getWikiStats()       #     ✅ 使用 api 层
│   ├── search.js                    #   搜索：results / domainFacets / loading
│   │   └── api.searchWiki()         #     ✅ 已改为 api 层（上一轮改进）
│   └── config.js                    #   配置：configData / healthData / compileStatus
│       └── api.setConfig/removeConfig #  ✅ 乐观更新（上一轮改进）
│
├── api/
│   ├── index.js                     #   统一 API 层：ApiError + 超时 + 重试 ✅
│   └── mermaid.js                   #   动态 CDN 加载 ❗
│
├── router/index.js                  #   Hash 路由，懒加载 ✅
│                                     #   ❗无导航守卫
│
├── styles/main.css                  #   全局 CSS（OKLCH + CSS Variables）
│
└── index.html                       #   HTML 模板
```

### 1.2 技术栈分析

| 层级 | 技术 | 版本 | 状态 |
|------|------|------|------|
| 框架 | Vue 3 + Composition API | ^3.5.13 | 当前 |
| 路由 | Vue Router (Hash) | ^4.5.0 | 懒加载 ✅ |
| 状态 | Pinia (Setup Store) | ^2.3.0 | 跨 store 引用合理 |
| 构建 | Vite 6 | ^6.0.0 | `target: es2020` ✅ |
| Markdown | marked | ^15.0.0 | ❗未启用 sanitize |
| 代码高亮 | highlight.js | ^11.11.0 | ✅ treeshaking 已做 |
| 图表 | mermaid@10 (CDN) | — | ❗外部加载 |
| CSS | OKLCH + CSS Variables | — | ✅ 已规范化 |
| 组合式 | Composables（自建） | — | ✅ 3 个已抽取 |

### 1.3 已识别问题清单

```
安全性      P0:   3 项（v-html XSS / CDN 依赖 / SVG v-html）
代码质量    P1:   5 项（getCurrentInstance / import 位置 / prop 引用 / 串行请求 / direct mutation）
性能        P1/P2: 3 项（GraphDialog 计算缓存 / 竞态保护 / Settings lazy load）
可维护性    P2:   4 项（路由守卫 / SearchOverlay 去重 / 数据源统一 / 错误边界）
健壮性      P1:   1 项（mermaid 主题不随 UI 切换）
┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈
合计        16 项
```

---

## 二、分阶段实施计划

### 阶段 1：安全加固（P0，8 工时）

#### 目标
消除所有已知安全漏洞。

#### 1.1 MarkdownRenderer v-html XSS 防护

**文件**：[MarkdownRenderer.vue](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/components/MarkdownRenderer.vue)

**现状**：
```js
const html = computed(() => {
  if (!props.markdown) return ''
  let rendered = marked.parse(props.markdown)
  rendered = rendered.replace(/\[\[([^\]]+)\]\]/g, ...)
  return rendered
})
```
```html
<div class="prose" v-html="html"></div>
```

**措施**：`marked` v15 内置 `sanitizer` 选项，开启后自动过滤危险 HTML 标签和属性。
```js
marked.setOptions({
  breaks: true,
  gfm: true,
  sanitize: true,     // ← 新增
  sanitizer: null,    // null = 使用 marked 内置默认 sanitizer
  highlight: function (code, lang) { ... },
})
```

**验收**：
- `<img src=x onerror=...>` 被过滤
- `<script>alert(1)</script>` 被过滤
- 合法 markdown（链接/表格/代码块/wiki-link）不受影响

**风险**：极低。「marked」内置 sanitizer 自 v4.3+ 稳定，仅过滤 HTML 标签，不影响 markdown 语法。

---

#### 1.2 Mermaid CDN 依赖改为本地构建

**文件**：
- [mermaid.js](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/api/mermaid.js)
- [package.json](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/package.json)
- [vite.config.js](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/vite.config.js)

**现状**：
```js
script.src = 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.min.js'
```

**措施**：
```bash
npm add mermaid@10
```

```js
// api/mermaid.js 改为
let mermaidModule = null

export async function loadMermaid() {
  if (mermaidModule) return mermaidModule
  if (window.mermaid) {           // fallback: 已有全局 mermaid（iframe 场景）
    mermaidModule = window.mermaid
    return mermaidModule
  }
  const mermaid = await import('mermaid')
  mermaid.default.initialize({
    startOnLoad: false,
    theme: document.documentElement.getAttribute('data-theme') === 'dark' ? 'dark' : 'default',
    securityLevel: 'loose',
  })
  mermaidModule = mermaid.default
  return mermaidModule
}
```

```js
// vite.config.js 中增加 chunk 分割
manualChunks: {
  mermaid: ['mermaid'],
}
```

**权衡**：mermaid 打包后约 2MB → 增大构建产物。但由于前端通过 `rust_embed` 嵌入二进制，离线场景下本地加载 > CDN 不可用的风险。可保留 import 后的 try/catch → CDN fallback。

**验收**：
- `npm run build` 产物中包含 mermaid chunk
- 断网环境下图谱功能正常
- CDN 作为 optional fallback

**风险**：中。mermaid 体积较大（~2MB），会增加 Rust 二进制大小约 2MB。如果对二进制大小敏感，可配置 `external` + CDN fallback 双保险策略。

---

#### 1.3 GraphDialog SVG v-html 安全加固

**文件**：[GraphDialog.vue](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/components/GraphDialog.vue#L24)

**现状**：
```html
<div v-html="svgContent"></div>
```

**措施**：mermaid 输出的 SVG 由库内部生成，本身可信。但为防御纵深，加入 sanitization 包装：
```js
// renderMermaid 中
const { svg } = await mermaid.render(id, mermaidCode.value)
svgContent.value = sanitizeHtml(svg, {
  allowedTags: ['svg', 'g', 'path', 'rect', 'text', 'tspan', 'line', 'circle', 'ellipse', 
                 'polygon', 'polyline', 'defs', 'marker', 'style', 'title', 'desc'],
  allowedAttributes: ['d', 'x', 'y', 'width', 'height', 'viewBox', 'fill', 'stroke', 
                       'class', 'id', 'transform', 'text-anchor', 'font-size', 'font-family',
                       'marker-end', 'marker-start', 'stroke-width', 'rx', 'ry', 'cx', 'cy',
                       'r', 'x1', 'y1', 'x2', 'y2', 'points', 'dx', 'dy'],
})
```

或者更轻量：确认 mermaid v10 的 `securityLevel: 'loose'` 已经在渲染层做了 sanitization，不再额外处理。

**验收**：
- SVG 渲染与之前一致
- 注入的 `<script>` 不在 SVG 中执行

**风险**：极低。mermaid v10 的 `securityLevel` 已在内部做了基本防护。

---

### 阶段 2：代码质量提升（P1，10 工时）

#### 2.1 替换 getCurrentInstance() → 模板事件委托

**文件**：[MarkdownRenderer.vue](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/components/MarkdownRenderer.vue#L77-L85)

**现状**：
```js
import { onMounted, onBeforeUnmount, getCurrentInstance } from 'vue'
const instance = getCurrentInstance()
onMounted(() => {
  instance?.vnode.el?.addEventListener('click', handleWikilinkClick)
})
```

**措施**：直接在模板上绑定 `@click` 并使用事件委托（与 `v-html` 同元素）：
```html
<div class="prose" v-html="html" @click="handleWikilinkClick"></div>
```
删除 `onMounted/onBeforeUnmount/getCurrentInstance` 相关代码。

**验收**：
- `getCurrentInstance()` 零使用
- wiki-link 点击跳转功能不变

**风险**：无。纯等价重构。

---

#### 2.2 import 语句移到文件顶部

**文件**：[MarkdownRenderer.vue](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/components/MarkdownRenderer.vue#L77-L79)

**现状**：`import { onMounted, ... } from 'vue'` 出现 L78（脚本中间），而非 L6 顶部。

**措施**：将该 import 合并到 L6 `import { computed } from 'vue'`：
```js
import { computed } from 'vue'    // L6 现状
// 改为
import { computed } from 'vue'    // L6 统一
```

删除 L77-L79 的重复 import。

**验收**：所有 `<script setup>` 中 import 在顶部。

**风险**：无。

---

#### 2.3 统一 `defineProps` 引用模式

**文件**（5 个）：
- [StatsDialog.vue](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/components/StatsDialog.vue#L46-L47)
- [DomainDialog.vue](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/components/DomainDialog.vue#L34-L35)
- [GraphDialog.vue](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/components/GraphDialog.vue#L38-L39)
- [SettingsModal.vue](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/components/SettingsModal.vue#L136-L137)

**现状**：
```js
defineProps({ open: Boolean })
defineEmits(['close'])
// ...
watch(() => open, (val) => { ... })   // open 来源不明确
```

**措施**：
```js
const props = defineProps({ open: Boolean })
const emit = defineEmits(['close'])
// ...
watch(() => props.open, (val) => { ... })
```

**验收**：
- 所有 `defineProps` 带 `const props =` 或解构
- `watch` 中明确引用 `props.open`

**风险**：无。等价重构，IDE 类型推断更准确。

---

#### 2.4 SettingsModal 初始化并行化

**文件**：[SettingsModal.vue](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/components/SettingsModal.vue#L156-L164)

**现状**：三个独立 API 调用串行执行（`await` × 3），总耗时 = sum(三个请求延迟)。
```js
await configStore.loadConfig()        // T1
await configStore.loadHealth()        // T1 + T2
await configStore.loadCompileStatus() // T1 + T2 + T3
```

**措施**：
```js
await Promise.all([
  configStore.loadConfig(),
  configStore.loadHealth(),
  configStore.loadCompileStatus(),
])
```
三个请求无依赖关系，并行执行总耗时 = max(T1, T2, T3)。

**验收**：
- 三个请求并行发出（可 Network 面板验证）
- 打开设置面板首屏渲染延迟降低 ≥ 50%

**风险**：极低。三个请求完全独立，无顺序依赖。

---

#### 2.5 DomainDialog 直接 mutation → action 调用

**文件**：[DomainDialog.vue](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/components/DomainDialog.vue#L40-L41)

**现状**：
```js
function selectDomain(domain) {
  wikiStore.domainFilter = domain === wikiStore.domainFilter ? null : domain
}
```

**措施**：在 `wiki.js` store 中新增 action：
```js
// stores/wiki.js
function setDomainFilter(domain) {
  domainFilter.value = domain === domainFilter.value ? null : domain
}
```
DomainDialog 调用：
```js
function selectDomain(domain) {
  wikiStore.setDomainFilter(domain)
}
```

**验收**：
- 无组件直接修改 store state
- 如有副作用（如 analytics），可在 action 中集中处理

**风险**：无。

---

#### 2.6 提取 formatQuality 到共享工具

**文件**：新建 `src/utils/format.js`

**现状**：
- [StatsDialog.vue:L55-L58](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/components/StatsDialog.vue#L55-L58)
- [SettingsModal.vue:L201-L205](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/components/SettingsModal.vue#L201-L205)

两处 `formatQuality` 定义几乎相同。

**措施**：
```js
// src/utils/format.js
export function formatQuality(score) {
  if (score == null) return '-'
  if (typeof score === 'string') return score
  return `${(score * 100).toFixed(0)}%`
}
```

**验收**：
- `formatQuality` 只定义一次
- 两处使用改为 `import { formatQuality } from '@/utils/format'`

**风险**：无。

---

### 阶段 3：性能优化与健壮性（P2，6 工时）

#### 3.1 GraphDialog `buildGlobalMermaid` 计算缓存

**文件**：[GraphDialog.vue](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/components/GraphDialog.vue#L81-L97)

**现状**：每次 `loadGraph()` 都重新计算 `buildGlobalMermaid()`。

**措施**：用 `computed` 替代函数调用（仅在 `domainsFromTree` 变化时重新计算）：
```js
const globalMermaidCode = computed(() => {
  const domains = wikiStore.domainsFromTree
  if (!domains.length) return 'graph TD\n  A[无数据]'
  let m = 'graph TD\n'
  // ... 构建逻辑
  return m
})
```

**验收**：
- 两次打开全局图谱之间不重新计算（无后端请求的话）
- `domainsFromTree` 变化时自动更新

**风险**：低。computed 依赖追踪正确即可。

---

#### 3.2 GraphDialog `renderMermaid` 竞态保护

**文件**：[GraphDialog.vue](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/components/GraphDialog.vue#L99-L110)

**现状**：
```js
async function renderMermaid() {
  const mermaid = await loadMermaid()
  const { svg } = await mermaid.render(id, mermaidCode.value)
  svgContent.value = svg  // 组件可能已卸载
}
```

**措施**：引入渲染版本号或 abort 标记：
```js
let renderVersion = 0

async function renderMermaid() {
  const version = ++renderVersion
  const mermaid = await loadMermaid()
  if (version !== renderVersion) return  // 已有新渲染请求
  const { svg } = await mermaid.render(id, mermaidCode.value)
  if (version !== renderVersion) return  // 组件已卸载/新请求
  svgContent.value = svg
}
```

同时修复 `watch(tab)` 的竞态：
```js
watch(tab, (val) => {
  if (val === 'visual' && !svgContent.value) {
    renderMermaid()
  }
})
```

**验收**：
- 快速切换 tab 后，只有最后一次渲染结果生效
- 关闭对话框后无内存泄漏

**风险**：低。version 机制是标准的竞态保护模式。

---

#### 3.3 Mermaid 主题响应式更新

**文件**：[mermaid.js](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/api/mermaid.js#L17)

**现状**：主题仅在首次 `initialize` 时设置，后续主题切换不生效。
```js
theme: document.documentElement.getAttribute('data-theme') === 'dark' ? 'dark' : 'default',
```

**措施**：在 `wikiStore.toggleTheme()` 中增加 mermaid 主题同步：
```js
// stores/wiki.js
function toggleTheme() {
  darkTheme.value = !darkTheme.value
  document.documentElement.setAttribute('data-theme', darkTheme.value ? 'dark' : 'light')
  // 同步 mermaid 主题
  if (mermaidModule) {
    mermaidModule.initialize({ theme: darkTheme.value ? 'dark' : 'default' })
  }
}
```

**验收**：
- 主题切换后，图谱重新渲染为对应主题

**风险**：低。需确保 `mermaidModule` 引用可从 `mermaid.js` 导出。

---

#### 3.4 SettingsModal tab 懒加载

**文件**：[SettingsModal.vue](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/components/SettingsModal.vue)

**现状**：所有 tab 内容在 dialog 打开时并行加载（阶段 2.4 改为并行），无论用户是否访问。

**措施**：
- `health` tab 数据：仅在用户切换到 health tab 时首次加载
- `compile` tab 数据：同上

```html
<div v-if="activeTab === 'health'">
  <!-- 已有 v-if，数据加载移到 watch(activeTab) -->
</div>
```

```js
watch(activeTab, (tab) => {
  if (tab === 'health' && !configStore.healthData) configStore.loadHealth()
  if (tab === 'compile' && !configStore.compileStatus) configStore.loadCompileStatus()
})
```

**验收**：
- 打开设置面板只加载 `config` tab 数据
- 切换到 health/compile tab 时按需加载

**风险**：低。用户快速切换 tab 可能触发多次请求，需加 loading 保护。

---

### 阶段 4：可维护性增强（P2，8 工时）

#### 4.1 路由导航守卫

**文件**：[router/index.js](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/router/index.js)

**措施**：
```js
router.beforeEach((to, from) => {
  // 无效路径重定向
  if (!to.matched.length) {
    return { path: '/' }
  }
})
```

同时为 `/search` 路由添加 `query.q` 校验。

**验收**：
- 访问不存在的路径 → 重定向到首页
- `/search?q=` 空参数 → 显示提示

**风险**：低。守卫逻辑简单。

---

#### 4.2 错误边界组件

**文件**：新建 `src/components/ErrorState.vue`

**措施**：
```html
<template>
  <div class="empty-entry">
    <div class="icon">{{ icon }}</div>
    <h2>{{ title }}</h2>
    <p>{{ message }}</p>
    <button v-if="retryable" class="btn" @click="$emit('retry')">重试</button>
  </div>
</template>
```

替换 [EntryDetail.vue](file:///home/smile/github_project/rsut_pdf_mcp/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/views/EntryDetail.vue#L6-L10) 中手动写的错误 UI。

**验收**：
- `ErrorState` 在 ≥ 2 个 view 中复用
- Props 完整：`icon`, `title`, `message`, `retryable`

**风险**：无。

---

#### 4.3 SearchOverlay 与 /search 路由去重决策

**现状**：同一搜索功能有两种入口：
- SearchOverlay：Header 搜索 → 浮层展示
- `/search?q=...`：独立路由页面

**建议**：统一为 SearchOverlay（浮层）。删除 `/search` 路由和 `SearchResults.vue`，因为：
1. 浮层交互更快（无页面跳转的 layout shift）
2. 减少维护两份搜索 UI
3. 与工具栏搜索体验一致

**验收**：
- `/search` 路由不存在
- `SearchResults.vue` 已归档或删除
- Header 搜索 100% 走 SearchOverlay

**风险**：中。需确认无外部链接依赖 `/search?q=`。如果 API 客户端通过 URL 触发搜索，需保留路由但内部重定向到 SearchOverlay。

---

#### 4.4 统计数据源统一

**现状**：
- `wikiStore.stats`（StatsDialog 使用）
- `configStore.healthData`（SettingsModal Health Tab 使用）

两者展示类似字段但数据源不同（`/wiki/stats` vs `/health`）。

**措施**：
- 确认两个 API 返回的字段映射关系
- 统一为一个数据源，或在 StatsDialog 中明确标注数据来源

**验收**：
- 两处展示的数据一致或明确标注差异

**风险**：低。

---

## 三、实施路线图

```
Day 1-2           Day 3-4           Day 5-6           Day 7-8
┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│ 阶段1: 安全  │   │ 阶段2: 质量  │   │ 阶段3: 性能  │   │ 阶段4: 可维护│
│             │   │             │   │             │   │             │
│ 1.1 XSS     │   │ 2.1 getCI   │   │ 3.1 computed│   │ 4.1 路由守卫│
│ 1.2 CDN     │   │ 2.2 import  │   │ 3.2 竞态    │   │ 4.2 ErrorState
│ 1.3 SVG     │──▶│ 2.3 props   │──▶│ 3.3 主题    │──▶│ 4.3 去重    │
│             │   │ 2.4 并行    │   │ 3.4 lazy    │   │ 4.4 数据源  │
│             │   │ 2.5 action  │   │             │   │             │
│             │   │ 2.6 util    │   │             │   │             │
│ 8工时        │   │ 10工时      │   │ 6工时        │   │ 8工时        │
└─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘
```

**总工时**：32 工时（单人 8 个工作日，或双人 4-5 个工作日）。

**验收节点**：
- 每阶段完成后 `npm run build` 零 error
- 阶段 1/2 合并时需人工回归测试 wiki-link、图谱渲染、搜索
- 阶段 3 合并时需测试主题切换 + tab 快速切换
- 阶段 4 合并时需测试路由重定向

---

## 四、风险矩阵

| # | 风险 | 阶段 | 概率 | 影响 | 缓解 |
|---|------|------|------|------|------|
| R1 | `marked.sanitize` 过度过滤 markdown 内嵌 HTML | 1.1 | 低 | 低 | 使用 marked 内置默认 sanitizer（保守），不自定义规则 |
| R2 | mermaid npm 化增加二进制体积 >5MB | 1.2 | 中 | 中 | 保留 CDN fallback；评估后决定方案 |
| R3 | `buildGlobalMermaid` computed 依赖追踪错误 | 3.1 | 低 | 低 | `domainsFromTree` 是 computed，追踪链清晰 |
| R4 | 删除 /search 路由破坏外部链接 | 4.3 | 中 | 中 | 保留路由但内部 redirect + toast 提示 |
| R5 | 并行化 Settings 请求打满后端连接 | 2.4 | 低 | 低 | 三个请求对后端压力可忽略 |

---

## 五、验收度量

| 指标 | 当前 | 目标 |
|------|------|------|
| XSS 向量（v-html 无 sanitization） | 2 | 0 |
| getCurrentInstance() 使用 | 1 | 0 |
| 非顶部 import 语句 | 1 | 0 |
| 未捕获 `const props =` 的 defineProps | 5 | 0 |
| 直接 store mutation | 1 | 0 |
| 重复工具函数 | 1 对 | 0 |
| 串行可并行的 API 调用 | 1 处 | 0 |
| 无竞态保护的异步渲染 | 1 处 | 0 |
| 路由守卫 | 0 | ≥ 1 |
| 错误边界复用 | 0 | ≥ 2 views |
| CDN-only 功能 | 1 (mermaid) | 0 (npm + CDN fallback) |
| `npm run build` status | 待测 | 零 error 零 warning |