<template>
  <div class="content-inner">
    <div class="empty-entry">
      <div class="icon">📖</div>
      <h2>知识编译展览</h2>
      <p>从左侧目录选择条目浏览知识编译成果</p>
      <p style="font-size:0.75rem;color:var(--text-muted);margin-top:0;">按 <span class="kbd">/</span> 快速搜索知识库</p>
    </div>
  </div>
</template>

<script setup>
import { onMounted } from 'vue'

onMounted(() => {
  document.title = 'Compendium 知识库'
})
</script>