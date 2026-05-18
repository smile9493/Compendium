const API_BASE = '/api'

export class ApiError extends Error {
  constructor(status, message) {
    super(message)
    this.name = 'ApiError'
    this.status = status
  }
}

async function request(path, options = {}) {
  const {
    retries = 0,
    timeout = 15000,
    signal: externalSignal,
    skipJsonContentType = false,
    ...fetchOptions
  } = options
  const url = `${API_BASE}${path}`

  let lastError = null

  for (let attempt = 0; attempt <= retries; attempt++) {
    const controller = new AbortController()
    const timer = setTimeout(() => controller.abort(), timeout)

    if (externalSignal) {
      externalSignal.addEventListener('abort', () => controller.abort())
    }

    const headers = { ...fetchOptions.headers }
    const isFormData = fetchOptions.body instanceof FormData
    if (!skipJsonContentType && !isFormData) {
      headers['Content-Type'] = headers['Content-Type'] ?? 'application/json'
    }

    try {
      const response = await fetch(url, {
        ...fetchOptions,
        signal: controller.signal,
        headers,
      })

      if (!response.ok) {
        const text = await response.text().catch(() => '')
        throw new ApiError(response.status, text || response.statusText)
      }

      return response.json()
    } catch (e) {
      clearTimeout(timer)
      if (e.name === 'AbortError' && !externalSignal?.aborted) {
        lastError = new ApiError(0, '请求超时')
      } else if (e instanceof ApiError) {
        lastError = e
      } else {
        lastError = new ApiError(0, e.message)
      }

      if (attempt < retries) {
        await new Promise(r => setTimeout(r, Math.min(500 * (attempt + 1), 2000)))
      }
    }
  }

  throw lastError
}

export const api = {
  getWikiTree() {
    return request('/wiki/tree')
  },

  getWikiEntry(path) {
    return request(`/wiki/entries/${encodeURIComponent(path)}`)
  },

  searchWiki(query, limit = 30, domain = null, signal = null) {
    const params = new URLSearchParams({ q: query, limit: String(limit) })
    if (domain) params.set('domain', domain)
    return request(`/wiki/search?${params}`, { signal })
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

  getHealth() {
    return request('/health')
  },

  getCompileStatus() {
    return request('/compile/status')
  },

  rebuildIndex() {
    return request('/index/rebuild', { method: 'POST', retries: 1 })
  },

  uploadPdf(file) {
    const formData = new FormData()
    formData.append('file', file)
    return request('/upload', {
      method: 'POST',
      body: formData,
      skipJsonContentType: true,
      timeout: 120000,
    })
  },
}