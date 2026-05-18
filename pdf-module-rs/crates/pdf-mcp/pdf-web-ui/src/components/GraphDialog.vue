<template>
  <Transition name="fade">
    <div v-if="open" class="overlay open" @click.self="emit('close')">
      <div class="dialog graph-dialog">
        <button class="dialog-close" @click="emit('close')">
          <X :size="16" />
        </button>

        <div class="graph-header">
          <div class="graph-icon">
            <GitBranch :size="18" />
          </div>
          <div class="graph-title-wrap">
            <div class="graph-title">{{ graphTitle }}</div>
            <div class="graph-subtitle">{{ subtitle }}</div>
          </div>
        </div>

        <div class="graph-toolbar">
          <div class="graph-tabs">
            <button class="graph-tab" :class="{ active: tab === 'visual' }" @click="tab = 'visual'">
              <Layout :size="13" />可视化
            </button>
            <button class="graph-tab" :class="{ active: tab === 'raw' }" @click="tab = 'raw'">
              <Code2 :size="13" />源码
            </button>
          </div>
          <div class="graph-toolbar-right">
            <div class="graph-zoom-controls">
              <button class="graph-zoom-btn" @click="zoomIn" title="放大">+</button>
              <button class="graph-zoom-btn" @click="zoomOut" title="缩小">−</button>
              <button class="graph-zoom-btn" @click="zoomReset" title="重置">⟲</button>
            </div>
          </div>
        </div>

        <div v-show="tab === 'visual'" class="graph-container" ref="graphContainer">
          <div v-if="loading" class="graph-loading">
            <span class="dots-loading"></span>
            <span>正在加载图谱…</span>
          </div>
          <div ref="svgWrap" v-html="svgContent" v-if="!loading && svgContent"></div>
          <EmptyState
            v-if="!loading && !svgContent"
            icon="tree"
            title="暂无图谱数据"
            description="选择一个条目查看其概念图谱"
          />
        </div>

        <pre v-show="tab === 'raw'" class="graph-raw">{{ mermaidCode || '无图谱数据' }}</pre>
      </div>
    </div>
  </Transition>
</template>

<script setup>
import { ref, computed, watch, nextTick } from 'vue'
import { useWikiStore } from '@/stores/wiki'
import { api } from '@/api'
import { loadMermaid } from '@/api/mermaid'
import { GitBranch, Layout, Code2, X } from 'lucide-vue-next'
import EmptyState from './EmptyState.vue'

const props = defineProps({ open: Boolean })
const emit = defineEmits(['close'])

const wikiStore = useWikiStore()

const tab = ref('visual')
const mermaidCode = ref('')
const svgContent = ref('')
const subtitle = ref('')
const loading = ref(false)
const graphContainer = ref(null)
const svgWrap = ref(null)
const graphTitle = ref('知识图谱')

let renderVersion = 0
const svgScale = ref(1)

const globalMermaidCode = computed(() => {
  const domains = wikiStore.domainsFromTree
  if (!domains.length) return 'graph TD\n  A[无数据]'
  let m = 'graph TD\n'
  const domainIds = []
  for (const d of domains) {
    const did = 'D_' + d.domain.replace(/[^a-zA-Z0-9]/g, '_')
    domainIds.push(did)
    m += `  ${did}["📁 ${esc(d.domain)} (${d.count})"]\n`
    for (const p of d.paths) {
      const eid = 'E_' + p.replace(/[^a-zA-Z0-9]/g, '_')
      m += `  ${did} --> ${eid}\n`
    }
  }
  m += '  classDef domain fill:#1f6feb,stroke:#58a6ff,color:#fff\n'
  domainIds.forEach(n => { m += `  class ${n} domain\n` })
  return m
})

async function loadGraph() {
  loading.value = true
  svgContent.value = ''
  mermaidCode.value = ''
  svgScale.value = 1
  let code = ''

  if (wikiStore.currentPath) {
    subtitle.value = '条目: ' + wikiStore.currentPath
    graphTitle.value = '条目概念图谱'
    try {
      const d = await api.getWikiGraph(wikiStore.currentPath)
      code = d.mermaid || ''
    } catch (e) {
      code = ''
    }
  } else {
    subtitle.value = '全局知识库结构'
    graphTitle.value = '全局知识图谱'
    code = globalMermaidCode.value
  }

  mermaidCode.value = code || 'graph TD\n  A[暂无图谱数据]'

  if (tab.value === 'visual') {
    await renderMermaid()
  }
  loading.value = false
}

async function renderMermaid() {
  if (!mermaidCode.value) return
  const version = ++renderVersion
  await nextTick()
  try {
    const mermaid = await loadMermaid()
    if (version !== renderVersion) return
    const theme = document.documentElement.getAttribute('data-theme') === 'dark' ? 'dark' : 'default'
    mermaid.initialize({
      theme,
      securityLevel: 'strict',
      startOnLoad: false,
      fontFamily: 'Inter, -apple-system, sans-serif',
    })
    const id = 'mermaid-graph-' + Date.now()
    const { svg } = await mermaid.render(id, mermaidCode.value)
    if (version !== renderVersion) return
    svgContent.value = svg
    applySvgScale()
  } catch (e) {
    if (version !== renderVersion) return
    svgContent.value = `<span style="color:var(--error);font-size:0.8125rem">图谱渲染失败: ${esc(e.message)}</span>`
  }
}

function applySvgScale() {
  nextTick(() => {
    if (svgWrap.value) {
      svgWrap.value.style.transform = `scale(${svgScale.value})`
      svgWrap.value.style.transition = 'transform 200ms var(--ease-spring)'
    }
  })
}

function zoomIn() {
  svgScale.value = Math.min(2, svgScale.value + 0.1)
  applySvgScale()
}

function zoomOut() {
  svgScale.value = Math.max(0.3, svgScale.value - 0.1)
  applySvgScale()
}

function zoomReset() {
  svgScale.value = 1
  applySvgScale()
}

watch(tab, (val) => {
  if (val === 'visual' && !svgContent.value) {
    renderMermaid()
  }
})

watch(() => props.open, async (val) => {
  if (val) {
    tab.value = 'visual'
    svgScale.value = 1
    await loadGraph()
  }
})

function esc(s) {
  if (!s) return ''
  const d = document.createElement('div')
  d.textContent = s
  return d.innerHTML
}
</script>

<style scoped>
.graph-dialog :deep(svg) {
  transform-origin: center center;
}
</style>