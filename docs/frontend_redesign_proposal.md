# Compendium 前端重设计提案

> **基线**: `pdf-web-ui/` — Vue 3 + Vite 6 + Pinia + OKLCH Design Tokens
> **日期**: 2026-05-22
> **范围**: 视觉重设计 + 交互升级 + 架构优化

---

## 一、现状评估

### ✅ 已有的良好基础

| 维度 | 现状 | 评价 |
|------|------|------|
| **设计体系** | OKLCH tokens + CSS Variables + 亮暗主题 | 优秀基础 |
| **布局架构** | CSS Grid 三栏 + 侧栏折叠 + 阅读模式 | 功能完整 |
| **组件化** | 17 组件 + 4 composables + 5 stores | 分层合理 |
| **技术栈** | Vue 3 Composition API + Pinia Setup Store | 现代化 |
| **国际化** | vue-i18n 已集成 | 可扩展 |
| **图标系统** | Lucide Vue Next | 轻量美观 |
| **字体策略** | Inter + JetBrains Mono | 专业选择 |

### ⚠️ 需要改进的方面

| 问题 | 严重度 | 详情 |
|------|--------|------|
| **首页空状态单调** | 高 | `WikiBrowser.vue` 只有一个 emoji + 两行文字，第一印象差 |
| **视觉层次扁平** | 高 | 大量组件使用相同的 `var(--surface)` + `var(--border)` 模式，缺乏纵深感 |
| **动效缺乏韵律** | 中 | 有 transition 但无编排逻辑，页面切换感生硬 |
| **数据可视化弱** | 中 | 统计面板仅数字展示，无趋势图/进度环等视觉元素 |
| **移动端适配缺失** | 高 | 无响应式断点，移动端完全不可用 |
| **空白页面多** | 中 | 加载/空/错误状态的视觉质量参差不齐 |
| **侧栏信息密度** | 低 | 树状导航可优化分组和视觉层次 |
| **Settings Modal 过重** | 中 | 17KB 组件，4 个 tab 全量渲染 |

---

## 二、重设计方向建议

### 方向 A: 🔬 **「Knowledge Studio」— 专业工作台风格**

> 灵感: Linear、Notion、Raycast

```
┌─────────────────────────────────────────────────────┐
│  ░░ Command Bar (⌘K) ░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
├──────────┬──────────────────────────────┬────────────┤
│          │                              │            │
│  Outline │  Rich Knowledge Canvas       │  Context   │
│  + Quick │  with Callouts, Graphs,      │  Panel     │
│  Actions │  Inline Mermaid              │  TOC +     │
│          │                              │  Graph +   │
│          │                              │  Related   │
└──────────┴──────────────────────────────┴────────────┘
```

**核心变化:**
- ⌘K Command Palette 取代搜索浮层
- 侧栏增加「快速操作」区域 (编译、搜索、统计)
- 主内容区引入 Callout 块、引用高亮、内联图谱
- 右栏改为上下文感知面板 (根据当前页面动态切换)

**适合场景**: 深度知识工作者、研究型用户

---

### 方向 B: 📚 **「Digital Library」— 现代数字图书馆**

> 灵感: Apple Books、Stripe Docs、GitBook

```
┌─────────────────────────────────────────────────────┐
│  Logo   [ 搜索... ]          🌙 ⚙️  📊             │
├──────────┬──────────────────────────────────────────┤
│          │                                          │
│  Domain  │  ┌─────────────────────────────────┐     │
│  Nav     │  │  Hero Banner + Knowledge Stats  │     │
│          │  └─────────────────────────────────┘     │
│  ── ── ──│                                          │
│          │  Recent Entries     Popular Domains       │
│  Tree    │  ┌────┐ ┌────┐     ┌────────────┐       │
│  Browser │  │    │ │    │     │ 🧠 Domain A │       │
│          │  └────┘ └────┘     └────────────┘       │
└──────────┴──────────────────────────────────────────┘
```

**核心变化:**
- 首页改为「知识库仪表盘」(统计 + 最近 + 热门)
- 引入 Hero Banner 展示知识库整体健康度
- 条目详情页优化排版 (更宽的 prose 区域 + 浮动 TOC)
- 图谱预览嵌入条目页面底部

**适合场景**: 知识库浏览、团队协作

---

### 方向 C: 🧪 **「Hybrid Dashboard」— 数据驱动型** ⭐ 推荐

