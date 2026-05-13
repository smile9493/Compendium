<template>
  <Transition name="fade">
    <div v-if="open" class="overlay open" @click.self="$emit('close')">
      <div class="dialog domain-dialog">
        <button class="dialog-close" @click="$emit('close')">&times;</button>
        <h2>🏷️ 领域总览</h2>
        <div v-if="wikiStore.domainsFromTree.length" class="domain-cards">
          <div
            v-for="d in wikiStore.domainsFromTree"
            :key="d.domain"
            class="domain-card"
            :class="{ selected: wikiStore.domainFilter === d.domain }"
            @click="selectDomain(d.domain)"
          >
            <div class="domain-card-header">
              <span class="dc-name">📁 {{ d.domain }}</span>
              <span class="dc-count">{{ d.count }} 个条目</span>
            </div>
            <div class="dc-paths">
              <span v-for="(p, i) in d.paths.slice(0, 10)" :key="i" class="dc-path">{{ p }}</span>
              <span v-if="d.paths.length > 10" class="dc-path">+{{ d.paths.length - 10 }} 更多</span>
            </div>
          </div>
        </div>
        <div v-else class="domain-empty">暂无领域数据</div>
      </div>
    </div>
  </Transition>
</template>

<script setup>
import { useWikiStore } from '@/stores/wiki'

defineProps({ open: Boolean })
defineEmits(['close'])

const wikiStore = useWikiStore()

function selectDomain(domain) {
  const newFilter = wikiStore.domainFilter === domain ? null : domain
  wikiStore.domainFilter = newFilter
}
</script>
