<template>
  <Transition name="drawer-slide">
    <aside v-if="compileStore.open" class="compile-drawer">
      <div class="compile-drawer-header">
        <div class="compile-drawer-title">
          <Hammer :size="18" />
          <span>{{ t('compile.console') }}</span>
        </div>
        <button class="header-btn icon-btn" @click="compileStore.closeDrawer()" v-tooltip="t('compile.close')">
          <X :size="16" />
        </button>
      </div>

      <div class="compile-tabs">
        <button
          v-for="tab in tabs"
          :key="tab.id"
          class="compile-tab"
          :class="{ active: compileStore.activeTab === tab.id }"
          @click="compileStore.activeTab = tab.id"
        >
          <component :is="tab.icon" :size="14" />
          {{ tab.label }}
        </button>
      </div>

      <div class="compile-drawer-body">
        <div v-if="compileStore.activeTab === 'trigger'" class="compile-section">
          <div class="compile-mode-row">
            <label class="compile-mode-label">
              <input type="radio" v-model="compileMode" value="single" />
              {{ t('compile.singleMode') }}
            </label>
            <label class="compile-mode-label">
              <input type="radio" v-model="compileMode" value="incremental" />
              {{ t('compile.incrementalMode') }}
            </label>
          </div>
          <div
            v-if="compileMode === 'single'"
            class="upload-area"
            @dragover.prevent
            @drop.prevent="onDrop"
          >
            <input type="file" ref="fileInput" accept=".pdf" class="upload-input" @change="onFile" />
            <div class="upload-hint">{{ t('compile.uploadHint') }}</div>
          </div>
          <p v-else class="compile-hint">{{ t('compile.incrementalHint') }}</p>
          <button
            class="btn btn-primary compile-start-btn"
            :disabled="compileStore.loading || compileStore.isRunning"
            @click="startCompile"
          >
            <Loader2 v-if="compileStore.loading" :size="14" class="spin" />
            <Play v-else :size="14" />
            {{
              compileStore.loading
                ? t('compile.processing')
                : compileMode === 'incremental'
                  ? t('compile.startIncremental')
                  : t('compile.uploadAndCompile')
            }}
          </button>
          <p v-if="compileStore.error" class="compile-error">{{ compileStore.error }}</p>
        </div>

        <div v-if="compileStore.activeTab === 'history'" class="compile-section">
          <div v-if="history.length" class="compile-list">
            <button
              v-for="(h, i) in history"
              :key="i"
              type="button"
              class="compile-item"
              @click="selectedHistory = h"
            >
              <span class="ci-status status-badge" :class="outcomeClass(h.outcome)">{{ h.outcome }}</span>
              <span class="ci-name">{{
                t('compile.entriesSummary', {
                  compiled: h.entries_compiled,
                  skipped: h.entries_skipped,
                })
              }}</span>
              <span class="ci-time">{{ formatTime(h.finished_at) }}</span>
            </button>
          </div>
          <p v-else class="compile-hint">{{ t('compile.noHistory') }}</p>
          <div v-if="selectedHistory" class="compile-detail">
            <div class="mono">{{ selectedHistory.message || compileStore.compileStatus?.message }}</div>
          </div>
        </div>

        <div v-if="compileStore.activeTab === 'status'" class="compile-section">
          <div class="compile-status-card">
            <div class="compile-status-header">
              <span class="compile-status-label">{{ t('compile.currentStatus') }}</span>
              <span class="status-badge" :class="statusClass">{{ compileStore.statusText }}</span>
            </div>
            <div class="compile-status-time">
              {{
                compileStore.compileStatus?.last_finished ||
                compileStore.compileStatus?.last_started ||
                t('compile.neverRun')
              }}
            </div>
            <div v-if="compileStore.isRunning" class="compile-progress-hint">
              <span class="dots-loading"></span>
              {{
                compileStore.pipelineStatus === 'awaiting_agent'
                  ? t('compile.awaitingAgent')
                  : t('compile.pollingHint')
              }}
            </div>

            <!-- Stage progress stepper -->
            <div v-if="compileStore.activeStages.length" class="stage-stepper">
              <div
                v-for="(stage, i) in compileStore.activeStages"
                :key="stage.id"
                class="stage-step"
                :class="[
                  'stage-step--' + stage.status,
                  { 'stage-step--active': stage.status === 'running' }
                ]"
              >
                <div v-if="i > 0" class="stage-step-connector" :class="{ done: stage.status === 'done' || stage.status === 'running' }"></div>
                <div class="stage-step-dot">
                  <Check v-if="stage.status === 'done'" :size="12" />
                  <Loader2 v-else-if="stage.status === 'running'" :size="12" class="spin" />
                  <span v-else class="stage-step-num">{{ i + 1 }}</span>
                </div>
                <div class="stage-step-label">{{ stageLabel(stage.id) }}</div>
                <div v-if="stage.durationMs" class="stage-step-dur">{{ stage.durationMs }}ms</div>
              </div>
            </div>

            <CompileStageList :stages="pipelineStages" />
          </div>
        </div>

        <div v-if="compileStore.activeTab === 'quality'" class="compile-section">
          <template v-if="compileStore.qualitySnapshot?.scanned_at">
            <div class="quality-summary-grid">
              <div class="health-item">
                <div class="hv">{{ compileStore.qualitySnapshot.issues_count }}</div>
                <div class="hl">{{ t('compile.issues') }}</div>
              </div>
              <div class="health-item">
                <div class="hv">{{ compileStore.qualitySnapshot.orphan_count }}</div>
                <div class="hl">{{ t('compile.orphans') }}</div>
              </div>
              <div class="health-item">
                <div class="hv">{{ compileStore.qualitySnapshot.contradiction_pairs }}</div>
                <div class="hl">{{ t('compile.contradictions') }}</div>
              </div>
              <div class="health-item">
                <div class="hv">{{ compileStore.qualitySnapshot.blocked_count ?? 0 }}</div>
                <div class="hl">{{ t('compile.blocked') }}</div>
              </div>
            </div>
            <div class="quality-issues-list">
              <button
                v-for="(issue, i) in compileStore.qualitySnapshot.top_issues"
                :key="i"
                type="button"
                class="quality-issue-row"
                @click="openIssue(issue.entry_path)"
              >
                <span class="issue-sev">{{ issue.severity }}</span>
                <span class="issue-path">{{ issue.entry_path }}</span>
              </button>
            </div>
          </template>
          <p v-else class="compile-hint">{{ t('compile.qualityAfterCompile') }}</p>
        </div>
      </div>

      <div
        v-if="compileStore.qualitySnapshot?.issues_count != null && compileStore.activeTab !== 'quality'"
        class="compile-quality-footer"
      >
        <button type="button" class="btn btn-sm" @click="compileStore.activeTab = 'quality'">
          {{
            t('compile.qualityFooter', {
              issues: compileStore.qualitySnapshot.issues_count,
              pairs: compileStore.qualitySnapshot.contradiction_pairs,
            })
          }}
        </button>
      </div>
    </aside>
  </Transition>
