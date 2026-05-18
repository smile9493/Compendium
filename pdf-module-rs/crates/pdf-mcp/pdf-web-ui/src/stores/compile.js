import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '@/api'
import { useWikiStore } from '@/stores/wiki'

export const useCompileStore = defineStore('compile', () => {
  const open = ref(false)
  const activeTab = ref('trigger')
  const compileStatus = ref(null)
  const qualitySnapshot = ref(null)
  const loading = ref(false)
  const error = ref(null)
  const lastFinishedAt = ref(null)

  let pollTimer = null

  const pipelineStatus = computed(
    () => compileStatus.value?.pipeline_status || compileStatus.value?.job?.pipeline_status
  )

  const isRunning = computed(
    () =>
      compileStatus.value?.running === true ||
      pipelineStatus.value === 'running' ||
      pipelineStatus.value === 'awaiting_agent'
  )

  const statusText = computed(() => {
    if (pipelineStatus.value === 'awaiting_agent') return '等待 Agent'
    if (isRunning.value && pipelineStatus.value === 'running') return '编译中'
    if (
      pipelineStatus.value === 'completed' ||
      (!compileStatus.value?.active_job_id && compileStatus.value?.last_outcome === 'success')
    )
      return '已完成'
    if (
      pipelineStatus.value === 'partial' ||
      (!compileStatus.value?.active_job_id && compileStatus.value?.last_outcome === 'partial')
    )
      return '部分完成'
    if (pipelineStatus.value === 'failed' || compileStatus.value?.last_outcome === 'error')
      return '失败'
    return '空闲'
  })

  function openDrawer(tab = 'trigger') {
    open.value = true
    activeTab.value = tab
    startPolling()
    refreshStatus()
  }

  function closeDrawer() {
    open.value = false
    stopPolling()
  }

  function startPolling() {
    stopPolling()
    pollTimer = setInterval(() => refreshStatus(), 2000)
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer)
      pollTimer = null
    }
  }

  async function refreshStatus() {
    try {
      const data = await api.getCompileStatus()
      const prevFinished = lastFinishedAt.value
      compileStatus.value = data
      qualitySnapshot.value = data.quality_snapshot || null
      const finished = data.last_finished || null
      const done =
        data.pipeline_status === 'completed' ||
        data.pipeline_status === 'partial' ||
        (!data.active_job_id &&
          !data.running &&
          (data.last_outcome === 'success' || data.last_outcome === 'partial'))
      if (finished && finished !== prevFinished && done && !data.running) {
        const wikiStore = useWikiStore()
        await wikiStore.loadTree()
      }
      lastFinishedAt.value = finished
    } catch (e) {
      console.error('Compile status poll failed:', e)
    }
  }

  async function uploadAndCompile(file, mode = 'single') {
    loading.value = true
    error.value = null
    try {
      if (mode === 'incremental') {
        await api.triggerIncrementalCompile()
      } else {
        const upload = await api.uploadPdf(file)
        await api.compileUploaded(upload.file_id)
      }
      activeTab.value = 'status'
      await refreshStatus()
    } catch (e) {
      error.value = e.message || '编译失败'
    } finally {
      loading.value = false
    }
  }

  async function triggerIncremental() {
    loading.value = true
    error.value = null
    try {
      await api.triggerIncrementalCompile()
      activeTab.value = 'status'
      await refreshStatus()
    } catch (e) {
      error.value = e.message || '增量编译失败'
    } finally {
      loading.value = false
    }
  }

  return {
    open,
    activeTab,
    compileStatus,
    qualitySnapshot,
    loading,
    error,
    isRunning,
    pipelineStatus,
    statusText,
    openDrawer,
    closeDrawer,
    refreshStatus,
    uploadAndCompile,
    triggerIncremental,
    startPolling,
    stopPolling,
  }
})
