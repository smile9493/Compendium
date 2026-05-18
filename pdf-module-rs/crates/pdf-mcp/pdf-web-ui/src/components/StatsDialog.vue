<template>
  <Transition name="fade">
    <div v-if="open" class="overlay open" @click.self="emit('close')">
      <div class="dialog stats-dialog">
        <button class="dialog-close" @click="emit('close')">
          <X :size="16" />
        </button>

        <div class="stats-header">
          <div class="stats-icon">
            <BarChart2 :size="18" />
          </div>
          <div class="stats-title-wrap">
            <div class="stats-title">知识库统计</div>
            <div class="stats-subtitle">系统运行状态概览</div>
          </div>
        </div>

        <div v-if="wikiStore.stats" class="stat-grid">
          <div class="stat-cell">
            <div class="stat-cell-icon">
              <FileText :size="16" />
            </div>
            <div class="sv">{{ wikiStore.stats.total_entries || 0 }}</div>
            <div class="sl">总条目</div>
          </div>
          <div class="stat-cell">
            <div class="stat-cell-icon warning">
              <AlertCircle :size="16" />
            </div>
            <div class="sv">{{ wikiStore.stats.orphan_count || 0 }}</div>
            <div class="sl">孤立条目</div>
          </div>
          <div class="stat-cell">
            <div class="stat-cell-icon error">
              <Zap :size="16" />
            </div>
            <div class="sv">{{ wikiStore.stats.contradiction_count || 0 }}</div>
            <div class="sl">矛盾对</div>
          </div>
          <div class="stat-cell">
            <div class="stat-cell-icon muted">
              <Link2 :size="16" />
            </div>
            <div class="sv">{{ wikiStore.stats.broken_link_count || 0 }}</div>
            <div class="sl">断链</div>
          </div>
          <div class="stat-cell">
            <div class="stat-cell-icon">
              <GitBranch :size="16" />
            </div>
            <div class="sv">{{ wikiStore.stats.graph_node_count || 0 }}</div>
            <div class="sl">图节点</div>
          </div>
          <div class="stat-cell">
            <div class="stat-cell-icon success">
              <Star :size="16" />
            </div>
            <div class="sv">{{ formatQuality(wikiStore.stats.avg_quality_score) }}</div>
            <div class="sl">平均质量分</div>
          </div>
        </div>

        <div v-else class="stats-loading">
          <span class="dots-loading"></span>
          <span>正在加载统计数据…</span>
        </div>

        <div class="stats-footer">
          <button class="btn btn-primary" @click="emit('close')">
            <Check :size="14" /> 完成
          </button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup>
import { watch } from 'vue'
import { useWikiStore } from '@/stores/wiki'
import { formatQuality } from '@/utils/format'
import { X, BarChart2, FileText, AlertCircle, Zap, Link2, GitBranch, Star, Check } from 'lucide-vue-next'

const props = defineProps({ open: Boolean })
const emit = defineEmits(['close'])

const wikiStore = useWikiStore()

watch(() => props.open, (val) => {
  if (val) wikiStore.loadStats()
})
</script>