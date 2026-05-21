<template>
  <Transition name="search-slide">
    <div class="search-overlay" :class="{ open: searchStore.open }" @click.self="searchStore.close()">
      <div class="search-toolbar">
        <div class="search-toolbar-left">
          <span class="results-count">{{ searchStore.results.length }} 个结果</span>
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
            <button
              type="button"
              class="search-facet-btn"
              :class="{ active: searchStore.searchMode === 'wiki_first' }"
              @click="setMode('wiki_first')"
            >
              索引
            </button>
          </div>
        </div>
        <div class="search-toolbar-right">
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

        <div v-else-if="searchStore.error" class="search-empty">
          <div class="search-empty-icon">
            <SearchX :size="48" />
          </div>
          <div class="search-empty-title">搜索出错</div>
          <div class="search-empty-desc">{{ searchStore.error }}</div>
        </div>

        <div v-else-if="searchStore.query && !searchStore.loading" class="search-empty">
          <div class="search-empty-icon">
            <SearchX :size="48" />
          </div>
          <div class="search-empty-title">无匹配结果</div>
          <div class="search-empty-desc">尝试使用更短或更通用的关键词</div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup>
import { computed } from 'vue'
import { useSearchStore } from '@/stores/search'
import { useWikiStore } from '@/stores/wiki'
import { openEntry } from '@/composables/useWikiNavigation'
import { X, FileText, FolderOpen, Highlighter, SearchX } from 'lucide-vue-next'

const searchStore = useSearchStore()
const wikiStore = useWikiStore()

const totalFacetCount = computed(() => {
  return searchStore.domainFacets.reduce((s, f) => s + f.count, 0)
})

function setMode(mode) {
  searchStore.searchMode = mode
  if (searchStore.query.trim().length >= 2) {
    searchStore.triggerSearch(searchStore.query)
  }
}

function scoreTier(score) {
  if (score >= 0.03) return { label: '极高', cls: 'score-extreme' }
  if (score >= 0.02) return { label: '高', cls: 'score-high' }
  if (score >= 0.01) return { label: '中', cls: 'score-med' }
  return { label: '低', cls: 'score-low' }
}

function openResult(r) {
  searchStore.close()
  openEntry(r.path)
}
</script>

<style scoped>
.search-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-xs) var(--space-xl);
  border-bottom: 1px solid var(--border);
  background: var(--surface);
  flex-shrink: 0;
  min-height: 36px;
}

.search-toolbar-left {
  display: flex;
  align-items: center;
  gap: var(--space-md);
}

.search-toolbar-right {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.search-mode-toggle {
  display: flex;
  gap: 4px;
}

.sh-hint {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.6875rem;
  color: var(--text-muted);
}

.search-close-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  padding: 4px;
  display: flex;
  align-items: center;
  border-radius: var(--radius-sm);
  transition: color var(--transition-fast), background var(--transition-fast);
}

.search-close-btn:hover {
  color: var(--text);
  background: var(--surface-hover);
}
</style>
