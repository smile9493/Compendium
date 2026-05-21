# Compendium 前端优化计划：知识阅读器 (Knowledge Reader)

> **定位**: Compendium 是一本在线知识书籍的浏览器。用户在这里阅读 AI 编译好的结构化知识，而非操作仪表盘或写代码。
> **灵感来源**: Obsidian Publish · GitBook · Notion Published Pages · Stripe Docs · Bear Notes
> **核心原则**: 内容为王 (Content-First)，让界面消失，让知识涌现。

---

## 一、风格定位：「温暖的深色书房」

不是冰冷的控制台，不是花哨的 SaaS 仪表盘，而是一间**安静的、光线恰到好处的深色书房**——你坐在里面翻阅一本精心编排的百科全书。

### 核心美学

| 维度 | 当前状态 | 目标状态 |
|------|---------|---------|
| **整体感受** | 功能堆砌的工具面板 | 沉浸式知识阅读体验 |
| **视觉重心** | 侧栏/顶栏/按钮均分注意力 | 90% 注意力聚焦在正文区 |
| **色彩温度** | 冷色调蓝灰 | 微暖的深灰+克制的青色点缀 |
| **装饰程度** | 多种阴影/光晕/渐变 | 极简——仅靠字体层级和留白 |
| **信息密度** | 中等 | 阅读区低密度（大量留白），导航区高密度 |

### 概念图

![Knowledge Wiki Reading Experience](/home/smile/.gemini/antigravity/brain/aa939bbc-0c62-4098-8a47-d5953b8adebe/compendium_wiki_reading_1779388739672.png)

---

## 二、具体设计方案

### 2.1 排版 (Typography) — 这是最重要的改进

知识库的灵魂是排版。当前的 `.prose` 样式已经不错，但还需要进一步打磨到"出版物级别"。

**正文区 (Prose):**
- 字体保持 `Inter`，行高从 `1.75` 提升到 `1.85`
- 段落间距加大（`margin: 1.2em 0`），让每段知识像独立的呼吸单元
- 阅读区最大宽度收窄到 `680px`（从 `740px`），更接近出版物的最佳阅读行宽（每行 60-75 个字符）
- 正文颜色从纯白降低一档（`oklch(0.88 0.005 250)`），减轻阅读疲劳

**标题层级:**
- `h1` 使用 `font-weight: 700`，字号 `1.875em`，底部加一道极淡的分隔线
- `h2` 使用 `font-weight: 650`，上方留出 `2.5em` 间距，制造"章节感"
- `h3`/`h4` 适度缩小，与正文的对比度适中
- 所有标题保持 `letter-spacing: -0.02em`，紧凑而有力

**代码块:**
- 背景色微微区别于主背景，用极细边框勾勒
- 行号颜色极淡，不干扰代码本身
- 代码字号 `0.85em`，与正文形成清晰的层级差异

**表格:**
- 去掉条纹背景（zebra stripes），改用极淡的行间分隔线
- 表头用稍重的字重（`600`）而非背景色来区分
- 表格整体与正文等宽，不溢出

---

### 2.2 色彩 (Color) — 减法策略

**核心原则：青色（Teal）只在需要用户注意的地方出现。**

| 元素 | 当前 | 调整 |
|------|------|------|
| 链接/Wiki-link | `var(--primary-light)` 亮青 | 保持，这是唯一大量使用彩色的地方 |
| 侧栏活动项 | 青色背景 + 左边框 | 改为**仅左边框**，背景极淡或无 |
| 各类 Badge | 6 种不同色彩 | 统一为 2-3 种：正常(灰)/警告(暖橙)/错误(红) |
| 按钮 | 多种样式 | 统一为 ghost 样式（文字+极淡边框），仅主操作用实心 |
| 背景发光效果 | `radial-gradient` 伪元素 | 移除。纯净的深色背景 |

**新增的微暖调整:**
```css
/* 背景从纯冷灰微调为带一丝暖意的深灰 */
--bg: oklch(0.14 0.008 260);        /* 极微的蓝紫底调，比纯冷灰更有书房感 */
--surface: oklch(0.18 0.008 260);
--text: oklch(0.88 0.005 260);       /* 正文不用纯白，降低刺激 */
--text-secondary: oklch(0.62 0.008 260);
```

---

### 2.3 布局 (Layout) — 让阅读区成为绝对主角

**三栏比例调整:**
```
当前:  侧栏 270px  |  内容 1fr  |  右栏 240px
目标:  侧栏 240px  |  内容 1fr  |  右栏 200px
```
- 侧栏和右栏各缩窄 30-40px，让阅读区更宽敞
- 侧栏字号缩小到 `0.8125rem`，降低对阅读区的视觉争夺

**顶部栏弱化:**
- 高度从 `48px` 降到 `42px`
- 背景色与主背景融合（不再使用半透明玻璃效果），仅保留底部 1px 分隔线
- Logo 和工具按钮色彩降低（使用 `--text-muted`），hover 时才亮起

