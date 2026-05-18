const API_BASE = '/api'

let activeKbId = null

export function setActiveKbId(kbId) {
  activeKbId = kbId || null
}

function withKb(path) {
  if (!activeKbId) return path
  const sep = path.includes('?') ? '&' : '?'
  return `${path}${sep}kb_id=${encodeURIComponent(activeKbId)}`
}

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
  listWorkspaces() {
    return request('/v1/workspaces')
  },

  setActiveWorkspace(kbId) {
    return request('/v1/workspaces/active', {
      method: 'POST',
      body: JSON.stringify({ kb_id: kbId }),
    })
  },

  getWikiTree() {
    return request(withKb('/wiki/tree'))
  },

  getWikiEntry(path) {
    return request(withKb(`/wiki/entries/${encodeURIComponent(path)}`))
  },

  searchWiki(query, limit = 30, domain = null, mode = 'hybrid', signal = null) {
    const params = new URLSearchParams({ q: query, limit: String(limit), mode })
    if (domain) params.set('domain', domain)
    return request(withKb(`/wiki/search?${params}`), { signal })
  },

  getWikiGraph(path) {
    return request(withKb(`/wiki/graph/${encodeURIComponent(path)}`))
  },

  getWikiStats() {
    return request(withKb('/wiki/stats'))
  },

  getWikiDomains() {
    return request(withKb('/wiki/domains'))
  },

  getConfig() {
    return request(withKb('/config'))
  },

  setConfig(key, value) {
    return request(withKb('/config'), {
      method: 'POST',
      body: JSON.stringify({ key, value }),
    })
  },

  removeConfig(key) {
    return request(withKb(`/config/${encodeURIComponent(key)}`), { method: 'DELETE' })
  },

  getHealth() {
    return request(withKb('/health'))
  },

  getCompileStatus() {
    return request(withKb('/compile/status'))
  },

  rebuildIndex() {
    return request(withKb('/index/rebuild'), { method: 'POST', retries: 1 })
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

  compileUploaded(fileId, domain = null) {
    return request(withKb('/compile/upload'), {
      method: 'POST',
      body: JSON.stringify({ file_id: fileId, domain }),
      timeout: 300000,
    })
  },

  triggerIncrementalCompile() {
    return request(withKb('/compile/incremental'), { method: 'POST', timeout: 300000 })
  },

  getQualitySummary() {
    return request(withKb('/quality/summary'))
  },

  getQualityIssues(limit = 50, severity = null) {
    const params = new URLSearchParams({ limit: String(limit) })
    if (severity) params.set('severity', severity)
    return request(withKb(`/quality/issues?${params}`))
  },
}