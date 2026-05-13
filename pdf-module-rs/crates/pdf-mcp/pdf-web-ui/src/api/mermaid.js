let mermaidPromise = null

export function loadMermaid() {
  if (mermaidPromise) return mermaidPromise

  mermaidPromise = new Promise((resolve, reject) => {
    // If already loaded (e.g., in MCP iframe context)
    if (window.mermaid && typeof window.mermaid.initialize === 'function') {
      resolve(window.mermaid)
      return
    }

    const script = document.createElement('script')
    script.src = 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.min.js'
    script.onload = () => {
      window.mermaid.initialize({
        startOnLoad: false,
        theme: document.documentElement.getAttribute('data-theme') === 'dark' ? 'dark' : 'default',
        securityLevel: 'loose',
      })
      resolve(window.mermaid)
    }
    script.onerror = () => {
      mermaidPromise = null
      reject(new Error('Failed to load Mermaid.js'))
    }
    document.head.appendChild(script)
  })

  return mermaidPromise
}