**右栏优化:**
- TOC（目录）的视觉层次通过缩进深度来表达，而非字号变化
- 移除 Mermaid 预览区的等宽代码展示，改为简洁的纯文本关系列表
- 整体色彩极淡，避免与正文区争夺视线

**阅读区留白:**
- 正文区 `padding` 增大（`padding: 2.5rem 2rem 6rem`）
- 正文块 `max-width: 680px`，居中显示
- 标题与正文之间的 Meta 信息区压缩为单行

---

### 2.4 组件优化 — 减少"存在感"

**条目详情页 (EntryDetail):**
- 移除 `.entry-card` 的边框和背景，标题直接铺在页面上
- Meta 信息（域/层级/质量分/版本）从醒目的彩色 Badge 改为**灰色小字、内联排列**，类似书籍封面下方的出版信息
- 标签（Tags）从圆角胶囊改为朴素的 `#tag` 文字形式，用等宽字体

**搜索覆盖层 (SearchOverlay):**
- 不做成花哨的 Command Palette
- 保持当前的下拉式覆盖层，但优化搜索结果的排版
- 搜索结果卡片去掉边框和阴影，改为列表形式（标题+摘要+域标签），用悬停时的微淡背景色区分

**对话框 (Dialogs):**
- 圆角从 `16px` 降到 `8px`
- 移除外层半透明遮罩的 blur 效果，改为纯色半透明遮罩
- 内部布局保持当前结构，仅微调字号和间距

**首页 (WikiBrowser):**
- 不做仪表盘。保持"空状态欢迎页"的定位
- 优化排版：使用更优雅的欢迎文案和键盘快捷键提示
- 可选：展示"最近浏览的 3 个条目"链接列表（纯文本链接，无卡片）

---

## 三、不做的事情

> [!IMPORTANT]
> 以下元素明确**不引入**，以保持 Wiki 阅读器的纯粹性：

- ❌ 毛玻璃效果 (Glassmorphism / backdrop-filter: blur)
- ❌ 卡片网格 (Card Grids)
- ❌ 仪表盘/数据看板
- ❌ Command Palette (⌘K)
- ❌ 渐变文字
- ❌ 环形图/趋势图等数据可视化组件
- ❌ 交错入场动画 (Staggered animations)
- ❌ 全站等宽字体

---

## 四、实施计划

### Phase 1: Typography & Color 精修 (2 天)

核心目标：让阅读体验达到出版物级别。

#### [MODIFY] [tokens.css](file:///home/smile/github_project/Compendium/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/styles/tokens.css)
- 调整背景色温度（微暖化）
- 降低正文颜色亮度
- 缩小圆角（`--radius: 6px`, `--radius-lg: 8px`）
- 缩窄侧栏/右栏/顶栏尺寸
- 缩窄内容区最大宽度（`--content-max: 680px`）

#### [MODIFY] [main.css](file:///home/smile/github_project/Compendium/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/styles/main.css)
- `.prose` 排版全面精修（行高、段距、标题间距、代码块、表格、引用块）
- `.entry-card` 去边框去阴影，扁平化
- Badge 颜色统一为灰色系，降低色彩噪音
- Meta 信息区紧凑化

#### [MODIFY] [layout.css](file:///home/smile/github_project/Compendium/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/styles/layout.css)
- 顶栏高度缩减，去除 `backdrop-filter`，背景融入页面
- 移除 `.app-layout::before` 的发光伪元素
- 侧栏/右栏宽度缩减
- 阅读区 padding 加大

---

### Phase 2: Component Polish (1-2 天)

#### [MODIFY] [EntryDetail.vue](file:///home/smile/github_project/Compendium/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/views/EntryDetail.vue)
- Meta Badge 改为灰色内联文字
- Tags 改为 `#tag` 朴素文字形式

#### [MODIFY] [WikiBrowser.vue](file:///home/smile/github_project/Compendium/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/views/WikiBrowser.vue)
- 优化欢迎页文案和排版
- 可选：增加"最近浏览"纯文本链接

#### [MODIFY] [AppHeader.vue](file:///home/smile/github_project/Compendium/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/components/AppHeader.vue)
- 降低按钮色彩强度，hover 时才亮起
- Logo 颜色弱化

#### [MODIFY] [RightBar.vue](file:///home/smile/github_project/Compendium/pdf-module-rs/crates/pdf-mcp/pdf-web-ui/src/components/RightBar.vue)
- TOC 样式优化（通过缩进而非字号区分层级）
- 整体色彩降低

---

### Phase 3: 收尾与技术债 (1 天)

- 合并 `FRONTEND_IMPROVEMENT_PLAN.md` 中已自然解决的项目
- 验证亮色主题（`[data-theme="light"]`）下的视觉一致性
- 确保 `npm run build` 零 error

---

## 五、预期效果

改造完成后，用户打开 Compendium 的感受应该是：

> "这是一本排版精美的知识百科全书，恰好长在浏览器里。"

而不是"这是一个开发者工具"或"这是一个后台管理系统"。

**总预估工时: 4-5 天（单人）**
