<template>
  <Transition name="fade">
    <div v-if="open" class="overlay open" @click.self="emit('close')">
      <div class="dialog settings-dialog">
        <button class="dialog-close" @click="emit('close')">
          <X :size="16" />
        </button>

        <div class="settings-header">
          <div class="settings-icon">
            <Settings :size="18" />
          </div>
          <div class="settings-title-wrap">
            <div class="settings-title">设置</div>
            <div class="settings-subtitle">系统配置与运行时管理</div>
          </div>
        </div>

        <div class="settings-tabs">
          <button
            v-for="tab in tabs"
            :key="tab.id"
            class="settings-tab"
            :class="{ active: activeTab === tab.id }"
            @click="activeTab = tab.id"
          >
            <component :is="tab.icon" :size="14" />
            {{ tab.label }}
          </button>
        </div>

        <div class="settings-body">
          <!-- Server Config -->
          <div v-if="activeTab === 'config'" class="settings-section">
            <div class="form-group">
              <label>VLM 模型</label>
              <select v-model="vlmModel">
                <option value="gpt-4o">GPT-4o</option>
                <option value="claude-3.5-sonnet">Claude 3.5 Sonnet</option>
                <option value="glm-4.6v">GLM-4.6v</option>
                <option value="">自定义</option>
              </select>
            </div>
            <div class="form-group">
              <label>API Key</label>
              <input type="password" v-model="vlmApiKey" placeholder="sk-..." />
            </div>
            <div class="form-group">
              <label>端点地址</label>
              <input type="text" v-model="vlmEndpoint" placeholder="https://api.openai.com/v1" />
            </div>
            <div class="settings-actions">
              <button class="btn btn-primary" @click="saveVlmConfig" :disabled="configStore.saving">
                <Check :size="14" />
                {{ configStore.saving ? '保存中…' : '保存配置' }}
              </button>
            </div>

            <div class="settings-divider"></div>

            <div class="settings-section-title">运行时配置</div>
            <div class="config-table-wrap">
              <table class="config-table">
                <thead>
                  <tr>
                    <th>键</th>
                    <th>值</th>
                    <th>操作</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(v, k) in configStore.configData" :key="k">
                    <td class="mono key-cell">{{ k }}</td>
                    <td class="mono value-cell">{{ String(v).slice(0, 60) }}</td>
                    <td>
                      <button class="btn btn-sm btn-danger" @click="configStore.deleteConfig(k)">
                        <Trash2 :size="12" />
                      </button>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
            <div class="config-add-row">
              <div class="form-group">
                <input type="text" v-model="newKey" placeholder="键名" />
              </div>
              <div class="form-group">
                <input type="text" v-model="newValue" placeholder="值" />
              </div>
              <button class="btn btn-primary btn-sm" @click="addConfig">
                <Plus :size="14" /> 添加
              </button>
            </div>
          </div>

          <!-- Health -->
          <div v-if="activeTab === 'health'" class="settings-section">
            <div v-if="configStore.healthData" class="health-grid">
              <div class="health-item">
                <div class="hv">{{ configStore.healthData.total_entries || 0 }}</div>
                <div class="hl">总条目</div>
              </div>
              <div class="health-item">
                <div class="hv">{{ configStore.healthData.orphan_count || 0 }}</div>
                <div class="hl">孤立条目</div>
              </div>
              <div class="health-item">
                <div class="hv">{{ configStore.healthData.contradiction_count || 0 }}</div>
                <div class="hl">矛盾对</div>
              </div>
              <div class="health-item">
                <div class="hv">{{ configStore.healthData.graph_nodes || 0 }}</div>
                <div class="hl">图节点</div>
              </div>
              <div class="health-item">
                <div class="hv">{{ configStore.healthData.graph_edges || 0 }}</div>
                <div class="hl">图边</div>
              </div>
              <div class="health-item">
                <div class="hv">{{ formatQuality(configStore.healthData.avg_quality_score) }}</div>
                <div class="hl">质量分</div>
              </div>
            </div>
            <div
              v-if="configStore.healthData?.extraction"
              class="settings-section-title extraction-panel-title"
            >
              提取栈
            </div>
            <div v-if="configStore.healthData?.extraction" class="extraction-detail">
              <div class="extraction-row">
                <span class="extraction-label">默认方法</span>
                <span class="mono">{{ configStore.healthData.extraction.default_method }}</span>
              </div>
              <div class="extraction-row">
                <span class="extraction-label">VLM</span>
                <span
                  class="status-badge"
                  :class="configStore.healthData.extraction.vlm_configured ? 'status-ok' : ''"
                >
                  {{ configStore.healthData.extraction.vlm_configured ? '已配置' : '未配置' }}
                </span>
              </div>
              <div class="extraction-row">
                <span class="extraction-label">后端</span>
                <span class="mono extraction-backends">
                  {{ (configStore.healthData.extraction.backends || []).join(', ') }}
                </span>
              </div>
            </div>
            <div v-else-if="!configStore.healthData" class="settings-loading">
              <span class="dots-loading"></span>
              <span>正在加载健康数据…</span>
            </div>
            <div v-if="indexRebuildStats" class="index-rebuild-stats">
              <div class="settings-section-title">上次索引重建</div>
              <div class="mono index-stats-line">
                全文 {{ indexRebuildStats.fulltext_entries_indexed ?? 0 }} · 图
                {{ indexRebuildStats.graph_nodes ?? 0 }}/{{ indexRebuildStats.graph_edges ?? 0 }}
              </div>
            </div>
            <p v-if="indexRebuildMessage" class="compile-hint" :class="indexRebuildError ? 'compile-error' : ''">
              {{ indexRebuildMessage }}
            </p>
            <div class="settings-actions">
              <button class="btn btn-primary" :disabled="configStore.loading" @click="rebuildIndex">
                <RefreshCw :size="14" />
                {{ configStore.loading ? '重建中…' : '重建索引' }}
              </button>
              <button class="btn" @click="refreshHealth">
                <RotateCcw :size="14" /> 刷新
              </button>
            </div>
          </div>

          <!-- Compile Queue -->
          <div v-if="activeTab === 'compile'" class="settings-section">
            <div v-if="configStore.compileStatus" class="compile-status-card">
              <div class="compile-status-header">
                <span class="compile-status-label">编译状态</span>
                <span class="status-badge" :class="compileStatusClass">
                  {{ compileStatusText }}
                </span>
              </div>
              <div class="compile-status-time">
                {{ configStore.compileStatus.last_finished || configStore.compileStatus.last_started || '从未运行' }}
              </div>
              <CompileStageList :stages="compilePipelineStages" />
            </div>
            <div v-else class="settings-loading">
              <span class="dots-loading"></span>
              <span>正在加载编译状态…</span>
            </div>
            <div class="settings-divider"></div>
            <div class="settings-section-title">上传 PDF</div>
            <div class="upload-area">
              <input type="file" ref="fileInput" accept=".pdf" @change="uploadFile" class="upload-input" />
              <div class="upload-hint">拖拽或点击上传 PDF 文件以触发编译</div>
            </div>
          </div>

          <!-- Locale -->
          <div v-if="activeTab === 'locale'" class="settings-section">
            <div class="form-group">
              <label>{{ t('settings.language') }}</label>
              <select :value="locale" @change="onLocaleChange">
                <option value="zh-CN">{{ t('settings.langZh') }}</option>
                <option value="en-US">{{ t('settings.langEn') }}</option>
              </select>
            </div>
          </div>

          <!-- About -->
          <div v-if="activeTab === 'about'" class="settings-section">
            <div class="about-card">
              <div class="about-logo">
                <BookOpen :size="24" />
              </div>
              <div class="about-info">
                <div class="about-name">Compendium</div>
                <div class="about-desc">知识编译引擎 — 将 PDF 转化为结构化知识图谱</div>
              </div>
            </div>
            <div class="about-grid">
              <div class="about-item">
                <div class="about-item-label">架构模式</div>
                <div class="about-item-value mono">Breakwater Architecture</div>
              </div>
              <div class="about-item">
                <div class="about-item-label">技术栈</div>
                <div class="about-item-value mono">Rust + Axum + Vue3 + Pinia</div>
              </div>
              <div class="about-item">
                <div class="about-item-label">许可证</div>
                <div class="about-item-value">MIT</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup>
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { setLocale } from '@/i18n'
import { useConfigStore } from '@/stores/config'
import { useWikiStore } from '@/stores/wiki'
import { api } from '@/api'
import { formatQuality } from '@/utils/format'
import { X, Settings, Server, Activity, Upload, Info, Check, Trash2, Plus, RefreshCw, RotateCcw, BookOpen } from 'lucide-vue-next'
import CompileStageList from '@/components/CompileStageList.vue'

