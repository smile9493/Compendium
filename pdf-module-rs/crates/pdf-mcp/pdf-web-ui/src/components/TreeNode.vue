<template>
  <div>
    <div
      class="tree-node"
      :class="{ active: isActive }"
      :style="{ paddingLeft: `calc(14px + ${depth} * 14px)` }"
      @click="handleClick"
      role="treeitem"
      :tabindex="isActive ? 0 : -1"
      :aria-expanded="isFolder ? isOpen : undefined"
    >
      <span class="caret" :class="{ open: isOpen }" :style="{ visibility: isFolder ? 'visible' : 'hidden' }">▶</span>
      <span class="node-icon">{{ isFolder && !isOpen ? '📁' : isEntry ? '📄' : '📂' }}</span>
      <span>{{ displayName }}</span>
    </div>
    <div v-if="isFolder" class="tree-children" :class="{ collapsed: !isOpen }">
      <TreeNode
        v-for="child in node.children"
        :key="child.path || child.name"
        :node="child"
        :depth="depth + 1"
      />
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { useWikiStore } from '@/stores/wiki'
import { openEntry } from '@/composables/useWikiNavigation'

const props = defineProps({
  node: { type: Object, required: true },
  depth: { type: Number, default: 0 },
})

const wikiStore = useWikiStore()
const isOpen = ref(true)

const isFolder = computed(() => props.node.children && props.node.children.length > 0)
const isEntry = computed(() => props.node.is_entry)

const displayName = computed(() => props.node.title || props.node.name || '')

const isActive = computed(() => {
  const current = wikiStore.currentPath || ''
  const target = props.node.path || ''
  return current === target
})

function handleClick() {
  if (isFolder.value) {
    isOpen.value = !isOpen.value
  }
  if (isEntry.value && props.node.path) {
    openEntry(props.node.path)
  }
}
</script>