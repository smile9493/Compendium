<template>
  <div class="empty-state">
    <div class="empty-icon" :style="{ color: iconColor }">
      <BookOpen v-if="icon === 'book'" :size="52" :stroke-width="1.5" />
      <Search v-else-if="icon === 'search'" :size="52" :stroke-width="1.5" />
      <FolderOpen v-else-if="icon === 'folder'" :size="52" :stroke-width="1.5" />
      <GitBranch v-else-if="icon === 'tree'" :size="52" :stroke-width="1.5" />
      <AlertTriangle v-else-if="icon === 'warning'" :size="52" :stroke-width="1.5" />
      <XCircle v-else-if="icon === 'error'" :size="52" :stroke-width="1.5" />
      <Settings v-else-if="icon === 'settings'" :size="52" :stroke-width="1.5" />
      <FileQuestion v-else :size="52" :stroke-width="1.5" />
    </div>
    <h3 class="empty-title">{{ title }}</h3>
    <p v-if="description" class="empty-desc">{{ description }}</p>
    <button v-if="action" class="btn btn-primary empty-action" @click="$emit('action')">
      {{ action }}
    </button>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { BookOpen, Search, FolderOpen, GitBranch, AlertTriangle, XCircle, Settings, FileQuestion } from 'lucide-vue-next'

const props = defineProps({
  icon: { type: String, default: 'book' },
  title: { type: String, default: '暂无内容' },
  description: { type: String, default: '' },
  action: { type: String, default: '' },
})

defineEmits(['action'])

const iconColor = computed(() => 'var(--text-muted)')
</script>