/**
 * Normalize wiki entry paths for routing and API calls.
 * @param {string} path
 * @returns {string}
 */
export function normalizeWikiPath(path) {
  if (!path || typeof path !== 'string') return ''
  let p = path.trim()
  p = p.replace(/^\/api\/wiki\/entries\//, '')
  p = p.replace(/^wiki\//, '')
  p = p.replace(/^\//, '')
  if (!p) return ''
  if (!p.endsWith('.md')) {
    p = `${p}.md`
  }
  return p
}

/**
 * Strip markdown to plain text for MCP excerpts.
 * @param {string} markdown
 * @param {number} maxLen
 * @returns {string}
 */
export function markdownExcerpt(markdown, maxLen = 2000) {
  if (!markdown) return ''
  const text = markdown
    .replace(/```[\s\S]*?```/g, '')
    .replace(/`[^`]+`/g, (m) => m.slice(1, -1))
    .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
    .replace(/\[\[([^\]]+)\]\]/g, '$1')
    .replace(/^#{1,6}\s+/gm, '')
    .replace(/[*_~]/g, '')
    .replace(/\s+/g, ' ')
    .trim()
  return text.length > maxLen ? `${text.slice(0, maxLen)}…` : text
}
