<template>
  <Transition name="fade">
    <div v-if="open" class="overlay open" @click.self="$emit('close')">
      <div class="dialog graph-dialog">
        <button class="dialog-close" @click="$emit('close')">&times;</button>
        <h2>{{ graphTitle }}</h2>
        <div class="graph-toolbar">
          <span class="graph-title">{{ subtitle }}</span>
          <div class="graph-tabs">
            <span
              class="graph-tab"
              :class="{ active: tab === 'visual' }"
              @click="tab = 'visual'"
            >可视化</span>
            <span
              class="graph-tab"
              :class="{ active: tab === 'raw' }"
              @click="tab = 'raw'"
            >源码</span>
          </div>
        </div>
        <div v-show="tab === 'visual'" class="graph-container" ref="graphContainer">
          <span v-if="loading" class="graph-loading">正在加载图谱…</span>
          <div v-html="svgContent" v-if="!loading"></div>
        </div>
        <pre v-show="tab === 'raw'" class="graph-raw" style="display:block;">{{ mermaidCode || '无图谱数据' }}</pre>
      </div>
    </div>
  </Transition>
</template>

<script setup>
import { ref, watch, nextTick } from 'vue'
import { useWikiStore } from '@/stores/wiki'
import { api } from '@/api'
import { loadMermaid } from '@/api/mermaid'

defineProps({ open: Boolean })
defineEmits(['close'])

const wikiStore = useWikiStore()

const tab = ref('visual')
const mermaidCode = ref('')
const svgContent = ref('')
const subtitle = ref('')
const loading = ref(false)
const graphContainer = ref(null)

const graphTitle = ref('🔗 知识图谱')

async function loadGraph() {
  loading.value = true
  svgContent.value = ''
  mermaidCode.value = ''
  let code = ''

  if (wikiStore.currentPath) {
    subtitle.value = '条目: ' + wikiStore.currentPath
    graphTitle.value = '🔗 条目概念图谱'
    try {
      const d = await api.getWikiGraph(wikiStore.currentPath)
      code = d.mermaid || ''
    } catch (e) {
      code = ''
    }
  } else {
    subtitle.value = '全局知识库结构'
    graphTitle.value = '🔗 全局知识图谱'
    code = buildGlobalMermaid()
  }

  mermaidCode.value = code || 'graph TD\n  A[暂无图谱数据]'

  if (tab.value === 'visual') {
    await renderMermaid()
  }
  loading.value = false
}

function buildGlobalMermaid() {
  const domains = wikiStore.domainsFromTree
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
  return m || 'graph TD\n  A[无数据]'
}

async function renderMermaid() {
  if (!mermaidCode.value) return
  await nextTick()
  try {
    const mermaid = await loadMermaid()
    const id = 'mermaid-graph-' + Date.now()
    const { svg } = await mermaid.render(id, mermaidCode.value)
    svgContent.value = svg
  } catch (e) {
    svgContent.value = `<span style="color:var(--error);font-size:0.8125rem">图谱渲染失败: ${esc(e.message)}</span>`
  }
}

watch(tab, async (val) => {
  if (val === 'visual' && !svgContent.value) {
    await renderMermaid()
  }
})

watch(() => open, async (val) => {
  if (val) {
    tab.value = 'visual'
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
