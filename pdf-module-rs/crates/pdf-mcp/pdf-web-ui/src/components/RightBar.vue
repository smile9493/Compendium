<template>
  <aside class="app-rightbar" :class="{ collapsed: collapsed }">
    <div class="rb-section">
      {{ t('rightbar.toc') }}
      <span v-if="headings.length" class="rb-action" @click="scrollToTop" :title="t('rightbar.backTop')">↑ {{ t('rightbar.backTop') }}</span>
    </div>
    <div id="toc-list">
      <span v-if="!headings.length" class="rightbar-empty">{{ t('rightbar.selectEntry') }}</span>
      <a
        v-for="(h, idx) in headings"
        :key="idx"
        class="toc-item"
        :class="{ active: activeHeadingIdx === idx }"
        :style="{ paddingLeft: `calc(14px + ${h.level} * 12px)` }"
        @click="scrollToHeading(idx)"
      >{{ h.text }}</a>
    </div>
    <div class="rb-divider"></div>
    <div class="rb-section">{{ t('rightbar.conceptGraph') }}</div>
    <div class="graph-preview">
      <div v-if="mermaidPreview" class="graph-mini">{{ mermaidPreview }}</div>
      <span v-else class="rightbar-empty">{{ t('rightbar.selectEntry') }}</span>
    </div>
    <div class="rightbar-back-top" @click="scrollToTop">↑ {{ t('rightbar.backTop') }}</div>
  </aside>
</template>

<script setup>
import { ref, watch, nextTick, onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'
import { useWikiStore } from '@/stores/wiki'

const { t } = useI18n()
import { useScrollSpy } from '@/composables/useScrollSpy'

const props = defineProps({
  collapsed: Boolean,
  mainScrollEl: { type: Object, default: null },
})

const wikiStore = useWikiStore()
const { activeHeadingIdx, setup: setupScrollSpy, reset: resetScrollSpy } = useScrollSpy(props.mainScrollEl)

const headings = ref([])
const mermaidPreview = ref('')
let proseObserver = null

function extractMermaidFromMarkdown(md) {
  if (!md) return ''
  const m = md.match(/```mermaid\n([\s\S]*?)```/)
  if (m) {
    return m[1].split('\n').slice(0, 8).join('\n')
  }
  return ''
}

function updateFromProse() {
  const prose = document.querySelector('.prose')
  if (!prose) {
    headings.value = []
    mermaidPreview.value = extractMermaidFromMarkdown(wikiStore.currentEntry?.body_markdown)
    return
  }

  const hs = prose.querySelectorAll('h1, h2, h3, h4')
  headings.value = Array.from(hs).map((el) => ({
    level: parseInt(el.tagName[1], 10) - 1,
    text: el.textContent.trim().slice(0, 40),
  }))

  const mermaidEl = prose.querySelector('code.language-mermaid')
  mermaidPreview.value = mermaidEl
    ? mermaidEl.textContent.split('\n').slice(0, 8).join('\n')
    : extractMermaidFromMarkdown(wikiStore.currentEntry?.body_markdown)

  setupScrollSpy()
}

function observeProse() {
  if (proseObserver) {
    proseObserver.disconnect()
    proseObserver = null
  }
  const prose = document.querySelector('.prose')
  if (!prose) return

  proseObserver = new MutationObserver(() => {
    updateFromProse()
  })
  proseObserver.observe(prose, { childList: true, subtree: true })
}

watch(
  () => wikiStore.currentEntry,
  async (entry) => {
    resetScrollSpy()
    if (!entry || entry.error) {
      headings.value = []
      mermaidPreview.value = ''
      if (proseObserver) {
        proseObserver.disconnect()
        proseObserver = null
      }
      return
    }
    await nextTick()
    updateFromProse()
    observeProse()
  },
)

onBeforeUnmount(() => {
  if (proseObserver) {
    proseObserver.disconnect()
  }
})

function scrollToHeading(idx) {
  const prose = document.querySelector('.prose')
  if (!prose) return
  const headingEls = prose.querySelectorAll('h1, h2, h3, h4')
  if (headingEls[idx]) {
    headingEls[idx].scrollIntoView({ behavior: 'smooth', block: 'start' })
    activeHeadingIdx.value = idx
  }
}

function scrollToTop() {
  const el = props.mainScrollEl?.value ?? props.mainScrollEl
  if (el) {
    el.scrollTo({ top: 0, behavior: 'smooth' })
  }
}
</script>
