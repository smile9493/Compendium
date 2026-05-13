import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '@/api'

export const useSearchStore = defineStore('search', () => {
  const query = ref('')
  const results = ref([])
  const domainFacets = ref([])
  const domainFilter = ref(null)
  const loading = ref(false)
  const selectedIdx = ref(-1)
  const open = ref(false)
  let abortController = null
  let searchTimer = null

  async function doSearch(q) {
    if (abortController) abortController.abort()
    abortController = new AbortController()
    const signal = abortController.signal

    loading.value = true
    open.value = true
    results.value = []
    selectedIdx.value = -1
    query.value = q

    try {
      const params = new URLSearchParams({ q, limit: '30' })
      if (domainFilter.value) params.set('domain', domainFilter.value)
      const response = await fetch(`/api/wiki/search?${params}`, { signal })
      if (!response.ok) throw new Error(String(response.status))
      const data = await response.json()
      results.value = data.results || []
      domainFacets.value = data.domain_facets || []
    } catch (e) {
      if (e.name === 'AbortError') return
      results.value = []
      domainFacets.value = []
    } finally {
      loading.value = false
    }
  }

  function triggerSearch(q) {
    clearTimeout(searchTimer)
    if (!q || !q.trim()) {
      close()
      return
    }
    searchTimer = setTimeout(() => doSearch(q.trim()), 250)
  }

  function setDomainFilter(domain) {
    domainFilter.value = domain || null
    if (query.value) doSearch(query.value)
  }

  function close() {
    open.value = false
    query.value = ''
    results.value = []
    domainFacets.value = []
    domainFilter.value = null
    selectedIdx.value = -1
  }

  function selectNext() {
    if (results.value.length === 0) return
    selectedIdx.value = Math.min(selectedIdx.value + 1, results.value.length - 1)
  }

  function selectPrev() {
    if (results.value.length === 0) return
    selectedIdx.value = Math.max(selectedIdx.value - 1, 0)
  }

  return {
    query,
    results,
    domainFacets,
    domainFilter,
    loading,
    selectedIdx,
    open,
    triggerSearch,
    doSearch,
    setDomainFilter,
    close,
    selectNext,
    selectPrev,
  }
})
