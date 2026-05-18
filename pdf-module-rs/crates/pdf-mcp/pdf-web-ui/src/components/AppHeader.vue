<template>
  <div class="app-header">
    <div class="header-left">
      <button class="header-btn icon-btn" @click="$emit('toggleSidebar')" v-tooltip="sidebarCollapsed ? '展开目录栏' : '折叠目录栏'">
        <Menu v-if="!sidebarCollapsed" :size="16" />
        <PanelLeft v-else :size="16" />
      </button>
      <span class="logo">
        <BookOpen :size="15" />
        rsut-pdf-mcp
      </span>
      <select
        v-if="workspaceStore.workspaces.length"
        class="kb-select"
        :value="workspaceStore.activeKbId ?? ''"
        @change="onKbChange"
        v-tooltip="'知识库工作区'"
      >
        <option v-for="w in workspaceStore.workspaces" :key="w.id" :value="w.id">
          {{ w.name }}
        </option>
      </select>
    </div>

    <div class="search-bar-wrap" :class="{ active: searchStore.open }">
      <Search :size="14" class="search-bar-icon" />
      <input
        ref="searchInputRef"
        type="text"
        class="search-bar"
        v-model="searchStore.query"
        @input="onSearchInput"
        @keydown="onSearchKeydown"
        placeholder="搜索知识库…"
        autocomplete="off"
      />
      <button v-if="searchStore.query" class="search-clear-btn" @click="clearSearch">
        <X :size="12" />
      </button>
      <span v-else class="search-shortcut"><span class="kbd">/</span></span>
    </div>

    <span class="header-spacer"></span>

    <div class="header-right">
      <button class="header-btn icon-btn" @click="$emit('openDomains')" v-tooltip="'所有领域'">
        <Tag :size="15" />
      </button>
      <button class="header-btn icon-btn" @click="$emit('openStats')" v-tooltip="'知识库统计'">
        <BarChart2 :size="15" />
      </button>
      <button class="header-btn icon-btn" @click="$emit('openGraph')" v-tooltip="'知识图谱'">
        <GitBranch :size="15" />
      </button>
      <button class="header-btn icon-btn" @click="$emit('openCompile')" v-tooltip="'编译控制台'">
        <Hammer :size="15" />
      </button>
      <span class="header-divider"></span>
      <button
        class="header-btn icon-btn"
        :class="{ active: wikiStore.readingMode }"
        @click="wikiStore.toggleReadingMode()"
        v-tooltip="'阅读模式'"
      >
        <BookMarked :size="15" />
      </button>
      <button class="header-btn icon-btn" @click="wikiStore.toggleTheme()" v-tooltip="'切换主题'">
        <Sun v-if="wikiStore.darkTheme" :size="15" />
        <Moon v-else :size="15" />
      </button>
      <button class="header-btn icon-btn" @click="$emit('openSettings')" v-tooltip="'设置'">
        <Settings :size="15" />
      </button>
      <button class="header-btn icon-btn" @click="$emit('toggleRightbar')" v-tooltip="rightbarCollapsed ? '展开信息栏' : '折叠信息栏'">
        <PanelRight v-if="!rightbarCollapsed" :size="16" />
        <PanelRightClose v-else :size="16" />
      </button>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { useWikiStore } from '@/stores/wiki'
import { useSearchStore } from '@/stores/search'
import { useWorkspaceStore } from '@/stores/workspace'
import { setActiveKbId } from '@/api'
import { openEntry } from '@/composables/useWikiNavigation'
import {
  Menu, PanelLeft, PanelRight, PanelRightClose, BookOpen, BookMarked,
  Tag, BarChart2, GitBranch, Hammer, Sun, Moon, Settings, Search, X,
} from 'lucide-vue-next'

defineProps({
  sidebarCollapsed: Boolean,
  rightbarCollapsed: Boolean,
})

defineEmits(['toggleSidebar', 'toggleRightbar', 'openDomains', 'openStats', 'openGraph', 'openCompile', 'openSettings'])

const wikiStore = useWikiStore()
const searchStore = useSearchStore()
const workspaceStore = useWorkspaceStore()
const searchInputRef = ref(null)

async function onKbChange(e) {
  const kbId = e.target.value
  await workspaceStore.setActive(kbId)
  setActiveKbId(kbId)
  await wikiStore.loadTree()
}

defineExpose({ searchInputRef })

function onSearchInput() {
  searchStore.triggerSearch(searchStore.query)
}

function clearSearch() {
  searchStore.query = ''
  searchStore.triggerSearch('')
  searchInputRef.value?.focus()
}

function onSearchKeydown(e) {
  if (e.key === 'Escape') {
    if (searchStore.open) {
      searchStore.close()
    } else {
      searchStore.clearAndClose()
      searchInputRef.value?.blur()
    }
    return
  }
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    if (!searchStore.open && searchStore.query.trim().length >= 2) {
      searchStore.triggerSearch(searchStore.query)
    }
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
      searchStore.close()
      openEntry(r.path)
    }
  }
}
</script>
