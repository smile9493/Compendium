<template>
  <div class="app-header">
    <div class="header-left">
      <button class="header-btn icon-btn" @click="$emit('toggleSidebar')" v-tooltip="sidebarCollapsed ? t('header.expandSidebar') : t('header.collapseSidebar')">
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
        v-tooltip="t('header.kbWorkspace')"
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
        :placeholder="t('app.searchPlaceholder')"
        autocomplete="off"
      />
      <button v-if="searchStore.query" class="search-clear-btn" @click="clearSearch">
        <X :size="12" />
      </button>
      <span v-else class="search-shortcut"><span class="kbd">/</span></span>
    </div>

    <span class="header-spacer"></span>

    <div class="header-right">
      <button class="header-btn icon-btn" @click="$emit('openDomains')" v-tooltip="t('header.allDomains')">
        <Tag :size="15" />
      </button>
      <button class="header-btn icon-btn" @click="$emit('openStats')" v-tooltip="t('header.kbStats')">
        <BarChart2 :size="15" />
      </button>
      <button class="header-btn icon-btn" @click="$emit('openGraph')" v-tooltip="t('header.knowledgeGraph')">
        <GitBranch :size="15" />
      </button>
      <button
        class="header-btn icon-btn compile-header-btn"
        :class="{ 'compile-active': compileStore.isRunning }"
        :aria-label="t('compile.tabTrigger')"
        @click="onOpenCompile"
        v-tooltip="compileHeaderTooltip"
      >
        <Hammer :size="15" />
        <span v-if="compileStore.isRunning" class="compile-status-dot" :class="compileDotClass" />
      </button>
      <span class="header-divider"></span>
      <button
        class="header-btn icon-btn"
        :class="{ active: wikiStore.readingMode }"
        @click="wikiStore.toggleReadingMode()"
        v-tooltip="t('header.readingMode')"
      >
        <BookMarked :size="15" />
      </button>
      <button class="header-btn icon-btn" @click="wikiStore.toggleTheme()" v-tooltip="t('header.toggleTheme')">
        <Sun v-if="wikiStore.darkTheme" :size="15" />
        <Moon v-else :size="15" />
      </button>
      <button class="header-btn icon-btn" @click="$emit('openSettings')" v-tooltip="t('header.settings')">
        <Settings :size="15" />
      </button>
      <button class="header-btn icon-btn" @click="$emit('toggleRightbar')" v-tooltip="rightbarCollapsed ? t('header.expandRightbar') : t('header.collapseRightbar')">
        <PanelRight v-if="!rightbarCollapsed" :size="16" />
        <PanelRightClose v-else :size="16" />
      </button>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { useWikiStore } from '@/stores/wiki'
import { useSearchStore } from '@/stores/search'
import { useWorkspaceStore } from '@/stores/workspace'
import { useCompileStore } from '@/stores/compile'
import { useI18n } from 'vue-i18n'
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
const compileStore = useCompileStore()
const { t } = useI18n()
const searchInputRef = ref(null)

const compileHeaderTooltip = computed(() => {
  if (!compileStore.isRunning) return t('compile.console')
  const statusKey =
    compileStore.pipelineStatus === 'awaiting_agent'
      ? 'compile.statusAwaiting'
      : compileStore.pipelineStatus === 'running'
        ? 'compile.statusRunning'
        : 'compile.statusIdle'
  return `${t('compile.console')}: ${t(statusKey)}`
})

const compileDotClass = computed(() =>
  compileStore.pipelineStatus === 'awaiting_agent' ? 'dot-warn' : 'dot-run'
)

function onOpenCompile() {
  const tab = compileStore.isRunning ? 'status' : 'trigger'
  compileStore.openDrawer(tab)
}

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
