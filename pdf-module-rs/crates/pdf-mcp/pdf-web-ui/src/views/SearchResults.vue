<template>
  <div class="content-inner">
    <div class="breadcrumb">
      <span @click="$router.push('/')">📚 首页</span>
      <span class="sep">/</span>
      <span>搜索: {{ route.query.q || '' }}</span>
    </div>

    <div v-if="search(route.query.q)" class="search-hits-list">
      <div
        v-for="r in searchResults"
        :key="r.path"
        class="search-hit"
        @click="openEntry(r)"
      >
        <div class="hit-header">
          <span class="hit-title">{{ r.title }}</span>
          <span class="score-badge" :class="scoreTier(r.score).cls">{{ scoreTier(r.score).label }} · {{ r.score.toFixed(1) }}</span>
        </div>
        <div class="hit-meta">
          <span class="badge badge-domain" style="font-size:0.625rem;padding:1px 6px;">{{ r.domain }}</span>
          <span v-if="r.match_count" style="color:var(--text-muted);">×{{ r.match_count }} 处匹配</span>
        </div>
        <div class="hit-snippet" v-html="r.snippet || '...'"></div>
      </div>
    </div>
    <div v-else class="search-empty">
      <div class="icon">🔍</div>
      <h3>无匹配结果</h3>
      <p>尝试使用更短或更通用的关键词</p>
    </div>
  </div>
</template>

<script setup>
import { ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useWikiStore } from '@/stores/wiki'
import { api } from '@/api'

const route = useRoute()
const router = useRouter()
const wikiStore = useWikiStore()

const searchResults = ref([])

function scoreTier(score) {
  if (score >= 20) return { label: '极高', cls: 'score-extreme' }
  if (score >= 5) return { label: '高', cls: 'score-high' }
  if (score >= 1) return { label: '中', cls: 'score-med' }
  return { label: '低', cls: 'score-low' }
}

async function search(q) {
  if (!q) return false
  try {
    const data = await api.searchWiki(q)
    searchResults.value = data.results || []
    return true
  } catch (e) {
    searchResults.value = []
    return true
  }
}

function openEntry(r) {
  wikiStore.navigateTo(r.path)
}

watch(() => route.query.q, (q) => {
  if (q) search(q)
}, { immediate: true })
</script>