const props = defineProps({ open: Boolean })
const emit = defineEmits(['close'])

const configStore = useConfigStore()

const activeTab = ref('config')
const tabs = [
  { id: 'config', label: '服务器配置', icon: Server },
  { id: 'health', label: '知识库健康', icon: Activity },
  { id: 'compile', label: '编译队列', icon: Upload },
  { id: 'about', label: '关于', icon: Info },
  { id: 'locale', label: '语言', icon: Info },
]

const { t, locale } = useI18n()

const vlmModel = ref('')
const vlmApiKey = ref('')
const vlmEndpoint = ref('')
const newKey = ref('')
const newValue = ref('')
const fileInput = ref(null)
const indexRebuildStats = ref(null)
const indexRebuildMessage = ref('')
const indexRebuildError = ref(false)

const compilePipelineStages = computed(
  () => configStore.compileStatus?.job?.stages || []
)

const compileStatusClass = computed(() => {
  if (configStore.compileStatus?.running) return 'status-warn'
  if (configStore.compileStatus?.last_outcome === 'success') return 'status-ok'
  return 'status-err'
})

const compileStatusText = computed(() => {
  if (configStore.compileStatus?.running) return '运行中'
  if (configStore.compileStatus?.last_outcome === 'success') return '成功'
  return configStore.compileStatus?.last_outcome || '未知'
})

