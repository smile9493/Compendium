<template>
  <Transition name="fade">
    <div v-if="open" class="overlay open" @click.self="$emit('close')">
      <div class="dialog settings-dialog">
        <button class="dialog-close" @click="$emit('close')">&times;</button>
        <h2>⚙️ 设置</h2>
        <div class="settings-tabs">
          <button
            v-for="tab in tabs"
            :key="tab.id"
            class="settings-tab"
            :class="{ active: activeTab === tab.id }"
            @click="activeTab = tab.id"
          >{{ tab.label }}</button>
        </div>
        <div class="settings-content">
          <!-- Server Config -->
          <div v-if="activeTab === 'config'">
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
            <button class="btn btn-primary" @click="saveVlmConfig" :disabled="configStore.saving">
              {{ configStore.saving ? '保存中…' : '保存 VLM 配置' }}
            </button>
            <div style="margin-top:16px;">
              <h3 style="font-size:0.8125rem;color:var(--text-muted);margin-bottom:8px;">运行时配置</h3>
              <table class="config-table">
                <thead><tr><th>键</th><th>值</th><th>操作</th></tr></thead>
                <tbody>
                  <tr v-for="(v, k) in configStore.configData" :key="k">
                    <td class="mono">{{ k }}</td>
                    <td class="mono">{{ String(v).slice(0, 60) }}</td>
                    <td><button class="btn btn-sm btn-danger" @click="configStore.deleteConfig(k)">删除</button></td>
                  </tr>
                </tbody>
              </table>
              <div class="form-row" style="margin-top:12px;">
                <div class="form-group">
                  <input type="text" v-model="newKey" placeholder="键名" />
                </div>
                <div class="form-group">
                  <input type="text" v-model="newValue" placeholder="值" />
                </div>
                <button class="btn btn-primary btn-sm" @click="addConfig">添加</button>
              </div>
            </div>
          </div>

          <!-- Health -->
          <div v-if="activeTab === 'health'">
            <div v-if="configStore.healthData" class="health-grid">
              <div class="health-item"><div class="hv">{{ configStore.healthData.total_entries || 0 }}</div><div class="hl">总条目</div></div>
              <div class="health-item"><div class="hv">{{ configStore.healthData.orphan_count || 0 }}</div><div class="hl">孤立条目</div></div>
              <div class="health-item"><div class="hv">{{ configStore.healthData.contradiction_count || 0 }}</div><div class="hl">矛盾对</div></div>
              <div class="health-item"><div class="hv">{{ configStore.healthData.graph_nodes || 0 }}</div><div class="hl">图节点</div></div>
              <div class="health-item"><div class="hv">{{ configStore.healthData.graph_edges || 0 }}</div><div class="hl">图边</div></div>
              <div class="health-item"><div class="hv">{{ formatQuality(configStore.healthData.avg_quality_score) }}</div><div class="hl">质量分</div></div>
            </div>
            <div v-else class="loading-placeholder">加载中…</div>
            <div style="margin-top:16px;display:flex;gap:8px;flex-wrap:wrap;">
              <button class="btn btn-primary" :disabled="configStore.loading" @click="rebuildIndex">
                {{ configStore.loading ? '重建中…' : '重建索引' }}
              </button>
              <button class="btn" @click="configStore.loadHealth(); configStore.loadCompileStatus();">
                刷新
              </button>
            </div>
          </div>

          <!-- Compile Queue -->
          <div v-if="activeTab === 'compile'">
            <div v-if="configStore.compileStatus" class="compile-list">
              <div class="compile-item">
                <span class="ci-status">
                  <span v-if="configStore.compileStatus.running" class="status-badge status-warn">运行中</span>
                  <span v-else-if="configStore.compileStatus.last_outcome === 'success'" class="status-badge status-ok">成功</span>
                  <span v-else class="status-badge status-err">{{ configStore.compileStatus.last_outcome || '未知' }}</span>
                </span>
                <span class="ci-name">最后编译</span>
                <span class="ci-time">{{ configStore.compileStatus.last_finished || configStore.compileStatus.last_started || '从未' }}</span>
              </div>
            </div>
            <div v-else class="loading-placeholder">加载中…</div>
            <div style="margin-top:16px;">
              <h3 style="font-size:0.8125rem;color:var(--text-muted);margin-bottom:8px;">上传 PDF 触发编译</h3>
              <div class="form-row">
                <input type="file" ref="fileInput" accept=".pdf" @change="uploadFile" />
              </div>
            </div>
          </div>

          <!-- About -->
          <div v-if="activeTab === 'about'">
            <div class="form-group">
              <label>项目</label>
              <p style="color:var(--text);">rsut-pdf-mcp — 知识编译引擎</p>
            </div>
            <div class="form-group">
              <label>架构模式</label>
              <p style="color:var(--text);font-family:var(--mono);font-size:0.75rem;">Breakwater Architecture</p>
            </div>
            <div class="form-group">
              <label>许可证</label>
              <p style="color:var(--text);">MIT</p>
            </div>
            <div class="form-group">
              <label>技术栈</label>
              <p style="color:var(--text-secondary);font-size:0.75rem;">Rust + Axum + Vue3 + Pinia</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup>
import { ref, watch } from 'vue'
import { useConfigStore } from '@/stores/config'
import { api } from '@/api'

defineProps({ open: Boolean })
defineEmits(['close'])

const configStore = useConfigStore()

const activeTab = ref('config')
const tabs = [
  { id: 'config', label: '服务器配置' },
  { id: 'health', label: '知识库健康' },
  { id: 'compile', label: '编译队列' },
  { id: 'about', label: '关于' },
]

const vlmModel = ref('')
const vlmApiKey = ref('')
const vlmEndpoint = ref('')
const newKey = ref('')
const newValue = ref('')
const fileInput = ref(null)

watch(() => open, async (val) => {
  if (val) {
    await configStore.loadConfig()
    await configStore.loadHealth()
    await configStore.loadCompileStatus()
    vlmModel.value = configStore.configData.vlm_model || ''
    vlmApiKey.value = configStore.configData.vlm_api_key || ''
    vlmEndpoint.value = configStore.configData.vlm_endpoint || ''
  }
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
  const result = await configStore.triggerRebuild()
  if (result) {
    configStore.loadHealth()
  }
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

function formatQuality(score) {
  if (score == null) return '-'
  if (typeof score === 'string') return score
  return `${(score * 100).toFixed(0)}%`
}
</script>
