<template>
  <div class="app-header">
    <button class="header-btn" @click="$emit('toggleSidebar')" :title="sidebarCollapsed ? '展开目录栏' : '折叠目录栏'">
      {{ sidebarCollapsed ? '▶' : '◀' }}
    </button>
    <span class="logo">📚 rsut-pdf-mcp</span>
    <div class="search-bar-wrap">
      <input
        ref="searchInput"
        type="text"
        class="search-bar"
        :value="searchQuery"
        @input="onSearchInput"
        @keydown="onSearchKeydown"
        placeholder="搜索知识库… (按 / 聚焦)"
        autocomplete="off"
      />
      <span class="search-shortcut">/</span>
    </div>
    <span class="header-spacer"></span>
    <button class="header-btn" @click="$emit('openDomains')" title="所有领域">🏷️ 领域</button>
    <button class="header-btn" @click="$emit('openStats')" title="知识库统计">📊 统计</button>
    <button class="header-btn" @click="$emit('openGraph')" title="知识图谱">🔗 图谱</button>
    <button class="header-btn" @click="wikiStore.toggleTheme()" title="切换主题">🌓</button>
    <button class="header-btn" @click="$emit('openSettings')" title="设置">⚙️</button>
    <button class="header-btn" @click="$emit('toggleRightbar')" :title="rightbarCollapsed ? '展开信息栏' : '折叠信息栏'">
      {{ rightbarCollapsed ? '◀' : '▶' }}
    </button>
  </div>
</template>

<script setup>
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { useWikiStore } from '@/stores/wiki'
import { useSearchStore } from '@/stores/search'

defineProps({
  sidebarCollapsed: Boolean,
  rightbarCollapsed: Boolean,
})

defineEmits(['toggleSidebar', 'toggleRightbar', 'openDomains', 'openStats', 'openGraph', 'openSettings'])

const wikiStore = useWikiStore()
const searchStore = useSearchStore()
const searchQuery = ref('')
const searchInput = ref(null)

function onSearchInput(e) {
  searchQuery.value = e.target.value
  searchStore.triggerSearch(e.target.value)
}

function onSearchKeydown(e) {
  if (e.key === 'Escape') {
    searchQuery.value = ''
    searchStore.close()
    searchInput.value?.blur()
    return
  }
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    searchStore.selectNext()
  }
  if (e.key === 'ArrowUp') {
    e.preventDefault()
    searchStore.selectPrev()
  }
  if (e.key === 'Enter' && searchStore.selectedIdx >= 0) {
    e.preventDefault()
    const r = searchStore.results[searchStore.selectedIdx]
    if (r) {
      searchQuery.value = ''
      searchStore.close()
      wikiStore.navigateTo(r.path)
    }
  }
}

function onGlobalKeydown(e) {
  if (e.key === '/' && !e.ctrlKey && !e.metaKey && document.activeElement !== searchInput.value) {
    e.preventDefault()
    searchInput.value?.focus()
    searchInput.value?.select()
  }
}

onMounted(() => {
  document.addEventListener('keydown', onGlobalKeydown)
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', onGlobalKeydown)
})
</script>