</template>

<script setup>
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useCompileStore } from '@/stores/compile'
import { openEntry } from '@/composables/useWikiNavigation'
import { Hammer, X, Upload, History, Activity, ShieldAlert, Play, Loader2, Check } from 'lucide-vue-next'
import CompileStageList from '@/components/CompileStageList.vue'
import { stageLabel } from '@/utils/compileStages'

const { t } = useI18n()
const compileStore = useCompileStore()
const compileMode = ref('single')
const fileInput = ref(null)
const pendingFile = ref(null)
const selectedHistory = ref(null)

const tabs = computed(() => [
  { id: 'trigger', label: t('compile.tabTrigger'), icon: Upload },
  { id: 'history', label: t('compile.tabHistory'), icon: History },
  { id: 'status', label: t('compile.tabStatus'), icon: Activity },
  { id: 'quality', label: t('compile.tabQuality'), icon: ShieldAlert },
])

const history = computed(() => compileStore.compileStatus?.history || [])

const pipelineStages = computed(
  () => compileStore.compileStatus?.job?.stages || []
)

const statusClass = computed(() => {
  if (compileStore.pipelineStatus === 'awaiting_agent') return 'status-warn'
  if (compileStore.isRunning) return 'status-warn'
  if (
    compileStore.pipelineStatus === 'completed' ||
    compileStore.compileStatus?.last_outcome === 'success'
  )
    return 'status-ok'
  if (compileStore.pipelineStatus === 'partial') return 'status-warn'
  return 'status-error'
})

function onFile(e) {
  const f = e.target.files?.[0]
  if (f) pendingFile.value = f
}

function onDrop(e) {
  const f = e.dataTransfer.files?.[0]
  if (f?.name?.toLowerCase().endsWith('.pdf')) pendingFile.value = f
}

async function startCompile() {
  if (compileMode.value === 'incremental') {
    await compileStore.triggerIncremental()
    return
  }
  const file = pendingFile.value || fileInput.value?.files?.[0]
  if (!file) {
    compileStore.error = t('compile.selectPdf')
    return
  }
  await compileStore.uploadAndCompile(file, 'single')
  pendingFile.value = null
}

