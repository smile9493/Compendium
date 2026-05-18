<template>
  <div v-if="stages.length" class="compile-stages">
    <div
      v-for="stage in stages"
      :key="stageKey(stage)"
      class="compile-stage-row"
      :class="'stage-' + (stage.status || 'pending')"
    >
      <span class="stage-name">{{ label(stage) }}</span>
      <span class="stage-status">{{ stage.status }}</span>
      <span v-if="stage.duration_ms" class="stage-dur">{{ stage.duration_ms }}ms</span>
    </div>
  </div>
</template>

<script setup>
import { stageLabel } from '@/utils/compileStages'

defineProps({
  stages: {
    type: Array,
    default: () => [],
  },
})

function stageKey(stage) {
  return typeof stage.stage === 'string' ? stage.stage : String(stage.stage)
}

function label(stage) {
  return stageLabel(stage.stage)
}
</script>
