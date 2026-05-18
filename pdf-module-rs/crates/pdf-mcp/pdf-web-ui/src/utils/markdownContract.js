/**
 * Structural markdown metrics (must match pdf-core `markdown_contract.rs`).
 */

export function analyzeMarkdownBody(body) {
  let headingCount = 0
  let wikilinkCount = 0
  let fencedCodeBlocks = 0
  let inFence = false

  for (const line of body.split('\n')) {
    const trimmed = line.trim()
    if (trimmed.startsWith('```')) {
      if (inFence) {
        inFence = false
      } else {
        inFence = true
        fencedCodeBlocks += 1
      }
      continue
    }
    if (inFence) continue

    if (trimmed.startsWith('#')) {
      const hashes = trimmed.match(/^#+/)?.[0].length ?? 0
      if (hashes > 0 && /\s/.test(trimmed[hashes] ?? '')) {
        headingCount += 1
      }
    }
    wikilinkCount += countWikilinks(trimmed)
  }

  return { headingCount, wikilinkCount, fencedCodeBlocks }
}

function countWikilinks(line) {
  let count = 0
  let rest = line
  while (true) {
    const start = rest.indexOf('[[')
    if (start === -1) break
    const after = rest.slice(start + 2)
    const end = after.indexOf(']]')
    if (end === -1) break
    count += 1
    rest = after.slice(end + 2)
  }
  return count
}

export function countRenderedBlocks(html) {
  const pre = (html.match(/<pre[\s>]/gi) || []).length
  const code = (html.match(/<code[\s>]/gi) || []).length
  return { pre, code }
}
