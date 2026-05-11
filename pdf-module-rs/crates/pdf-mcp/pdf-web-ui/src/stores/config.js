import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '@/api'

export const useConfigStore = defineStore('config', () => {
  const configData = ref({})
  const healthData = ref(null)
  const compileStatus = ref(null)
  const loading = ref(false)
  const saving = ref(false)

  async function loadConfig() {
    try {
      const data = await api.getConfig()
      configData.value = data.config || {}
    } catch (e) {
      configData.value = {}
    }
  }

  async function loadHealth() {
    try {
      healthData.value = await api.getHealth()
    } catch (e) {
      healthData.value = null
    }
  }

  async function loadCompileStatus() {
    try {
      compileStatus.value = await api.getCompileStatus()
    } catch (e) {
      compileStatus.value = null
    }
  }

  async function updateConfig(key, value) {
    saving.value = true
    try {
      await api.setConfig(key, value)
      await loadConfig()
    } finally {
      saving.value = false
    }
  }

  async function deleteConfig(key) {
    try {
      await api.removeConfig(key)
      await loadConfig()
    } catch (e) {
      console.error('Failed to delete config', e)
    }
  }

  async function triggerRebuild() {
    loading.value = true
    try {
      return await api.rebuildIndex()
    } finally {
      loading.value = false
    }
  }

  return {
    configData,
    healthData,
    compileStatus,
    loading,
    saving,
    loadConfig,
    loadHealth,
    loadCompileStatus,
    updateConfig,
    deleteConfig,
    triggerRebuild,
  }
})
