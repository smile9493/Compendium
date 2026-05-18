import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { i18n } from '@/i18n'
import enUS from '@/locales/en-US.json'
import { useWikiStore } from '@/stores/wiki'
import { api } from '@/api'

vi.mock('@/api', () => ({
  api: {
    getWikiEntry: vi.fn(),
    getWikiTree: vi.fn(),
    getWikiStats: vi.fn(),
  },
}))

describe('wiki store loadEntry', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    i18n.global.setLocaleMessage('en-US', enUS)
    i18n.global.locale.value = 'en-US'
    vi.clearAllMocks()
  })

  it('sets error when body_markdown is missing', async () => {
    api.getWikiEntry.mockResolvedValue({
      entry: { title: 'T', body_markdown: '   ' },
    })
    const store = useWikiStore()
    await store.loadEntry('IT/foo.md')
    expect(store.currentEntry.error).toBe('Entry has no Markdown body')
  })
})
