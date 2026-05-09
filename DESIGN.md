# PDF Module - Design System v2

## Design Philosophy

**Register**: Product (tool/admin interface)
**Color Strategy**: Committed - one saturated color carries 30-60% of surface
**Theme**: Dark - optimized for focused work in dim environments

## Colors (OKLCH)

### Primary Color - Deep Teal/Cyan
Performance, technology, reliability

- **Primary**: `oklch(0.65 0.15 195)` - Deep teal (#0d9488)
- **Primary Light**: `oklch(0.75 0.12 195)` - Lighter teal (#14b8a6)
- **Primary Dark**: `oklch(0.50 0.15 195)` - Darker teal (#0f766e)

### Neutrals (Cool-tinted)
Dark theme with subtle blue undertone

- **Background**: `oklch(0.15 0.01 250)` - Very dark blue-gray (#0f172a)
- **Surface**: `oklch(0.20 0.01 250)` - Dark surface (#1e293b)
- **Surface Hover**: `oklch(0.25 0.01 250)` - Hover state (#334155)
- **Border**: `oklch(0.30 0.01 250)` - Subtle borders (#475569)
- **Text Primary**: `oklch(0.95 0.005 250)` - Near white (#f1f5f9)
- **Text Secondary**: `oklch(0.70 0.01 250)` - Muted text (#94a3b8)
- **Text Muted**: `oklch(0.50 0.01 250)` - Very muted (#64748b)

### Semantic Colors

- **Success**: `oklch(0.70 0.15 150)` - Green (#22c55e)
- **Error**: `oklch(0.65 0.20 25)` - Red (#ef4444)
- **Warning**: `oklch(0.75 0.15 85)` - Amber (#f59e0b)
- **Info**: `oklch(0.70 0.12 240)` - Blue (#3b82f6)

### Accent Colors (Data Visualization)

- **Accent 1**: `oklch(0.70 0.15 330)` - Pink (#ec4899)
- **Accent 2**: `oklch(0.75 0.15 65)` - Orange (#f97316)
- **Accent 3**: `oklch(0.70 0.15 280)` - Purple (#a855f7)

## Typography

### Font Stack
- **Primary**: JetBrains Mono - Technical, precise, modern
- **Secondary**: Inter - Clean, readable for longer text

### Scale (Major Third 1.25)
- **Display**: 3rem (48px) - Hero numbers
- **H1**: 2rem (32px) - Page titles
- **H2**: 1.5rem (24px) - Section titles
- **H3**: 1.25rem (20px) - Subsections
- **Body**: 1rem (16px) - Default
- **Small**: 0.875rem (14px) - Metadata
- **Micro**: 0.75rem (12px) - Labels, tags

### Line Height
- Headings: 1.1
- Body: 1.6
- Code/Data: 1.4

## Layout

### Grid
- **Columns**: 12
- **Gutter**: 1.5rem (24px)
- **Max Width**: 1400px

### Spacing Scale
- **xs**: 0.25rem (4px)
- **sm**: 0.5rem (8px)
- **md**: 1rem (16px)
- **lg**: 1.5rem (24px)
- **xl**: 2rem (32px)
- **2xl**: 3rem (48px)
- **3xl**: 4rem (64px)

### Layout Patterns

**Dashboard Layout**:
```
┌─────────────────────────────────────┐
│  Header (fixed, minimal)            │
├──────┬──────────────────────────────┤
│      │                              │
│ Nav  │  Main Content Area           │
│      │  (no cards, direct content)  │
│      │                              │
└──────┴──────────────────────────────┘
```

**Data Display**:
- Use tables for structured data
- Use inline sections for metrics
- Avoid card wrapping
- Direct content presentation

## Components

### Buttons
- **Primary**: Filled with primary color
- **Secondary**: Ghost (transparent with border)
- **Ghost**: No border, text only
- **Sizes**: sm, md, lg

### Inputs
- Dark background
- Subtle border
- Focus: primary color border glow
- Monospace for file paths

### Data Display
- **Metrics**: Large numbers, minimal labels
- **Tables**: Minimal styling, zebra optional
- **Code**: Syntax highlighted, dark background

### Navigation
- **Sidebar**: Collapsible, icon + text
- **Tabs**: Underline style
- **Breadcrumbs**: Minimal, text only

## Motion

### Principles
- Purposeful only
- Fast and snappy (150-200ms)
- Exponential ease-out

### Transitions
- Hover: 150ms ease-out
- Expand: 200ms ease-out-quart
- Page: 250ms ease-out-quint

## Visual Hierarchy

### Primary Focus Areas
1. **Metrics/Stats** - Large, prominent
2. **Actions** - Clear, accessible
3. **Data** - Organized, scannable

### Avoid
- ❌ Card grids
- ❌ Side-stripe borders
- ❌ Gradient text
- ❌ Glassmorphism
- ❌ Hero-metric template
- ❌ Identical card layouts

## Responsive

### Breakpoints
- **Mobile**: < 640px
- **Tablet**: 640px - 1024px
- **Desktop**: > 1024px
- **Wide**: > 1400px

### Behavior
- Sidebar: Always visible on desktop, overlay on mobile
- Content: Full width, no artificial constraints
- Data: Scrollable tables, not cards
