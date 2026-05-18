import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import CompileStageList from '@/components/CompileStageList.vue'

describe('CompileStageList', () => {
  it('renders stage rows', () => {
    const wrapper = mount(CompileStageList, {
      props: {
        stages: [
          { stage: 'extract', status: 'succeeded', duration_ms: 120 },
          { stage: 'agent_wiki', status: 'running' },
        ],
      },
    })
    expect(wrapper.text()).toContain('提取')
    expect(wrapper.text()).toContain('succeeded')
    expect(wrapper.text()).toContain('120ms')
  })
})
