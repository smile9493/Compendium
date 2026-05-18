import { describe, it, expect, beforeEach } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { useCompileStore } from '@/stores/compile'
import enUS from '@/locales/en-US.json'

describe('compile store i18n labels', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('maps awaiting_agent to English label via i18n helper', () => {
    const i18n = createI18n({ legacy: false, locale: 'en-US', messages: { 'en-US': enUS } })
    const store = useCompileStore()
    store.compileStatus = { pipeline_status: 'awaiting_agent', running: true }
    const key =
      store.pipelineStatus === 'awaiting_agent' ? 'compile.statusAwaiting' : 'compile.statusIdle'
    expect(i18n.global.t(key)).toBe('Awaiting agent')
  })
})
