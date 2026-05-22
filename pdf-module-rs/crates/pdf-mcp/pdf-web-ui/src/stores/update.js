import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '@/api'

export const useUpdateStore = defineStore('update', () => {
  const currentVersion = ref(null)
  const latestVersion = ref(null)
  const updateAvailable = ref(false)
  const checking = ref(false)
  const downloading = ref(false)
  const downloadProgress = ref(0)
  const releaseNotes = ref('')
  const releaseUrl = ref('')
  const error = ref('')
  const checkedAt = ref(null)
  const prepareStatus = ref(null)
  const prepareMessage = ref('')

  async function fetchVersion() {
    try {
      currentVersion.value = await api.getVersion()
    } catch (e) {
      console.error('Failed to fetch version:', e)
    }
  }

  async function checkForUpdates() {
    checking.value = true
    error.value = ''
    try {
      const result = await api.checkUpdate()
      currentVersion.value = result.current_version
      updateAvailable.value = result.update_available
      checkedAt.value = result.checked_at
      if (result.latest_release) {
        latestVersion.value = result.latest_release.tag_name
        releaseNotes.value = result.latest_release.body
        releaseUrl.value = result.latest_release.html_url
      } else {
        latestVersion.value = null
        releaseNotes.value = ''
        releaseUrl.value = ''
      }
      return result
    } catch (e) {
      error.value = e.message || 'Update check failed'
      throw e
    } finally {
      checking.value = false
    }
  }

  async function prepareUpdate() {
    downloading.value = true
    downloadProgress.value = 0
    prepareStatus.value = 'downloading'
    error.value = ''
    try {
      const result = await api.prepareUpdate()
      prepareStatus.value = result.status
      prepareMessage.value = result.message
      return result
    } catch (e) {
      prepareStatus.value = 'error'
      prepareMessage.value = e.message || 'Update preparation failed'
      throw e
    } finally {
      downloading.value = false
      downloadProgress.value = 100
    }
  }

  function reset() {
    checking.value = false
    downloading.value = false
    downloadProgress.value = 0
    error.value = ''
    prepareStatus.value = null
    prepareMessage.value = ''
    updateAvailable.value = false
    latestVersion.value = null
    releaseNotes.value = ''
    releaseUrl.value = ''
  }

  return {
    currentVersion,
    latestVersion,
    updateAvailable,
    checking,
    downloading,
    downloadProgress,
    releaseNotes,
    releaseUrl,
    error,
    checkedAt,
    prepareStatus,
    prepareMessage,
    fetchVersion,
    checkForUpdates,
    prepareUpdate,
    reset,
  }
})
