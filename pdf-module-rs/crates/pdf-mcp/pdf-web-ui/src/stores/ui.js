import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useUiStore = defineStore('ui', () => {
  const statsOpen = ref(false)
  const domainOpen = ref(false)
  const graphOpen = ref(false)
  const settingsOpen = ref(false)
  const sidebarCollapsed = ref(false)
  const rightbarCollapsed = ref(false)
  const mobileNav = ref('browse')

  return {
    statsOpen,
    domainOpen,
    graphOpen,
    settingsOpen,
    sidebarCollapsed,
    rightbarCollapsed,
    mobileNav,
  }
})