function formatTime(iso) {
  if (!iso) return ''
  try {
    return new Date(iso).toLocaleString()
  } catch {
    return iso
  }
}

function outcomeClass(outcome) {
  if (outcome === 'success') return 'status-ok'
  if (outcome === 'error') return 'status-error'
  return ''
}

function openIssue(path) {
  if (path) {
    compileStore.closeDrawer()
    openEntry(path)
  }
}
</script>

<style scoped>
.compile-drawer {
  position: fixed;
  top: var(--header-height, 48px);
  right: 0;
  bottom: 0;
  width: min(460px, 92vw);
  z-index: 200;
  background: var(--surface);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  box-shadow: -4px 0 24px color-mix(in oklch, var(--text) 8%, transparent);
}
.compile-drawer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
}
.compile-drawer-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
}
.compile-tabs {
  display: flex;
  gap: 4px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
  flex-wrap: wrap;
}
.compile-tab {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  font-size: 0.8125rem;
  color: var(--text-muted);
  background: transparent;
  border: none;
  cursor: pointer;
}
.compile-tab.active {
  background: color-mix(in oklch, var(--primary) 12%, transparent);
  color: var(--primary);
}
.compile-drawer-body {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}
.compile-section { display: flex; flex-direction: column; gap: 12px; }
.compile-mode-row { display: flex; flex-direction: column; gap: 8px; }
.compile-mode-label { display: flex; align-items: center; gap: 8px; font-size: 0.875rem; }
.compile-hint { font-size: 0.8125rem; color: var(--text-muted); margin: 0; }
.compile-start-btn { width: 100%; justify-content: center; }
.compile-error { color: var(--error); font-size: 0.8125rem; margin: 0; }
.compile-progress-hint { display: flex; align-items: center; gap: 8px; font-size: 0.8125rem; color: var(--text-muted); margin-top: 8px; }
.compile-detail { margin-top: 12px; padding: 10px; background: var(--surface-2); border-radius: var(--radius-sm); font-size: 0.75rem; }
.compile-quality-footer {
  padding: 10px 16px;
  border-top: 1px solid var(--border);
}
.quality-summary-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
  margin-bottom: 12px;
}
.quality-issues-list { display: flex; flex-direction: column; gap: 4px; }
.quality-issue-row {
  display: flex;
  gap: 8px;
  text-align: left;
  padding: 8px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  background: var(--surface-2);
  cursor: pointer;
  font-size: 0.75rem;
}
.quality-issue-row:hover { border-color: var(--primary); }
.issue-sev { color: var(--text-muted); flex-shrink: 0; }
.issue-path { color: var(--text); overflow: hidden; text-overflow: ellipsis; }
.drawer-slide-enter-active,
.drawer-slide-leave-active { transition: transform 0.22s ease; }
.drawer-slide-enter-from,
.drawer-slide-leave-to { transform: translateX(100%); }
.spin { animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

/* ── Stage stepper ── */
.stage-stepper {
  display: flex;
  align-items: flex-start;
  gap: 0;
  padding: 12px 0 4px;
  overflow-x: auto;
}
.stage-step {
  display: flex;
  flex-direction: column;
  align-items: center;
  flex: 1;
  min-width: 0;
  position: relative;
}
.stage-step-connector {
  position: absolute;
  top: 10px;
  right: 50%;
  width: 100%;
  height: 2px;
  background: var(--border);
  z-index: 0;
}
.stage-step-connector.done {
  background: var(--primary);
}
.stage-step-dot {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.625rem;
  font-weight: 600;
  background: var(--surface-2);
  border: 2px solid var(--border);
  color: var(--text-muted);
  z-index: 1;
  flex-shrink: 0;
}
.stage-step--done .stage-step-dot {
  background: var(--primary);
  border-color: var(--primary);
  color: #fff;
}
.stage-step--running .stage-step-dot {
  background: var(--surface);
  border-color: var(--primary);
  color: var(--primary);
}
.stage-step--failed .stage-step-dot {
  background: var(--error);
  border-color: var(--error);
  color: #fff;
}
.stage-step-label {
  margin-top: 4px;
  font-size: 0.625rem;
  color: var(--text-muted);
  text-align: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100%;
}
.stage-step--active .stage-step-label {
  color: var(--primary);
  font-weight: 600;
}
.stage-step-dur {
  font-size: 0.5625rem;
  color: var(--text-muted);
  opacity: 0.7;
}
.stage-step-num {
  line-height: 1;
}
</style>
