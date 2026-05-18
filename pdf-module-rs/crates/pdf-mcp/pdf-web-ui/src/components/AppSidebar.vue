<template>
  <aside class="app-sidebar" :class="{ collapsed: collapsed }">
    <div class="domain-chips">
      <span
        v-for="d in wikiStore.domainsFromTree"
        :key="d.domain"
        class="domain-chip"
        :class="{ active: wikiStore.domainFilter === d.domain }"
        @click="toggleDomain(d.domain)"
      >
        {{ d.domain }}<span class="chip-count">{{ d.count }}</span>
      </span>
      <span v-if="!wikiStore.domainsFromTree.length && !wikiStore.loadingTree" class="rightbar-empty">暂无领域</span>
    </div>
    <div class="sidebar-divider"></div>
    <div class="sidebar-section">
      知识目录
      <span v-if="wikiStore.activeDomain" class="section-action" @click="wikiStore.domainFilter = null">清除目录筛选</span>
    </div>
    <div v-if="wikiStore.loadingTree" class="loading-placeholder">
      <span class="dots-loading"></span>加载中…
    </div>
    <div v-else id="tree-root">
      <template v-if="wikiStore.tree && wikiStore.tree.children">
        <div v-for="node in filteredChildren" :key="node.path || node.name">
          <TreeNode :node="node" :depth="0" />
        </div>
      </template>
      <EmptyState
        v-else-if="!wikiStore.tree"
        icon="folder"
        title="知识库未配置"
        description="请在设置中添加知识库路径"
      />
    </div>
  </aside>
</template>

<script setup>
import { computed } from 'vue'
import { useWikiStore } from '@/stores/wiki'
import TreeNode from './TreeNode.vue'
import EmptyState from './EmptyState.vue'

defineProps({ collapsed: Boolean })

const wikiStore = useWikiStore()

const filteredChildren = computed(() => {
  const children = wikiStore.tree?.children || []
  if (!wikiStore.domainFilter) return children
  return children.filter(n => matchesDomain(n, wikiStore.domainFilter))
})

function matchesDomain(node, domain) {
  if (node.domain === domain) return true
  if (node.children) return node.children.some(c => matchesDomain(c, domain))
  return false
}

function toggleDomain(domain) {
  wikiStore.domainFilter = wikiStore.domainFilter === domain ? null : domain
}
</script>