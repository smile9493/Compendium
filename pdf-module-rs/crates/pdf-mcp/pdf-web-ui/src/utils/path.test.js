import { describe, it, expect } from 'vitest'
import { normalizeWikiPath, markdownExcerpt } from './path.js'

describe('normalizeWikiPath', () => {
  it('adds .md suffix', () => {
    expect(normalizeWikiPath('rust/guide')).toBe('rust/guide.md')
  })

  it('strips API prefix', () => {
    expect(normalizeWikiPath('/api/wiki/entries/foo.md')).toBe('foo.md')
  })

  it('returns empty for invalid input', () => {
    expect(normalizeWikiPath('')).toBe('')
  })
})

describe('markdownExcerpt', () => {
  it('strips markdown syntax', () => {
    const md = '# Title\n\nHello **world**'
    const excerpt = markdownExcerpt(md, 100)
    expect(excerpt).toContain('Title')
    expect(excerpt).toContain('Hello')
    expect(excerpt).not.toContain('**')
  })
})
