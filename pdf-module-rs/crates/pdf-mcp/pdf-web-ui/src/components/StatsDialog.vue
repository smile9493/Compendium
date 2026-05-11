<template>
  <Transition name="fade">
    <div v-if="open" class="overlay open" @click.self="$emit('close')">
      <div class="dialog stats-dialog">
        <button class="dialog-close" @click="$emit('close')">&times;</button>
        <h2>📊 知识库统计</h2>
        <div v-if="wikiStore.stats" class="stat-grid2">
          <div class="stat-cell">
            <div class="sv">{{ wikiStore.stats.total_entries || 0 }}</div>
            <div class="sl">总条目</div>
          </div>
          <div class="stat-cell">
            <div class="sv">{{ wikiStore.stats.orphan_count || 0 }}</div>
            <div class="sl">孤立条目</div>
          </div>
          <div class="stat-cell">
            <div class="sv">{{ wikiStore.stats.contradiction_count || 0 }}</div>
            <div class="sl">矛盾对</div>
          </div>
          <div class="stat-cell">
            <div class="sv">{{ wikiStore.stats.broken_link_count || 0 }}</div>
            <div class="sl">断链</div>
          </div>
          <div class="stat-cell">
            <div class="sv">{{ wikiStore.stats.graph_node_count || 0 }}</div>
            <div class="sl">图节点</div>
          </div>
          <div class="stat-cell">
            <div class="sv">{{ formatQuality(wikiStore.stats.avg_quality_score) }}</div>
            <div class="sl">平均质量分</div>
          </div>
        </div>
        <div v-else class="loading-placeholder">加载中…</div>
        <div class="stats-footer">
          <button class="header-btn" @click="$emit('close')">关闭</button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup>
import { watch } from 'vue'
import { useWikiStore } from '@/stores/wiki'

defineProps({ open: Boolean })
defineEmits(['close'])

const wikiStore = useWikiStore()

watch(() => open, (val) => {
  if (val) wikiStore.loadStats()
})

function formatQuality(score) {
  if (score == null) return '-'
  return `${(score * 100).toFixed(0)}%`
}
</script>
