import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '@/api'
import { useWikiStore } from '@/stores/wiki'

export const useSearchStore = defineStore('search', () => {
  const query = ref('')
  const results = ref([])
  const loading = ref(false)
  const open = ref(false)
  const selectedIdx = ref(-1)
  const error = ref(null)
  const domainFacets = ref([])
  const searchMode = ref('hybrid')
  const searchMeta = ref(null)

  let debounceTimer = null
  let currentController = null

  const hasResults = computed(() => results.value.length > 0)

  function close() {
    open.value = false
    results.value = []
    selectedIdx.value = -1
    error.value = null
    domainFacets.value = []
    searchMeta.value = null
  }

  function clearAndClose() {
    query.value = ''
    close()
  }

  function setDomainFilter(domain) {
    const wikiStore = useWikiStore()
    wikiStore.domainFilter = domain
    if (query.value.trim().length >= 2) {
      triggerSearch(query.value)
    }
  }

  function triggerSearch(q) {
    if (currentController) {
      currentController.abort()
    }

    query.value = q
    clearTimeout(debounceTimer)

    if (!q || q.trim().length < 2) {
      results.value = []
      open.value = false
      error.value = null
      domainFacets.value = []
      searchMeta.value = null
      return
    }

    loading.value = true
    error.value = null
    selectedIdx.value = -1

    debounceTimer = setTimeout(async () => {
      currentController = new AbortController()
      const wikiStore = useWikiStore()
      try {
        const data = await api.searchWiki(
          q.trim(),
          30,
          wikiStore.activeDomain,
          searchMode.value,
          currentController.signal,
        )
        const entries = data.results || data.entries || data || []
        results.value = entries
        searchMeta.value = data.meta || null
        open.value = true
        domainFacets.value = data.domain_facets?.length
          ? data.domain_facets
          : extractFacets(entries)
        if (data.meta?.index_empty && entries.length === 0) {
          error.value = '全文索引为空，请在编译抽屉中完成编译或重建索引。'
        }
      } catch (e) {
        if (e.name === 'AbortError') return
        error.value = e.message || 'Search failed'
        results.value = []
        open.value = true
      } finally {
        if (currentController?.signal.aborted) return
        loading.value = false
      }
    }, 200)
  }

  function extractFacets(entries) {
    const facetMap = {}
    for (const entry of entries) {
      const domain = entry.domain || '其他'
      if (!facetMap[domain]) {
        facetMap[domain] = { domain, count: 0 }
      }
      facetMap[domain].count++
    }
    return Object.values(facetMap).sort((a, b) => b.count - a.count)
  }

  function selectNext() {
    if (results.value.length > 0) {
      selectedIdx.value = Math.min(selectedIdx.value + 1, results.value.length - 1)
    }
  }

  function selectPrev() {
    selectedIdx.value = Math.max(selectedIdx.value - 1, -1)
  }

  return {
    query,
    results,
    loading,
    open,
    selectedIdx,
    error,
    domainFacets,
    searchMode,
    searchMeta,
    hasResults,
    close,
    clearAndClose,
    setDomainFilter,
    triggerSearch,
    selectNext,
    selectPrev,
  }
})
