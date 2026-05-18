import { describe, it, expect, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import OpsBanner from '@/components/OpsBanner.vue'
import { useSearchStore } from '@/stores/search'
import zhCN from '@/locales/zh-CN.json'

describe('OpsBanner', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('shows when search index is empty', async () => {
    const i18n = createI18n({ legacy: false, locale: 'zh-CN', messages: { 'zh-CN': zhCN } })
    const searchStore = useSearchStore()
    searchStore.searchMeta = { index_empty: true, used_fallback: false, mode: 'hybrid' }
    searchStore.open = true

    const wrapper = mount(OpsBanner, {
      global: { plugins: [i18n] },
    })
    expect(wrapper.text()).toContain('索引为空')
  })
})
