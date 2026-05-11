<template>
  <div class="search-overlay" :class="{ open: searchStore.open }" @click.self="searchStore.close()">
    <div class="search-header">
      <span>
        <span class="sh-count">{{ resultSummary }}</span>
        <span class="sh-hint">↑↓ 导航 · Enter 打开 · Esc 关闭</span>
      </span>
      <button class="search-close-btn" @click="searchStore.close()">&times;</button>
    </div>
    <div v-if="searchStore.domainFacets.length > 1" class="search-domain-chips">
      <span class="sd-label">领域筛选</span>
      <span
        class="search-domain-chip"
        :class="{ active: !searchStore.domainFilter }"
        @click="searchStore.setDomainFilter(null)"
      >全部 ({{ totalFacetCount }})</span>
      <span
        v-for="f in searchStore.domainFacets"
        :key="f.domain"
        class="search-domain-chip"
        :class="{ active: searchStore.domainFilter === f.domain }"
        @click="searchStore.setDomainFilter(f.domain)"
      >{{ f.domain }}<span class="sdc-count">{{ f.count }}</span></span>
    </div>
    <div class="search-results-inner">
      <div v-if="searchStore.loading" class="search-spinner">
        <span class="spin-icon"></span>正在检索…
      </div>
      <template v-else-if="searchStore.results.length > 0">
        <div
          v-for="(r, i) in searchStore.results"
          :key="r.path"
          class="search-hit"
          :class="{ active: searchStore.selectedIdx === i }"
          @click="openResult(r)"
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
      </template>
      <div v-else-if="!searchStore.loading && searchStore.query" class="search-empty">
        <div class="icon">🔍</div>
        <h3>无匹配结果</h3>
        <p>尝试使用更短或更通用的关键词</p>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useSearchStore } from '@/stores/search'
import { useWikiStore } from '@/stores/wiki'

const searchStore = useSearchStore()
const wikiStore = useWikiStore()

const totalFacetCount = computed(() => {
  return searchStore.domainFacets.reduce((s, f) => s + f.count, 0)
})

const resultSummary = computed(() => {
  const total = searchStore.results.length
  if (searchStore.loading) return '搜索中…'
  if (!total) return ''
  let s = `找到 ${total} 条结果`
  if (searchStore.domainFacets.length > 1) {
    s += ' (' + searchStore.domainFacets.map(f => `${f.domain}: ${f.count}`).join(', ') + ')'
  }
  return s
})

function scoreTier(score) {
  if (score >= 20) return { label: '极高', cls: 'score-extreme' }
  if (score >= 5) return { label: '高', cls: 'score-high' }
  if (score >= 1) return { label: '中', cls: 'score-med' }
  return { label: '低', cls: 'score-low' }
}

function openResult(r) {
  searchStore.close()
  wikiStore.navigateTo(r.path)
}
</script>
