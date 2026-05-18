import { describe, it, expect, vi, beforeEach } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { openEntry } from '@/composables/useWikiNavigation'

const push = vi.fn()

vi.mock('@/router', () => ({
  default: { push },
}))

vi.mock('@/stores/search', () => ({
  useSearchStore: () => ({ close: vi.fn() }),
}))

describe('useWikiNavigation', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    push.mockClear()
  })

  it('openEntry pushes normalized path to router', () => {
    openEntry('/IT/foo.md')
    expect(push).toHaveBeenCalledWith({
      name: 'entry',
      params: { path: 'IT/foo.md' },
    })
  })
})
