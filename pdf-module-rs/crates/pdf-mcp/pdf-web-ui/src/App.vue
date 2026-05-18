<template>
  <div
    class="app-layout"
    :class="layoutClasses"
    ref="layoutRef"
  >
    <AppHeader
      ref="headerRef"
      :sidebar-collapsed="uiStore.sidebarCollapsed"
      :rightbar-collapsed="uiStore.rightbarCollapsed"
      @toggle-sidebar="uiStore.sidebarCollapsed = !uiStore.sidebarCollapsed"
      @toggle-rightbar="uiStore.rightbarCollapsed = !uiStore.rightbarCollapsed"
      @open-domains="uiStore.domainOpen = true"
      @open-stats="uiStore.statsOpen = true"
      @open-graph="uiStore.graphOpen = true"
      @open-settings="uiStore.settingsOpen = true"
      @open-compile="compileStore.openDrawer()"
    />

    <AppSidebar :collapsed="uiStore.sidebarCollapsed" />

    <main class="app-main" ref="mainRef">
      <router-view v-slot="{ Component, route }">
        <transition name="page" mode="out-in">
          <component :is="Component" :key="route.fullPath" />
        </transition>
      </router-view>
    </main>

    <RightBar
      :collapsed="uiStore.rightbarCollapsed"
      :main-scroll-el="mainRef"
    />

    <SearchOverlay />

    <StatsDialog :open="uiStore.statsOpen" @close="uiStore.statsOpen = false" />
    <DomainDialog :open="uiStore.domainOpen" @close="uiStore.domainOpen = false" />
    <GraphDialog v-if="uiStore.graphOpen" :open="uiStore.graphOpen" @close="uiStore.graphOpen = false" />
    <SettingsModal :open="uiStore.settingsOpen" @close="uiStore.settingsOpen = false" />
    <CompileDrawer />

    <button v-if="isMcpMode && wikiStore.currentEntry && !wikiStore.currentEntry.error" class="mcp-ask-btn" @click="askAi">
      向 AI 提问此条目
    </button>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, watch, defineAsyncComponent } from 'vue'
import { useWikiStore } from '@/stores/wiki'
import { useUiStore } from '@/stores/ui'
import { useCompileStore } from '@/stores/compile'
import { useWorkspaceStore } from '@/stores/workspace'
import { setActiveKbId } from '@/api'
import { useKeyboard } from '@/composables/useKeyboard'
import { markdownExcerpt } from '@/utils/path'
import {
  MCP_UI_MESSAGE_TYPES,
  MCP_UI_PROTOCOL_VERSION,
  MCP_UI_SOURCE_WIKI_BROWSER,
  mcpTargetOrigin as resolveMcpOrigin,
} from '@/mcp/protocol'
import AppHeader from '@/components/AppHeader.vue'
import AppSidebar from '@/components/AppSidebar.vue'
import RightBar from '@/components/RightBar.vue'
import SearchOverlay from '@/components/SearchOverlay.vue'
import StatsDialog from '@/components/StatsDialog.vue'
import DomainDialog from '@/components/DomainDialog.vue'
import SettingsModal from '@/components/SettingsModal.vue'
import CompileDrawer from '@/components/CompileDrawer.vue'

const GraphDialog = defineAsyncComponent(() => import('@/components/GraphDialog.vue'))

const wikiStore = useWikiStore()
const uiStore = useUiStore()
const compileStore = useCompileStore()
const workspaceStore = useWorkspaceStore()

const isMcpMode = ref(false)
const mainRef = ref(null)
const headerRef = ref(null)

const layoutClasses = computed(() => ({
  'left-collapsed': uiStore.sidebarCollapsed || wikiStore.readingMode,
  'right-collapsed': uiStore.rightbarCollapsed || wikiStore.readingMode,
  'reading-mode': wikiStore.readingMode,
  'mcp-mode': isMcpMode.value,
}))

useKeyboard(() => headerRef.value?.searchInputRef)

watch(
  () => wikiStore.readingMode,
  (on) => {
    if (on) {
      uiStore.sidebarCollapsed = true
      uiStore.rightbarCollapsed = true
    }
  },
)

onMounted(async () => {
  wikiStore.initTheme()
  await workspaceStore.fetchWorkspaces()
  if (workspaceStore.activeKbId) {
    setActiveKbId(workspaceStore.activeKbId)
  }
  wikiStore.loadTree()
  isMcpMode.value = window.parent !== window
})

function askAi() {
  const entry = wikiStore.currentEntry
  if (!entry || entry.error) return

  const payload = {
    v: MCP_UI_PROTOCOL_VERSION,
    type: MCP_UI_MESSAGE_TYPES.ASK_AI,
    source: MCP_UI_SOURCE_WIKI_BROWSER,
    title: entry.title,
    path: wikiStore.currentPath,
    excerpt: markdownExcerpt(entry.body_markdown || '', 2000),
  }

  let origin = window.location.origin
  try {
    if (document.referrer) {
      origin = new URL(document.referrer).origin
    }
  } catch {
    /* ignore */
  }
  window.parent.postMessage(payload, resolveMcpOrigin(origin))
}
</script>
