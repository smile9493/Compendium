let mermaidModule = null
let mermaidPromise = null

export function getMermaid() {
  return mermaidModule
}

export async function loadMermaid() {
  if (mermaidModule) return mermaidModule
  if (mermaidPromise) return mermaidPromise

  mermaidPromise = (async () => {
    if (window.mermaid && typeof window.mermaid.initialize === 'function') {
      mermaidModule = window.mermaid
    } else {
      const mod = await import('mermaid')
      mermaidModule = mod.default
    }

    const theme = document.documentElement.getAttribute('data-theme') === 'dark' ? 'dark' : 'default'
    mermaidModule.initialize({
      startOnLoad: false,
      theme,
      securityLevel: 'strict',
    })

    return mermaidModule
  })()

  return mermaidPromise
}
