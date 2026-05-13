<template>
  <div class="content-inner">
    <div v-if="wikiStore.currentEntry">
      <div v-if="wikiStore.currentEntry.error" class="empty-entry">
        <div class="icon">⚠️</div>
        <h2>加载失败</h2>
        <p>{{ wikiStore.currentEntry.error }}</p>
      </div>
      <template v-else>
        <div class="breadcrumb">
          <span @click="$router.push('/')">📚 首页</span>
          <template v-for="(part, i) in pathParts" :key="i">
            <span class="sep">/</span>
            <span @click="navigateToBreadcrumb(i)" :style="{ cursor: isLast(i) ? 'default' : 'pointer' }">
              {{ part }}
            </span>
          </template>
        </div>

        <div class="entry-card">
          <h1>{{ wikiStore.currentEntry.title || 'Untitled' }}</h1>
          <div class="meta-row">
            <span class="badge badge-domain">{{ wikiStore.currentEntry.domain || '?' }}</span>
            <span class="badge badge-level">{{ wikiStore.currentEntry.level || 'L1' }}</span>
            <span
              class="badge badge-status"
              :class="{ needs_recompile: wikiStore.currentEntry.status === 'needs_recompile' }"
            >{{ wikiStore.currentEntry.status || '?' }}</span>
            <span class="badge badge-quality">质量 {{ qualityScore }}</span>
            <span v-if="wikiStore.currentEntry.version" class="badge badge-version">v{{ wikiStore.currentEntry.version }}</span>
            <span v-if="wikiStore.currentEntry.source" class="badge badge-source">源: {{ wikiStore.currentEntry.source }}</span>
          </div>
          <div v-if="hasTags" class="tag-list">
            <span v-for="tag in wikiStore.currentEntry.tags" :key="tag" class="tag-item">{{ tag }}</span>
          </div>
        </div>

        <MarkdownRenderer :markdown="wikiStore.currentEntry.body_markdown || wikiStore.currentEntry.body || ''" />

        <div v-if="hasRelations" class="relations">
          <div v-if="wikiStore.currentEntry.related?.length" class="relation-panel">
            <h3>🔗 相关条目</h3>
            <div class="link-grid">
              <span
                v-for="r in wikiStore.currentEntry.related"
                :key="r"
                class="link-card"
                @click="wikiStore.navigateTo(r)"
              >{{ r }}</span>
            </div>
          </div>
          <div v-if="wikiStore.currentEntry.contradictions?.length" class="relation-panel">
            <h3>⚡ 矛盾条目</h3>
            <div class="link-grid">
              <span
                v-for="r in wikiStore.currentEntry.contradictions"
                :key="r"
                class="link-card contradiction"
                @click="wikiStore.navigateTo(r)"
              >{{ r }}</span>
            </div>
          </div>
          <div v-if="wikiStore.currentEntry.backlinks?.length" class="relation-panel">
            <h3>↩️ 反向链接</h3>
            <div class="link-grid">
              <span
                v-for="r in wikiStore.currentEntry.backlinks"
                :key="r"
                class="link-card"
                @click="wikiStore.navigateTo(r)"
              >{{ r }}</span>
            </div>
          </div>
        </div>
      </template>
    </div>
    <div v-else class="loading-placeholder">
      <span class="spin-icon"></span>加载中…
    </div>
  </div>
</template>

<script setup>
import { computed, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useWikiStore } from '@/stores/wiki'
import MarkdownRenderer from '@/components/MarkdownRenderer.vue'

const props = defineProps({ path: { type: String, default: '' } })
const route = useRoute()
const wikiStore = useWikiStore()

const pathParts = computed(() => {
  const p = wikiStore.currentPath || ''
  return p.replace('.md', '').split('/').filter(Boolean)
})

function isLast(i) {
  return i === pathParts.value.length - 1
}

function navigateToBreadcrumb(i) {
  const p = pathParts.value.slice(0, i + 1).join('/') + '.md'
  wikiStore.navigateTo(p)
}

const qualityScore = computed(() => {
  const q = wikiStore.currentEntry?.quality_score
  if (q == null) return '-'
  return `${(q * 100).toFixed(0)}%`
})

const hasTags = computed(() => {
  return wikiStore.currentEntry?.tags?.length > 0
})

const hasRelations = computed(() => {
  const e = wikiStore.currentEntry
  return e && (e.related?.length || e.contradictions?.length || e.backlinks?.length)
})

// Load entry when route params change
watch(() => props.path || route.params.path, async (newPath) => {
  if (newPath) {
    await wikiStore.navigateTo(newPath)
  }
}, { immediate: true })
</script>
