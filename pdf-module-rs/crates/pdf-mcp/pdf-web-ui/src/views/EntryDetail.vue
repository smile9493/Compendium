<template>
  <div class="content-inner">
    <div v-if="wikiStore.currentEntry">
      <div class="reading-progress" :style="{ transform: `scaleX(${progress})` }"></div>

      <ErrorState
        v-if="wikiStore.currentEntry.error"
        icon="⚠️"
        :title="errorTitle"
        :message="wikiStore.currentEntry.error"
        retryable
        @retry="retryLoad"
      />
      <template v-else>
        <div class="breadcrumb">
          <span @click="openHome">📚 首页</span>
          <template v-for="(part, i) in pathParts" :key="i">
            <span class="sep">/</span>
            <span
              @click="navigateToBreadcrumb(i)"
              :style="{ cursor: isLast(i) ? 'default' : 'pointer', color: isLast(i) ? 'var(--text)' : '' }"
            >
              {{ part }}
            </span>
          </template>
        </div>

        <div class="entry-header">
          <h1>{{ wikiStore.currentEntry.title || 'Untitled' }}</h1>
          <div class="meta-inline">
            <span class="meta-item">{{ wikiStore.currentEntry.domain || '?' }}</span>
            <span class="meta-sep">·</span>
            <span class="meta-item">{{ wikiStore.currentEntry.level || 'L1' }}</span>
            <span class="meta-sep">·</span>
            <span
              class="meta-item"
              :class="{
                'meta-warn': wikiStore.currentEntry.status === 'needs_recompile',
                'meta-error': wikiStore.currentEntry.status === 'error',
              }"
            >{{ wikiStore.currentEntry.status || '?' }}</span>
            <span class="meta-sep">·</span>
            <span class="meta-item">{{ t('entry.quality', { score: qualityScore }) }}</span>
            <template v-if="wikiStore.currentEntry.version">
              <span class="meta-sep">·</span>
              <span class="meta-item">v{{ wikiStore.currentEntry.version }}</span>
            </template>
            <template v-if="wikiStore.currentEntry.source">
              <span class="meta-sep">·</span>
              <span class="meta-item">{{ t('entry.source', { name: wikiStore.currentEntry.source }) }}</span>
            </template>
            <span class="meta-sep">·</span>
            <button type="button" class="btn btn-sm share-link-btn meta-share-btn" @click="copyShareLink">
              {{ shareCopied ? $t('share.copied') : $t('share.copyLink') }}
            </button>
          </div>
          <div v-if="hasTags" class="tag-list">
            <span v-for="tag in wikiStore.currentEntry.tags" :key="tag" class="tag-hash">#{{ tag }}</span>
          </div>
        </div>

        <MarkdownRenderer v-if="entryMarkdown" :markdown="entryMarkdown" />

        <div v-if="hasRelations" class="relations">
          <div v-if="wikiStore.currentEntry.related?.length" class="relation-section">
            <h3>🔗 相关条目</h3>
            <div class="link-grid">
              <span
                v-for="r in wikiStore.currentEntry.related"
                :key="r"
                class="link-card"
                @click="openEntry(r)"
              >{{ r }}</span>
            </div>
          </div>
          <div v-if="wikiStore.currentEntry.contradictions?.length" class="relation-section">
            <h3>⚡ 矛盾条目</h3>
            <div class="link-grid">
              <span
                v-for="r in wikiStore.currentEntry.contradictions"
                :key="r"
                class="link-card contradiction"
                @click="openEntry(r)"
              >{{ r }}</span>
            </div>
          </div>
          <div v-if="wikiStore.currentEntry.backlinks?.length" class="relation-section">
            <h3>↩️ 反向链接</h3>
            <div class="link-grid">
              <span
                v-for="r in wikiStore.currentEntry.backlinks"
                :key="r"
                class="link-card"
                @click="openEntry(r)"
              >{{ r }}</span>
            </div>
          </div>
        </div>
      </template>
    </div>

    <div v-else class="skeleton-entry">
      <div class="skeleton skeleton-heading"></div>
      <div class="skeleton-meta">
        <span class="skeleton-chip"></span>
        <span class="skeleton-chip"></span>
        <span class="skeleton-chip"></span>
      </div>
      <div class="skeleton skeleton-text medium"></div>
      <div class="skeleton skeleton-text medium"></div>
      <div class="skeleton skeleton-text short"></div>
      <div class="skeleton skeleton-block"></div>
      <div class="skeleton skeleton-text"></div>
      <div class="skeleton skeleton-text"></div>
      <div class="skeleton skeleton-text medium"></div>
    </div>
  </div>