> 灵感: Vercel Dashboard、Supabase Studio、Arc Browser

```
┌───────────────────────────────────────────────────────┐
│  🔖 Compendium  ┃ [KB Selector] ┃ [⌘K Search]  🌙 ⚙ │
├────────┬──────────────────────────────────┬────────────┤
│        │                                  │            │
│ Smart  │  ╔═══════════════════════════╗   │ Contextual │
│ Nav    │  ║  Dynamic Landing:         ║   │ Insights   │
│        │  ║  · KB Health Ring         ║   │            │
│ ▸ 域A  │  ║  · Recent Activity Feed   ║   │ ▸ TOC      │
│ ▸ 域B  │  ║  · Quick Actions Grid     ║   │ ▸ Graph    │
│ ▸ 域C  │  ╚═══════════════════════════╝   │ ▸ Related  │
│        │                                  │ ▸ Quality  │
│        │  OR                              │            │
│        │                                  │            │
│        │  ╔═══════════════════════════╗   │            │
│        │  ║  Entry Detail:            ║   │            │
│        │  ║  · Refined Typography     ║   │            │
│        │  ║  · Inline Annotations     ║   │            │
│        │  ║  · Embedded Mermaid       ║   │            │
│        │  ╚═══════════════════════════╝   │            │
└────────┴──────────────────────────────────┴────────────┘
```

**核心变化一览:**

#### 1. 首页重塑：知识库仪表盘
- **健康度环形图**: 用 SVG 动画展示 quality_score 分布
- **活动时间线**: 最近编译/修改记录流式展示
- **快速动作网格**: 4 个核心入口 (搜索、编译、图谱、设置)
- **域分布热力图**: 可视化知识领域覆盖度

#### 2. 搜索体验：Command Palette
- ⌘K 弹出全局命令面板，融合搜索 + 快捷操作
- 模糊匹配 + 域过滤 + 最近访问
- 替换当前的 SearchOverlay，统一交互模型

#### 3. 条目详情：沉浸式阅读
- 更精致的 Prose 排版 (行间距、段落间距优化)
- 标题锚点悬浮高亮
- Mermaid 图内联渲染 (条目内 graph 直接展示)
- 「阅读模式」增强：去除一切 chrome，全屏 prose

#### 4. 右栏：上下文智能面板
- 根据当前页面自动切换内容
- 条目页：TOC + 质量报告 + 关联图谱迷你视图
- 首页：全局统计摘要
- 搜索中：搜索历史 + 建议

#### 5. 动效编排系统
- **页面切换**: 内容区 fade + slide-up，侧栏保持不变
- **列表项目**: 交错入场 (staggered animation)
- **交互反馈**: press scale (0.97) + subtle bounce
- **骨架屏**: shimmer 从左到右的波光效果

#### 6. 响应式设计
- **≤768px**: 底部导航 + 抽屉侧栏 + 全屏条目
- **768-1024px**: 折叠侧栏 + 隐藏右栏
- **≥1024px**: 完整三栏布局

---

## 三、具体实施建议

### Phase 1: 基础升级 (3-4 天)

| 任务 | 涉及文件 | 描述 |
|------|----------|------|
| **设计令牌增强** | `tokens.css` | 新增渐变、模糊度、animation 令牌 |
| **首页重设计** | `WikiBrowser.vue` | 从空状态 → 知识库仪表盘 |
| **响应式断点** | `layout.css` | 添加 mobile/tablet media queries |
| **骨架屏升级** | `main.css` + 新组件 | shimmer 动画 + 组件化骨架 |
| **Command Palette** | 新组件 `CommandPalette.vue` | 替换 SearchOverlay，统一 ⌘K 入口 |

### Phase 2: 交互优化 (2-3 天)

| 任务 | 涉及文件 | 描述 |
|------|----------|------|
| **页面转场动效** | `App.vue` + `main.css` | 优化 `<transition>` 动画编排 |
| **列表交错动画** | 树节点 + 搜索结果 | staggered enter animations |
| **Prose 排版升级** | `main.css` prose 部分 | 更精致的 Markdown 渲染样式 |
| **TOC 浮动高亮** | `RightBar.vue` | 滚动同步 + smooth highlight |
| **微交互** | 全局 | press feedback, hover glow, focus ring |

### Phase 3: 数据可视化 (2 天)

