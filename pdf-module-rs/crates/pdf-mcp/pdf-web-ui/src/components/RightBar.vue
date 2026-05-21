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
        :style="{ paddingLeft: `calc(12px + ${h.level} * 14px)` }"
        @click="scrollToHeading(idx)"
      >{{ h.text }}</a>
    </div>
    <div class="rb-divider"></div>
    <div class="rb-section">{{ t('rightbar.related') }}</div>
    <div class="graph-preview">
      <ul v-if="relatedPreview.length" class="graph-relation-list">
        <li
          v-for="name in relatedPreview"
          :key="name"
          class="graph-relation-item"
          @click="openEntry(name)"
        >{{ name }}</li>
      </ul>
      <span v-else class="rightbar-empty">{{ t('rightbar.selectEntry') }}</span>
    </div>
    <div class="rightbar-back-top" @click="scrollToTop">↑ {{ t('rightbar.backTop') }}</div>
  </aside>
</template>

<script setup>
import { ref, watch, nextTick, onBeforeUnmount, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useWikiStore } from '@/stores/wiki'
import { useScrollSpy } from '@/composables/useScrollSpy'
import { openEntry } from '@/composables/useWikiNavigation'

const { t } = useI18n()

const props = defineProps({
  collapsed: Boolean,
  mainScrollEl: { type: Object, default: null },
})

const wikiStore = useWikiStore()
const { activeHeadingIdx, setup: setupScrollSpy, reset: resetScrollSpy } = useScrollSpy(props.mainScrollEl)

const headings = ref([])
let proseObserver = null

const relatedPreview = computed(() => {
  const entry = wikiStore.currentEntry
  if (!entry || entry.error) return []
  const names = [
    ...(entry.related || []),
    ...(entry.backlinks || []),
  ]
  return [...new Set(names)].slice(0, 8)
})

function updateFromProse() {
  const prose = document.querySelector('.prose')
  if (!prose) {
    headings.value = []
    return
  }

  const hs = prose.querySelectorAll('h1, h2, h3, h4')
  headings.value = Array.from(hs).map((el) => ({
    level: parseInt(el.tagName[1], 10) - 1,
    text: el.textContent.trim().slice(0, 40),
  }))

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
