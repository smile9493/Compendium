const STORAGE_KEY = 'pdf-wiki-recent'
const MAX_RECENT = 3

export function getRecentPaths() {
  if (typeof localStorage === 'undefined') return []
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    const list = raw ? JSON.parse(raw) : []
    return Array.isArray(list) ? list.filter((p) => typeof p === 'string').slice(0, MAX_RECENT) : []
  } catch {
    return []
  }
}

export function pushRecentPath(path) {
  if (!path || typeof localStorage === 'undefined') return
  const list = getRecentPaths().filter((p) => p !== path)
  list.unshift(path)
  localStorage.setItem(STORAGE_KEY, JSON.stringify(list.slice(0, MAX_RECENT)))
}

export function resolveRecentLabels(paths, tree) {
  if (!paths.length) return []
  const nameByPath = new Map()

  function walk(node) {
    if (node.is_entry && node.path) {
      nameByPath.set(node.path, node.name || node.path)
    }
    node.children?.forEach(walk)
  }
  if (tree) walk(tree)

  return paths.map((path) => ({
    path,
    label: nameByPath.get(path) || path.split('/').pop() || path,
  }))
}
