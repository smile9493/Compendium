import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '@/api'

export const useWikiStore = defineStore('wiki', () => {
  const tree = ref(null)
  const currentEntry = ref(null)
  const currentPath = ref(null)
  const domains = ref([])
  const stats = ref(null)
  const domainFilter = ref(null)
  const darkTheme = ref(true)
  const loadingTree = ref(false)

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

  async function navigateTo(path) {
    currentPath.value = path
    currentEntry.value = null
    try {
      const data = await api.getWikiEntry(path)
      if (data.error) throw new Error(data.error)
      currentEntry.value = data.entry
    } catch (e) {
      currentEntry.value = { error: e.message }
    }
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
  }

  function initTheme() {
    document.documentElement.setAttribute('data-theme', darkTheme.value ? 'dark' : 'light')
  }

  return {
    tree,
    currentEntry,
    currentPath,
    domains,
    stats,
    domainFilter,
    darkTheme,
    loadingTree,
    domainsFromTree,
    loadTree,
    navigateTo,
    loadStats,
    toggleTheme,
    initTheme,
  }
})
