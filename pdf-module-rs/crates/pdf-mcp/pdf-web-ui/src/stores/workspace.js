import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, setActiveKbId } from '@/api'

export const useWorkspaceStore = defineStore('workspace', () => {
  const workspaces = ref([])
  const activeKbId = ref(null)
  const loading = ref(false)

  async function fetchWorkspaces() {
    loading.value = true
    try {
      const data = await api.listWorkspaces()
      workspaces.value = data.workspaces ?? []
      // Auto-select: explicit active_kb_id > workspace with active flag > first workspace
      activeKbId.value = data.active_kb_id
        ?? workspaces.value.find((w) => w.active)?.id
        ?? workspaces.value[0]?.id
        ?? null
      if (activeKbId.value) {
        setActiveKbId(activeKbId.value)
      }
    } finally {
      loading.value = false
    }
  }

  async function setActive(kbId) {
    await api.setActiveWorkspace(kbId)
    activeKbId.value = kbId
    setActiveKbId(kbId)
    workspaces.value = workspaces.value.map((w) => ({
      ...w,
      active: w.id === kbId,
    }))
  }

  function kbQuery() {
    return activeKbId.value ? { kb_id: activeKbId.value } : {}
  }

  return {
    workspaces,
    activeKbId,
    loading,
    fetchWorkspaces,
    setActive,
    kbQuery,
  }
})
