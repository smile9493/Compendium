import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { marked } from 'marked'
import { analyzeMarkdownBody, countRenderedBlocks } from './markdownContract.js'
import { normalizeWikiPath } from './path.js'

const __dirname = dirname(fileURLToPath(import.meta.url))
const fixturePath = join(
  __dirname,
  '../../../../pdf-core/tests/fixtures/wiki_sample.md',
)

function stripFrontMatter(content) {
  const trimmed = content.trimStart()
  if (!trimmed.startsWith('---')) return trimmed
  const parts = trimmed.split('---')
  return parts.length >= 3 ? parts.slice(2).join('---').trimStart() : trimmed
}

function renderWithWikilinks(markdown) {
  let rendered = marked.parse(markdown)
  rendered = rendered.replace(/\[\[([^\]]+)\]\]/g, (_match, rawPath) => {
    const normalized = normalizeWikiPath(rawPath)
    if (!normalized) return rawPath
    return `<a class="wikilink" data-path="${normalized}">link</a>`
  })
  return rendered
}

describe('markdownContract', () => {
  const raw = readFileSync(fixturePath, 'utf8')
  const body = stripFrontMatter(raw)
  const structure = analyzeMarkdownBody(body)

  it('matches expected structure counts', () => {
    expect(structure.headingCount).toBe(3)
    expect(structure.wikilinkCount).toBe(3)
    expect(structure.fencedCodeBlocks).toBe(2)
  })

  it('marked output preserves wikilink and code block counts', () => {
    const html = renderWithWikilinks(body)
    const wikilinks = (html.match(/class="wikilink"/g) || []).length
    const blocks = countRenderedBlocks(html)
    expect(wikilinks).toBe(structure.wikilinkCount)
    expect(blocks.pre).toBeGreaterThanOrEqual(structure.fencedCodeBlocks)
  })
})