watch(() => props.open, async (val) => {
  if (val) {
    activeTab.value = 'config'
    vlmModel.value = ''
    vlmApiKey.value = ''
    vlmEndpoint.value = ''
    await configStore.loadConfig()
    vlmModel.value = configStore.configData.vlm_model || ''
    vlmApiKey.value = configStore.configData.vlm_api_key || ''
    vlmEndpoint.value = configStore.configData.vlm_endpoint || ''
  }
})

watch(activeTab, (tab) => {
  if (tab === 'health' && !configStore.healthData) configStore.loadHealth()
  if (tab === 'compile' && !configStore.compileStatus) configStore.loadCompileStatus()
})

async function saveVlmConfig() {
  const updates = [{ key: 'vlm_model', value: vlmModel.value }]
  if (vlmApiKey.value) updates.push({ key: 'vlm_api_key', value: vlmApiKey.value })
  if (vlmEndpoint.value) updates.push({ key: 'vlm_endpoint', value: vlmEndpoint.value })
  for (const u of updates) {
    await configStore.updateConfig(u.key, u.value)
  }
}

async function addConfig() {
  if (!newKey.value.trim()) return
  await configStore.updateConfig(newKey.value.trim(), newValue.value.trim())
  newKey.value = ''
  newValue.value = ''
}

async function rebuildIndex() {
  const wikiStore = useWikiStore()
  indexRebuildMessage.value = ''
  indexRebuildError.value = false
  try {
    const result = await configStore.triggerRebuild()
    indexRebuildStats.value = result
    indexRebuildMessage.value = '索引重建成功'
    wikiStore.clearEntryCache()
    await wikiStore.loadTree()
    await configStore.loadHealth()
  } catch (err) {
    indexRebuildError.value = true
    indexRebuildMessage.value = err?.message || '索引重建失败'
  }
}

async function refreshHealth() {
  await configStore.loadHealth()
  await configStore.loadCompileStatus()
}

function onLocaleChange(e) {
  setLocale(e.target.value)
}

async function uploadFile(e) {
  const file = e.target.files?.[0]
  if (!file) return
  try {
    await api.uploadPdf(file)
    configStore.loadCompileStatus()
  } catch (err) {
    console.error('Upload failed', err)
  }
}
</script>