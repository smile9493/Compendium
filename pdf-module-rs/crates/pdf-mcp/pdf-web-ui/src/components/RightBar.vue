<template>
  <aside class="app-rightbar" :class="{ collapsed: collapsed }">
    <div class="rb-section">页面目录</div>
    <div id="toc-list">
      <span v-if="!headings.length" style="padding:6px 14px;color:var(--text-muted);font-size:0.6875rem;">选择条目后显示</span>
      <a
        v-for="(h, idx) in headings"
        :key="idx"
        class="toc-item"
        :style="{ paddingLeft: `calc(14px + ${h.level} * 12px)` }"
        @click="scrollToHeading(idx)"
      >{{ h.text }}</a>
    </div>
    <div class="rb-divider"></div>
    <div class="rb-section">概念图谱</div>
    <div class="graph-preview">
      <div v-if="mermaidPreview" class="graph-mini">{{ mermaidPreview }}</div>
      <span v-else style="color:var(--text-muted);">选择条目后显示</span>
    </div>
  </aside>
</template>

<script setup>
import { computed } from 'vue'
import { useWikiStore } from '@/stores/wiki'

defineProps({ collapsed: Boolean })

const wikiStore = useWikiStore()

const headings = computed(() => {
  const entry = wikiStore.currentEntry
  if (!entry || !entry.body_html) return []
  const div = document.createElement('div')
  div.innerHTML = entry.body_html
  const hs = div.querySelectorAll('h1, h2, h3, h4')
  return Array.from(hs).map(el => ({
    level: parseInt(el.tagName[1]) - 1,
    text: el.textContent.trim().slice(0, 40),
  }))
})

// Simple Mermaid preview from body content
const mermaidPreview = computed(() => {
  const entry = wikiStore.currentEntry
  if (!entry || !entry.body_html) return ''
  // Extract first mermaid code block
  const m = entry.body_html.match(/<pre><code class="language-mermaid">([\s\S]*?)<\/code><\/pre>/)
  if (m) {
    const lines = m[1].split('\n').slice(0, 8)
    return lines.join('\n')
  }
  return ''
})

function scrollToHeading(idx) {
  const prose = document.querySelector('.prose')
  if (!prose) return
  const headings = prose.querySelectorAll('h1, h2, h3, h4')
  if (headings[idx]) {
    headings[idx].scrollIntoView({ behavior: 'smooth', block: 'start' })
  }
}
</script>
