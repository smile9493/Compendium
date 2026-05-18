<template>
  <div class="content-inner share-entry-view">
    <ErrorState
      v-if="error"
      icon="⚠️"
      title="无法打开分享"
      :message="error"
    />
    <template v-else-if="entry">
      <div class="share-badge">{{ $t('share.readOnly') }}</div>
      <h1>{{ entry.title || 'Untitled' }}</h1>
      <MarkdownRenderer v-if="entry.body_markdown" :markdown="entry.body_markdown" />
    </template>
    <div v-else class="settings-loading">
      <span class="dots-loading"></span>
      加载分享条目…
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { api } from '@/api'
import MarkdownRenderer from '@/components/MarkdownRenderer.vue'
import ErrorState from '@/components/ErrorState.vue'

const route = useRoute()
const entry = ref(null)
const error = ref(null)

async function load() {
  error.value = null
  entry.value = null
  const token = route.params.token
  const path = route.params.path
  if (!token || !path) {
    error.value = '无效的分享链接'
    return
  }
  try {
    const pathStr = Array.isArray(path) ? path.join('/') : path
    const data = await api.getShareEntry(token, pathStr)
    if (data.error) {
      error.value = data.error
      return
    }
    entry.value = data.entry
  } catch (e) {
    error.value = e.message || '加载失败'
  }
}

onMounted(load)
watch(() => route.fullPath, load)
</script>
