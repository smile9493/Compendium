<template>
  <div>
    <div
      class="tree-node"
      :class="{ active: isActive }"
      :style="{ paddingLeft: `calc(10px + ${depth} * 14px)` }"
      @click="handleClick"
    >
      <span class="caret" :class="{ open: isOpen }" :style="{ visibility: isFolder ? 'visible' : 'hidden' }">▶</span>
      <span class="node-icon">{{ node.is_entry ? '📄' : '📁' }}</span>
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

const props = defineProps({
  node: { type: Object, required: true },
  depth: { type: Number, default: 0 },
})

const wikiStore = useWikiStore()
const isOpen = ref(true)

const isFolder = computed(() => props.node.children && props.node.children.length > 0)
const isEntry = computed(() => props.node.is_entry)

const displayName = computed(() => props.node.title || props.node.name || '')

const isActive = computed(() => wikiStore.currentPath === props.node.path)

function handleClick() {
  if (isFolder.value) {
    isOpen.value = !isOpen.value
  } else if (isEntry.value && props.node.path) {
    wikiStore.navigateTo(props.node.path)
  }
}
</script>
