const API_BASE = '/api'

async function request(path, options = {}) {
  const url = `${API_BASE}${path}`
  const response = await fetch(url, {
    headers: { 'Content-Type': 'application/json', ...options.headers },
    ...options,
  })
  if (!response.ok) {
    const text = await response.text().catch(() => '')
    throw new Error(`API ${response.status}: ${text || response.statusText}`)
  }
  return response.json()
}

export const api = {
  // Wiki
  getWikiTree() {
    return request('/wiki/tree')
  },

  getWikiEntry(path) {
    return request(`/wiki/entries/${encodeURIComponent(path)}`)
  },

  searchWiki(query, limit = 30, domain = null) {
    const params = new URLSearchParams({ q: query, limit: String(limit) })
    if (domain) params.set('domain', domain)
    return request(`/wiki/search?${params}`)
  },

  getWikiGraph(path) {
    return request(`/wiki/graph/${encodeURIComponent(path)}`)
  },

  getWikiStats() {
    return request('/wiki/stats')
  },

  getWikiDomains() {
    return request('/wiki/domains')
  },

  // Config
  getConfig() {
    return request('/config')
  },

  setConfig(key, value) {
    return request('/config', {
      method: 'POST',
      body: JSON.stringify({ key, value }),
    })
  },

  removeConfig(key) {
    return request(`/config/${encodeURIComponent(key)}`, { method: 'DELETE' })
  },

  // Health & Compile
  getHealth() {
    return request('/health')
  },

  getCompileStatus() {
    return request('/compile/status')
  },

  rebuildIndex() {
    return request('/index/rebuild', { method: 'POST' })
  },

  // Upload
  uploadPdf(file) {
    const formData = new FormData()
    formData.append('file', file)
    return fetch(`${API_BASE}/upload`, { method: 'POST', body: formData })
      .then(r => r.json())
  },
}
