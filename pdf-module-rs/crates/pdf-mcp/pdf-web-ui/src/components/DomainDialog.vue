<template>
  <Transition name="fade">
    <div v-if="open" class="overlay open" @click.self="emit('close')">
      <div class="dialog domain-dialog">
        <button class="dialog-close" @click="emit('close')">
          <X :size="16" />
        </button>

        <div class="domain-header">
          <div class="domain-icon">
            <Tag :size="18" />
          </div>
          <div class="domain-title-wrap">
            <div class="domain-title">领域总览</div>
            <div class="domain-subtitle">{{ wikiStore.domainsFromTree.length }} 个领域分类</div>
          </div>
        </div>

        <div v-if="wikiStore.domainsFromTree.length" class="domain-cards">
          <div
            v-for="d in wikiStore.domainsFromTree"
            :key="d.domain"
            class="domain-card"
            :class="{ selected: wikiStore.domainFilter === d.domain }"
            @click="selectDomain(d.domain)"
          >
            <div class="domain-card-body">
              <div class="domain-card-left">
                <div class="domain-folder-icon">
                  <FolderOpen :size="14" />
                </div>
                <div class="domain-info">
                  <span class="dc-name">{{ d.domain }}</span>
                  <span class="dc-paths">{{ d.count }} 个条目</span>
                </div>
              </div>
              <div class="domain-card-actions">
                <span v-if="wikiStore.domainFilter === d.domain" class="domain-selected-badge">
                  <Check :size="11" />
                </span>
              </div>
            </div>
            <div class="domain-paths-preview">
              <span v-for="(p, i) in d.paths.slice(0, 5)" :key="i" class="dc-path-item">{{ p }}</span>
              <span v-if="d.paths.length > 5" class="dc-path-item more">+{{ d.paths.length - 5 }} 更多</span>
            </div>
          </div>
        </div>

        <div v-else class="domain-loading">
          <span class="dots-loading"></span>
          <span>正在加载领域数据…</span>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup>
import { useWikiStore } from '@/stores/wiki'
import { X, Tag, FolderOpen, Check } from 'lucide-vue-next'

const props = defineProps({ open: Boolean })
const emit = defineEmits(['close'])

const wikiStore = useWikiStore()

function selectDomain(domain) {
  wikiStore.setDomainFilter(domain)
}
</script>