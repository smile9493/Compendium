import router from '@/router'
import { useSearchStore } from '@/stores/search'
import { normalizeWikiPath } from '@/utils/path'

/**
 * Navigate to a wiki entry (URL is the source of truth).
 * @param {string} path
 */
export function openEntry(path) {
  const normalized = normalizeWikiPath(path)
  if (!normalized) return

  const searchStore = useSearchStore()
  searchStore.close()

  router.push({ name: 'entry', params: { path: normalized } })
}

export function openHome() {
  const searchStore = useSearchStore()
  searchStore.close()
  router.push({ name: 'wiki' })
}