</template>

<script setup>
import { computed, watch, nextTick, onBeforeUnmount, ref } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useWikiStore } from '@/stores/wiki'
import { useWorkspaceStore } from '@/stores/workspace'
import { api } from '@/api'
import { useReadingProgress } from '@/composables/useReadingProgress'
import { openEntry, openHome } from '@/composables/useWikiNavigation'
import MarkdownRenderer from '@/components/MarkdownRenderer.vue'
import ErrorState from '@/components/ErrorState.vue'

const { t } = useI18n()
const route = useRoute()
const wikiStore = useWikiStore()
const workspaceStore = useWorkspaceStore()
const shareCopied = ref(false)

async function copyShareLink() {
  const kbId = workspaceStore.activeKbId
  const path = wikiStore.currentPath
  if (!kbId || !path) return
  try {
    const { token } = await api.createShareLink(kbId, path)
    const url = `${window.location.origin}${window.location.pathname}#/share/${token}/${path.replace(/^\//, '')}`
    await navigator.clipboard.writeText(url)
    shareCopied.value = true
    setTimeout(() => {
      shareCopied.value = false
    }, 2000)
  } catch (e) {
    console.error('Share link failed', e)
  }
}

const { progress, setup: setupProgress } = useReadingProgress()

const SCROLL_KEY_PREFIX = 'scroll:'

const pathParts = computed(() => {
  const p = wikiStore.currentPath || ''
  return p.replace('.md', '').split('/').filter(Boolean)
})

const entryMarkdown = computed(() => wikiStore.currentEntry?.body_markdown?.trim() || '')

const errorTitle = computed(() => {
  const err = wikiStore.currentEntry?.error || ''
  return err.includes('Markdown') ? '正文不可用' : '加载失败'
})

function isLast(i) {
  return i === pathParts.value.length - 1
}

function navigateToBreadcrumb(i) {
  const p = pathParts.value.slice(0, i + 1).join('/') + '.md'
  openEntry(p)
}

function retryLoad() {
  if (wikiStore.currentPath) {
    wikiStore.loadEntry(wikiStore.currentPath, { force: true })
  }
}

const qualityScore = computed(() => {
  const q = wikiStore.currentEntry?.quality_score
  if (q == null) return '-'
  return `${(q * 100).toFixed(0)}%`
})

const hasTags = computed(() => wikiStore.currentEntry?.tags?.length > 0)

const hasRelations = computed(() => {
  const e = wikiStore.currentEntry
  return e && (e.related?.length || e.contradictions?.length || e.backlinks?.length)
})

function scrollStorageKey() {
  return `${SCROLL_KEY_PREFIX}${wikiStore.currentPath || ''}`
}

function saveScroll() {
  const main = document.querySelector('.app-main')
  if (main && wikiStore.currentPath) {
    sessionStorage.setItem(scrollStorageKey(), String(main.scrollTop))
  }
}

function restoreScroll() {
  const main = document.querySelector('.app-main')
  const raw = sessionStorage.getItem(scrollStorageKey())
  if (main && raw) {
    const top = parseInt(raw, 10)
    if (!Number.isNaN(top)) {
      main.scrollTop = top
    }
  }
}

watch(
  () => wikiStore.currentEntry,
  async (entry) => {
    if (!entry || entry.error) return
    document.title = `${entry.title || 'Untitled'} - Compendium 知识库`
    await nextTick()
    setupProgress()
    restoreScroll()
  }
)

onBeforeUnmount(() => {
  saveScroll()
})

watch(
  () => route.fullPath,
  () => {
    saveScroll()
  }
)
</script>
