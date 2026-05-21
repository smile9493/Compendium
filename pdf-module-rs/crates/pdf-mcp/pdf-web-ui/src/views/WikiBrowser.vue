<template>
  <div class="content-inner">
    <div class="empty-entry">
      <div class="icon" aria-hidden="true">📖</div>
      <h2>{{ t('welcome.title') }}</h2>
      <p>{{ t('welcome.subtitle') }}</p>
      <p class="search-hint">
        {{ t('welcome.searchHint') }}
        <span class="kbd">/</span>
      </p>

      <nav v-if="recentPaths.length" class="welcome-recent" :aria-label="t('welcome.recentTitle')">
        <h3 class="welcome-recent-title">{{ t('welcome.recentTitle') }}</h3>
        <ul class="welcome-recent-list">
          <li v-for="item in recentPaths" :key="item.path">
            <button type="button" class="welcome-recent-link" @click="openEntry(item.path)">
              {{ item.label }}
            </button>
          </li>
        </ul>
      </nav>
    </div>
  </div>
</template>

<script setup>
import { computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useWikiStore } from '@/stores/wiki'
import { openEntry } from '@/composables/useWikiNavigation'
import { getRecentPaths, resolveRecentLabels } from '@/composables/useRecentEntries'

const { t } = useI18n()
const wikiStore = useWikiStore()

const recentPaths = computed(() =>
  resolveRecentLabels(getRecentPaths(), wikiStore.tree),
)

onMounted(() => {
  document.title = t('app.title')
})
</script>
