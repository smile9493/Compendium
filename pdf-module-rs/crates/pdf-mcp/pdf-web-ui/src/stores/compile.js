import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '@/api'
import { useWikiStore } from '@/stores/wiki'

const KNOWN_STAGES = ['extract', 'prompt_gen', 'agent_wiki', 'index_rebuild', 'quality_gate']

export const useCompileStore = defineStore('compile', () => {
  const open = ref(false)
  const activeTab = ref('trigger')
  const compileStatus = ref(null)
  const qualitySnapshot = ref(null)
  const loading = ref(false)
  const error = ref(null)
  const lastFinishedAt = ref(null)
  const currentStage = ref(null)
  const progress = ref(null)

  let pollTimer = null
  let backgroundTimer = null
  let eventSource = null
  let sseFailed = false
  let sseReconnectTimer = null
  let sseReconnectAttempts = 0
  const SSE_MAX_RECONNECT_DELAY = 30000

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

  const activeStages = computed(() => {
    const stages = compileStatus.value?.job?.stages || []
    const result = []
    for (const stageId of KNOWN_STAGES) {
      const match = stages.find(s => s.stage === stageId)
      if (match) {
        result.push({ id: stageId, status: match.status, durationMs: match.duration_ms })
      } else {
        const stageIdx = KNOWN_STAGES.indexOf(stageId)
        const activeIdx = match ? KNOWN_STAGES.indexOf(match.stage) : -1
        result.push({ id: stageId, status: stageIdx < activeIdx ? 'done' : 'pending' })
      }
    }
    return result
  })

  function handleCompileEvent(data) {
    if (!data || typeof data !== 'object') return
    if (data.job_id) {
      compileStatus.value = { ...compileStatus.value, active_job_id: data.job_id }
    }
    if (data.stage) {
      currentStage.value = data.stage
    }
    if (data.status) {
      compileStatus.value = { ...compileStatus.value, pipeline_status: data.status }
    }
    if (data.progress !== undefined) {
      progress.value = data.progress
    }
  }

  function applySnapshot(data) {
    if (!data || typeof data !== 'object') return
    compileStatus.value = data
    qualitySnapshot.value = data.quality_snapshot || null

    if (data.job?.stages) {
      const active = data.job.stages.find(s => s.status === 'running')
      currentStage.value = active?.stage || null
    }
  }

  function openDrawer(tab = 'trigger') {
    open.value = true
    activeTab.value = tab
    startRealtime()
    refreshStatus()
  }

  function closeDrawer() {
    open.value = false
    stopRealtime()
  }

  function startRealtime() {
    if (!sseFailed && typeof EventSource !== 'undefined') {
      stopPolling()
      startSSE()
      return
    }
    startPolling()
  }

  function stopRealtime() {
    stopSSE()
    stopPolling()
  }

  function startSSE() {
    stopSSE()
    try {
      const url = api.compileEventsUrl()
      eventSource = new EventSource(url)

      eventSource.addEventListener('compile-status', (ev) => {
        try {
          const data = JSON.parse(ev.data)
          handleCompileEvent(data)
          applySnapshot(data)
          handleCompileFinished(data)
          sseReconnectAttempts = 0
        } catch (e) {
          console.warn('SSE parse error:', e)
        }
      })

      eventSource.onerror = () => {
        stopSSE()
        sseReconnectAttempts++
        const delay = Math.min(1000 * Math.pow(2, sseReconnectAttempts), SSE_MAX_RECONNECT_DELAY)
        if (open.value) {
          sseReconnectTimer = setTimeout(() => {
            if (open.value) startSSE()
          }, delay)
        } else {
          sseFailed = true
          startPolling()
        }
      }
    } catch {
      sseFailed = true
      startPolling()
    }
  }

  function stopSSE() {
    if (eventSource) {
      eventSource.close()
      eventSource = null
    }
    if (sseReconnectTimer) {
      clearTimeout(sseReconnectTimer)
      sseReconnectTimer = null
    }
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

  function startBackgroundWatch() {
    stopBackgroundWatch()
    refreshStatus()
    backgroundTimer = setInterval(() => {
      if (!open.value) refreshStatus()
    }, 5000)
  }

  function stopBackgroundWatch() {
    if (backgroundTimer) {
      clearInterval(backgroundTimer)
      backgroundTimer = null
    }
  }

  async function handleCompileFinished(data) {
    const prevFinished = lastFinishedAt.value
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
  }

  async function refreshStatus() {
    try {
      const data = await api.getCompileStatus()
      applySnapshot(data)
      await handleCompileFinished(data)
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
    currentStage,
    progress,
    activeStages,
    applySnapshot,
    handleCompileEvent,
    openDrawer,
    closeDrawer,
    refreshStatus,
    startBackgroundWatch,
    stopBackgroundWatch,
    uploadAndCompile,
    triggerIncremental,
    startPolling,
    stopPolling,
    startSSE,
    stopSSE,
    startRealtime,
    stopRealtime,
  }
})
