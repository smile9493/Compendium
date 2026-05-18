<template>
  <div v-if="visible" class="ops-banner" role="status">
    <AlertTriangle :size="16" />
    <span>{{ t('ops.indexEmpty') }}</span>
    <button type="button" class="btn btn-sm" @click="onRebuild">{{ t('ops.rebuild') }}</button>
    <button type="button" class="ops-banner-dismiss" @click="dismissed = true" aria-label="关闭">
      <X :size="14" />
    </button>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { AlertTriangle, X } from 'lucide-vue-next'
import { useSearchStore } from '@/stores/search'
import { useConfigStore } from '@/stores/config'
import { useWikiStore } from '@/stores/wiki'

const { t } = useI18n()
const searchStore = useSearchStore()
const configStore = useConfigStore()
const wikiStore = useWikiStore()
const dismissed = ref(false)

const visible = computed(
  () =>
    !dismissed.value &&
    searchStore.searchMeta?.index_empty === true &&
    searchStore.open
)

async function onRebuild() {
  try {
    await configStore.triggerRebuild()
    dismissed.value = true
    wikiStore.clearEntryCache()
    await wikiStore.loadTree()
  } catch (e) {
    console.error('Index rebuild failed', e)
  }
}
</script>
