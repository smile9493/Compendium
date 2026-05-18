import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '@/api'
import { i18n } from '@/i18n'
import { normalizeWikiPath } from '@/utils/path'

const CACHE_TTL_MS = 5 * 60 * 1000

export const useWikiStore = defineStore('wiki', () => {
  const tree = ref(null)
  const currentEntry = ref(null)
  const currentPath = ref(null)
  const domains = ref([])
  const stats = ref(null)
  /** @deprecated use activeDomain — kept for gradual migration */
  const domainFilter = ref(null)
  const activeDomain = domainFilter
  const darkTheme = ref(true)
  const loadingTree = ref(false)
  const loadingEntry = ref(false)
  const readingMode = ref(false)

  /** @type {Map<string, { entry: object, fetchedAt: number }>} */
  const entryCache = new Map()

  const domainsFromTree = computed(() => {
    if (!tree.value) return []
    const seen = new Set()
    const result = []
    function walk(node) {
      if (node.domain && !seen.has(node.domain)) {
        seen.add(node.domain)
        result.push({ domain: node.domain, count: 0, paths: [] })
      }
      if (node.children) node.children.forEach(walk)
      if (node.is_entry && node.domain) {
        const d = result.find(x => x.domain === node.domain)
        if (d) {
          d.count++
          d.paths.push(node.path || '')
        }
      }
    }
    walk(tree.value)
    return result
  })

  function clearEntryCache() {
    entryCache.clear()
  }

  function getCachedEntry(path) {
    const cached = entryCache.get(path)
    if (!cached) return null
    if (Date.now() - cached.fetchedAt > CACHE_TTL_MS) {
      entryCache.delete(path)
      return null
    }
    return cached.entry
  }

  function setCachedEntry(path, entry) {
    entryCache.set(path, { entry, fetchedAt: Date.now() })
  }

  async function loadTree() {
    loadingTree.value = true
    try {
      const data = await api.getWikiTree()
      tree.value = data.tree
    } catch (e) {
      tree.value = null
    } finally {
      loadingTree.value = false
    }
  }

  async function loadEntry(path, { force = false } = {}) {
    const normalized = normalizeWikiPath(path)
    if (!normalized) {
      currentPath.value = null
      currentEntry.value = { error: i18n.global.t('entry.invalidPath') }
      return
    }

    currentPath.value = normalized

    if (!force) {
      const cached = getCachedEntry(normalized)
      if (cached) {
        currentEntry.value = cached
        return
      }
    }

    loadingEntry.value = true
    currentEntry.value = null
    try {
      const data = await api.getWikiEntry(normalized)
      if (data.error) throw new Error(data.error)
      const entry = data.entry
      if (!entry?.body_markdown?.trim()) {
        currentEntry.value = { error: i18n.global.t('entry.noBody') }
        return
      }
      currentEntry.value = entry
      setCachedEntry(normalized, entry)
    } catch (e) {
      currentEntry.value = { error: e.message }
    } finally {
      loadingEntry.value = false
    }
  }

  /** @deprecated Use openEntry() + router-driven loadEntry */
  async function navigateTo(path) {
    await loadEntry(path)
  }

  function clearCurrentEntry() {
    currentPath.value = null
    currentEntry.value = null
  }

  async function loadStats() {
    try {
      const data = await api.getWikiStats()
      stats.value = data
    } catch (e) {
      stats.value = null
    }
  }

  function toggleTheme() {
    darkTheme.value = !darkTheme.value
    document.documentElement.setAttribute('data-theme', darkTheme.value ? 'dark' : 'light')
    localStorage.setItem('pdf-wiki-theme', darkTheme.value ? 'dark' : 'light')
  }

  function setActiveDomain(domain) {
    domainFilter.value = domainFilter.value === domain ? null : domain
  }

  function setDomainFilter(domain) {
    setActiveDomain(domain)
  }

  function toggleReadingMode() {
    readingMode.value = !readingMode.value
    localStorage.setItem('pdf-wiki-reading-mode', readingMode.value ? '1' : '0')
  }

  function initTheme() {
    const saved = localStorage.getItem('pdf-wiki-theme')
    if (saved) {
      darkTheme.value = saved === 'dark'
    }
    document.documentElement.setAttribute('data-theme', darkTheme.value ? 'dark' : 'light')
    readingMode.value = localStorage.getItem('pdf-wiki-reading-mode') === '1'
  }

  return {
    tree,
    currentEntry,
    currentPath,
    domains,
    stats,
    domainFilter,
    activeDomain,
    darkTheme,
    loadingTree,
    loadingEntry,
    readingMode,
    domainsFromTree,
    loadTree,
    loadEntry,
    navigateTo,
    clearCurrentEntry,
    clearEntryCache,
    loadStats,
    toggleTheme,
    setActiveDomain,
    setDomainFilter,
    toggleReadingMode,
    initTheme,
  }
})
