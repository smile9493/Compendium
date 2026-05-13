<template>
  <div class="app-layout" :class="{ 'left-collapsed': sidebarCollapsed, 'right-collapsed': rightbarCollapsed, 'mcp-mode': isMcpMode }" ref="layoutRef">
    <AppHeader
      :sidebar-collapsed="sidebarCollapsed"
      :rightbar-collapsed="rightbarCollapsed"
      @toggle-sidebar="sidebarCollapsed = !sidebarCollapsed"
      @toggle-rightbar="rightbarCollapsed = !rightbarCollapsed"
      @open-domains="domainOpen = true"
      @open-stats="statsOpen = true"
      @open-graph="graphOpen = true"
      @open-settings="settingsOpen = true"
    />

    <AppSidebar :collapsed="sidebarCollapsed" />

    <main class="app-main">
      <router-view />
    </main>

    <RightBar :collapsed="rightbarCollapsed" />

    <SearchOverlay />

    <StatsDialog :open="statsOpen" @close="statsOpen = false" />
    <DomainDialog :open="domainOpen" @close="domainOpen = false" />
    <GraphDialog :open="graphOpen" @close="graphOpen = false" />
    <SettingsModal :open="settingsOpen" @close="settingsOpen = false" />

    <button v-if="isMcpMode && wikiStore.currentEntry" class="mcp-ask-btn" @click="askAi">
      💬 向 AI 提问此条目
    </button>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useWikiStore } from '@/stores/wiki'
import AppHeader from '@/components/AppHeader.vue'
import AppSidebar from '@/components/AppSidebar.vue'
import RightBar from '@/components/RightBar.vue'
import SearchOverlay from '@/components/SearchOverlay.vue'
import StatsDialog from '@/components/StatsDialog.vue'
import DomainDialog from '@/components/DomainDialog.vue'
import GraphDialog from '@/components/GraphDialog.vue'
import SettingsModal from '@/components/SettingsModal.vue'

const wikiStore = useWikiStore()

const sidebarCollapsed = ref(false)
const rightbarCollapsed = ref(false)
const statsOpen = ref(false)
const domainOpen = ref(false)
const graphOpen = ref(false)
const settingsOpen = ref(false)
const isMcpMode = ref(false)

onMounted(() => {
  wikiStore.initTheme()
  wikiStore.loadTree()

  // Check if running inside MCP iframe
  isMcpMode.value = window.parent !== window

  // Close search when clicking outside
  document.addEventListener('click', (e) => {
    const searchStore = useSearchStore()
    if (searchStore.open && !e.target.closest('.search-bar-wrap') && !e.target.closest('.search-overlay')) {
      searchStore.close()
    }
  })
})

function askAi() {
  if (!wikiStore.currentEntry) return
  window.parent.postMessage({
    type: 'mcp-ask-ai',
    source: 'wiki-browser',
    title: wikiStore.currentEntry.title,
    path: wikiStore.currentPath,
    body: wikiStore.currentEntry.body_html || wikiStore.currentEntry.body || '',
  }, '*')
}
</script>