| 任务 | 涉及文件 | 描述 |
|------|----------|------|
| **质量环形图** | `StatsDialog.vue` / 首页 | SVG-based donut chart |
| **域分布图** | 首页仪表盘 | 水平条形图/热力色块 |
| **活动时间线** | 首页 | 最近编译活动的时间线展示 |
| **迷你关联图** | `RightBar.vue` | 条目的 N-hop 邻居迷你图谱 |

### Phase 4: 打磨与移动端 (2 天)

| 任务 | 涉及文件 | 描述 |
|------|----------|------|
| **移动端导航** | `layout.css` + `App.vue` | 底部 Tab Bar + 滑动抽屉 |
| **性能优化** | 多文件 | 组件懒加载 + CSS 分割 |
| **FRONTEND_IMPROVEMENT_PLAN 收尾** | 16 项技术债 | 整合之前的改进计划 |
| **端到端测试** | `e2e/` | 关键流程测试覆盖 |

---

## 四、视觉概念预览

### 色彩方案优化建议

当前的 Deep Teal 色系是好的选择，但可以增加更丰富的渐变层次：

```css
/* 新增渐变令牌 */
--gradient-hero: linear-gradient(135deg, 
  oklch(0.20 0.02 250) 0%, 
  oklch(0.18 0.04 200) 50%, 
  oklch(0.15 0.01 250) 100%);

--gradient-card: linear-gradient(180deg, 
  color-mix(in oklch, var(--surface) 90%, var(--primary) 10%) 0%,
  var(--surface) 100%);

--gradient-accent: linear-gradient(135deg, 
  var(--primary) 0%, 
  var(--primary-light) 100%);

/* 光晕效果 */
--glow-primary: 0 0 20px color-mix(in oklch, var(--primary) 15%, transparent),
                0 0 60px color-mix(in oklch, var(--primary) 5%, transparent);
```

### 排版优化

```css
/* 更精致的 prose 排版 */
.prose {
  font-size: 0.9375rem;
  line-height: 1.8;           /* 从 1.75 微调 */
  letter-spacing: -0.008em;   /* 更精细的字距 */
  font-feature-settings: 'kern' 1, 'liga' 1, 'calt' 1, 'ss01' 1;
}

.prose h1 { 
  font-size: 1.75em; 
  font-weight: 800;        /* 更大胆的标题 */
  background: var(--gradient-accent);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}
```

---

## 五、技术债整合

> [!IMPORTANT]
> 此重设计应与 [FRONTEND_IMPROVEMENT_PLAN.md](file:///home/smile/github_project/Compendium/FRONTEND_IMPROVEMENT_PLAN.md) 中的 16 项技术债同步推进。

以下项目可在重设计过程中自然解决：

| 原计划项 | 重设计中的对应工作 |
|----------|-------------------|
| 1.2 Mermaid CDN → 本地 | 已完成 (package.json 已有 mermaid 依赖) |
| 2.1 去除 getCurrentInstance | Prose 组件重写时一并处理 |
| 2.4 Settings 并行化 | Settings 面板重新设计时自然解决 |
| 4.2 ErrorState 复用 | 已有 `ErrorState.vue`，重设计中统一样式 |
| 4.3 SearchOverlay 去重 | Command Palette 替换后自然消除 |

---

## 六、关键决策点

> [!NOTE]
> 请确认以下选择，我将据此开始实施。

### 决策 1: 整体方向

- **A: Knowledge Studio** — 重操作，适合深度用户
- **B: Digital Library** — 重浏览，适合知识共享
- **C: Hybrid Dashboard** ⭐ — 兼顾两者，数据驱动 (推荐)

### 决策 2: 搜索模式

- **保留 SearchOverlay** — 改进现有体验，风险低
- **引入 Command Palette (⌘K)** ⭐ — 现代化交互，替换 SearchOverlay + 快捷操作

### 决策 3: 首页策略

- **保留空状态** — 仅美化当前设计
- **知识库仪表盘** ⭐ — 新增统计/活动/快捷入口首页

### 决策 4: 移动端优先级

- **暂不处理** — 聚焦桌面体验
- **响应式适配** ⭐ — 基本的移动端可用性
- **移动端原生体验** — 完整的移动端交互设计

### 决策 5: 实施节奏

- **一次性全面重设计** — 风险高，效果好
- **渐进式改进** ⭐ — 按 Phase 逐步推进，每阶段可交付

---

*总预估工时: 9-11 天 (单人全职)*
