<template>
  <Transition name="search-slide">
    <div class="search-overlay" :class="{ open: searchStore.open }" @click.self="searchStore.close()">
      <div class="search-header">
        <div class="search-header-left">
          <Search :size="16" class="search-icon" />
          <input
            type="text"
            v-model="searchStore.query"
            class="search-input"
            placeholder="搜索知识库…"
            @input="onSearch"
            ref="searchInputRef"
          />
          <button v-if="searchStore.query" class="search-clear-btn" @click="clearSearch">
            <X :size="14" />
          </button>
        </div>
        <div class="search-header-right">
          <div class="search-mode-toggle">
            <button
              type="button"
              class="search-facet-btn"
              :class="{ active: searchStore.searchMode === 'hybrid' }"
              @click="setMode('hybrid')"
            >
              混合
            </button>
            <button
              type="button"
              class="search-facet-btn"
              :class="{ active: searchStore.searchMode === 'keyword' }"
              @click="setMode('keyword')"
            >
              关键词
            </button>
          </div>
          <span class="sh-hint">
            <span class="kbd">↑↓</span> 导航
            <span class="kbd">Enter</span> 打开
            <span class="kbd">Esc</span> 关闭
          </span>
          <button class="search-close-btn" @click="searchStore.close()">
            <X :size="16" />
          </button>
        </div>
      </div>

      <div v-if="searchStore.domainFacets.length > 1" class="search-domain-bar">
        <span class="search-facet-label">结果筛选</span>
        <button
          class="search-facet-btn"
          :class="{ active: !wikiStore.activeDomain }"
          @click="searchStore.setDomainFilter(null)"
        >
          全部 <span class="facet-count">{{ totalFacetCount }}</span>
        </button>
        <button
          v-for="f in searchStore.domainFacets"
          :key="f.domain"
          class="search-facet-btn"
          :class="{ active: wikiStore.activeDomain === f.domain }"
          @click="searchStore.setDomainFilter(f.domain)"
        >
          {{ f.domain }} <span class="facet-count">{{ f.count }}</span>
        </button>
      </div>

      <div class="search-results-inner">
        <div v-if="searchStore.loading" class="search-loading">
          <span class="dots-loading"></span>
          <span>正在检索…</span>
        </div>

        <template v-else-if="searchStore.results.length > 0">
          <div class="search-results-header">
            <span class="results-count">{{ searchStore.results.length }} 个结果</span>
          </div>
          <div
            v-for="(r, i) in searchStore.results"
            :key="r.path"
            class="search-hit"
            :class="{ active: searchStore.selectedIdx === i }"
            @click="openResult(r)"
          >
            <div class="hit-top">
              <div class="hit-title-row">
                <FileText :size="14" class="hit-icon" />
                <span class="hit-title">{{ r.title }}</span>
                <span class="score-badge" :class="scoreTier(r.score).cls">
                  {{ scoreTier(r.score).label }}
                </span>
              </div>
              <div class="hit-meta">
                <span class="hit-domain">
                  <FolderOpen :size="11" />
                  {{ r.domain }}
                </span>
                <span v-if="r.match_count" class="hit-matches">
                  <Highlighter :size="11" />
                  {{ r.match_count }} 处匹配
                </span>
              </div>
            </div>
            <div class="hit-snippet" v-html="r.snippet || '...'"></div>
          </div>
        </template>

        <div v-else-if="searchStore.query && !searchStore.loading" class="search-empty">
          <div class="search-empty-icon">
            <SearchX :size="48" />
          </div>
          <div class="search-empty-title">无匹配结果</div>
          <div class="search-empty-desc">尝试使用更短或更通用的关键词</div>
        </div>

        <div v-else-if="!searchStore.query" class="search-hint-state">
          <div class="search-hint-icon">
            <Search :size="32" />
          </div>
          <div class="search-hint-text">输入关键词开始搜索</div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup>
import { ref, computed, watch, nextTick } from 'vue'
import { useSearchStore } from '@/stores/search'
import { useWikiStore } from '@/stores/wiki'
import { openEntry } from '@/composables/useWikiNavigation'
import { Search, X, FileText, FolderOpen, Highlighter, SearchX } from 'lucide-vue-next'

const searchStore = useSearchStore()
const wikiStore = useWikiStore()
const searchInputRef = ref(null)

const totalFacetCount = computed(() => {
  return searchStore.domainFacets.reduce((s, f) => s + f.count, 0)
})

function onSearch() {
  searchStore.triggerSearch(searchStore.query)
}

function clearSearch() {
  searchStore.query = ''
  searchStore.triggerSearch('')
}

function setMode(mode) {
  searchStore.searchMode = mode
  if (searchStore.query.trim().length >= 2) {
    searchStore.triggerSearch(searchStore.query)
  }
}

watch(() => searchStore.open, async (val) => {
  if (val) {
    await nextTick()
    searchInputRef.value?.focus()
  }
})

function scoreTier(score) {
  if (score >= 0.03) return { label: '极高', cls: 'score-extreme' }
  if (score >= 0.02) return { label: '高', cls: 'score-high' }
  if (score >= 0.01) return { label: '中', cls: 'score-med' }
  if (score >= 20) return { label: '极高', cls: 'score-extreme' }
  if (score >= 5) return { label: '高', cls: 'score-high' }
  return { label: '低', cls: 'score-low' }
}

function openResult(r) {
  searchStore.close()
  openEntry(r.path)
}
</script>

<style scoped>
.search-mode-toggle {
  display: flex;
  gap: 4px;
  margin-right: 8px;
}
.search-header-right {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}
</style>
