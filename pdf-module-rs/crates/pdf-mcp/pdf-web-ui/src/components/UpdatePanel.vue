<template>
  <div class="update-panel">
    <!-- Current version display -->
    <div class="update-version-card">
      <div class="update-version-header">
        <span class="update-version-label">{{ t('update.currentVersion') }}</span>
        <span class="update-version-value mono">{{ store.currentVersion?.version || '…' }}</span>
      </div>
      <div class="update-version-meta" v-if="store.currentVersion">
        <span class="update-meta-item">
          Semver {{ store.currentVersion.semver }}
        </span>
        <span class="update-meta-item">
          {{ t('update.deploymentMode') }}:
          <span class="status-badge" :class="deploymentClass">
            {{ deploymentLabel }}
          </span>
        </span>
      </div>
      <div class="update-version-meta" v-if="store.checkedAt">
        <span class="update-meta-item update-checked-at">
          {{ t('update.lastChecked') }}: {{ formatTime(store.checkedAt) }}
        </span>
      </div>
    </div>

    <!-- Check button -->
    <div class="update-actions">
      <button
        class="btn btn-primary"
        :disabled="store.checking"
        @click="handleCheck"
      >
        <RefreshCw :size="14" :class="{ 'spin': store.checking }" />
        {{ store.checking ? t('update.checking') : t('update.checkButton') }}
      </button>
    </div>

    <!-- Error state -->
    <div v-if="store.error && !store.updateAvailable" class="update-error">
      <AlertCircle :size="14" />
      {{ store.error }}
    </div>

    <!-- Up to date -->
    <div v-if="!store.updateAvailable && store.checkedAt && !store.error" class="update-status update-up-to-date">
      <CheckCircle :size="16" />
      <span>{{ t('update.upToDate') }}</span>
    </div>

    <!-- Update available -->
    <div v-if="store.updateAvailable && store.latestVersion" class="update-available-section">
      <div class="update-available-header">
        <ArrowUpCircle :size="18" />
        <span class="update-available-text">{{ t('update.updateAvailable') }}</span>
      </div>

      <div class="update-new-version">
        <span class="update-version-tag mono">{{ store.latestVersion }}</span>
        <a
          v-if="store.releaseUrl"
          :href="store.releaseUrl"
          target="_blank"
          rel="noopener noreferrer"
          class="update-release-link"
        >
          <ExternalLink :size="12" />
          GitHub
        </a>
      </div>

      <!-- Release notes -->
      <div v-if="store.releaseNotes" class="update-release-notes">
        <div class="update-release-notes-label">{{ t('update.releaseNotes') }}</div>
        <div class="update-release-notes-body" v-html="renderMarkdown(store.releaseNotes)"></div>
      </div>

      <!-- Download button -->
      <div class="update-actions">
        <button
          class="btn btn-primary"
          :disabled="store.downloading || store.prepareStatus === 'ready'"
          @click="handlePrepare"
        >
          <Download :size="14" />
          {{ downloadButtonText }}
        </button>
      </div>

      <!-- Downloading progress -->
      <div v-if="store.prepareStatus === 'downloading'" class="update-progress">
        <div class="update-progress-bar">
          <div
            class="update-progress-fill"
            :style="{ width: store.downloadProgress + '%' }"
          ></div>
        </div>
        <span class="update-progress-text">{{ t('update.downloading') }}… {{ store.downloadProgress }}%</span>
      </div>

      <!-- Ready state -->
      <div v-if="store.prepareStatus === 'ready'" class="update-status update-ready">
        <CheckCircle :size="16" />
        <div class="update-ready-body">
          <div class="update-ready-title">{{ t('update.ready') }}</div>
          <div class="update-ready-instructions" v-if="store.prepareMessage">
            <code class="mono">{{ store.prepareMessage }}</code>
          </div>
          <div class="update-ready-hint" v-if="isDocker">
            {{ t('update.readyDocker') }}
          </div>
          <div class="update-ready-hint" v-else>
            {{ t('update.readyNative') }}
          </div>
        </div>
      </div>

      <!-- Error in prepare -->
      <div v-if="store.prepareStatus === 'error'" class="update-error">
        <AlertCircle :size="14" />
        {{ store.prepareMessage || t('update.error') }}
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useUpdateStore } from '@/stores/update'
import { marked } from 'marked'
import { RefreshCw, ArrowUpCircle, Download, CheckCircle, AlertCircle, ExternalLink } from 'lucide-vue-next'

const { t } = useI18n()
const store = useUpdateStore()

const isDocker = computed(() => store.currentVersion?.deployment_mode === 'docker')

const deploymentClass = computed(() => {
  if (isDocker.value) return 'status-ok'
  return ''
})

const deploymentLabel = computed(() => {
  const mode = store.currentVersion?.deployment_mode
  if (mode === 'docker') return 'Docker'
  if (mode === 'native') return 'Native'
  return 'Unknown'
})

const downloadButtonText = computed(() => {
  if (store.downloading) return t('update.downloading')
  return t('update.downloadButton')
})

function formatTime(iso) {
  if (!iso) return ''
  try {
    const d = new Date(iso)
    return d.toLocaleString()
  } catch {
    return iso
  }
}

function renderMarkdown(md) {
  if (!md) return ''
  try {
    return marked.parse(md)
  } catch {
    return md
  }
}

async function handleCheck() {
  try {
    await store.checkForUpdates()
  } catch {
    // error already stored in store.error
  }
}

async function handlePrepare() {
  try {
    await store.prepareUpdate()
  } catch {
    // error already stored in store
  }
}
</script>
