import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '@/api'

export const useConfigStore = defineStore('config', () => {
  const configData = ref({})
  const loading = ref(false)
  const saving = ref(false)
  const healthData = ref(null)
  const compileStatus = ref(null)
  const serverInfo = ref(null)

  async function loadConfig() {
    loading.value = true
    try {
      configData.value = await api.getConfig()
    } catch (e) {
      console.error('Failed to load config:', e)
    } finally {
      loading.value = false
    }
  }

  async function loadHealth() {
    try {
      healthData.value = await api.getHealth()
    } catch (e) {
      console.error('Failed to load health:', e)
    }
  }

  async function loadServerInfo() {
    try {
      serverInfo.value = await api.getServerInfo()
    } catch (e) {
      console.error('Failed to load server info:', e)
      serverInfo.value = null
    }
  }

  async function loadCompileStatus() {
    try {
      compileStatus.value = await api.getCompileStatus()
    } catch (e) {
      console.error('Failed to load compile status:', e)
    }
  }

  async function updateConfig(key, value) {
    saving.value = true
    const prev = configData.value[key]
    configData.value = { ...configData.value, [key]: value }
    try {
      await api.setConfig(key, value)
    } catch (e) {
      configData.value = { ...configData.value, [key]: prev }
      throw e
    } finally {
      saving.value = false
    }
  }

  async function deleteConfig(key) {
    saving.value = true
    const prev = { ...configData.value }
    const newData = { ...configData.value }
    delete newData[key]
    configData.value = newData
    try {
      await api.removeConfig(key)
    } catch (e) {
      configData.value = prev
      throw e
    } finally {
      saving.value = false
    }
  }

  async function triggerRebuild() {
    saving.value = true
    try {
      const result = await api.rebuildIndex()
      return result
    } finally {
      saving.value = false
    }
  }

  return {
    configData,
    loading,
    saving,
    healthData,
    compileStatus,
    serverInfo,
    loadConfig,
    loadHealth,
    loadServerInfo,
    loadCompileStatus,
    updateConfig,
    deleteConfig,
    triggerRebuild,
  }
})