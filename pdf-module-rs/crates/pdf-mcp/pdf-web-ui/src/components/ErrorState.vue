<template>
  <div class="empty-entry">
    <div class="icon">{{ icon }}</div>
    <h2>{{ title }}</h2>
    <p>{{ message }}</p>
    <button v-if="retryable" class="btn" @click="$emit('retry')">重试</button>
  </div>
</template>

<script setup>
defineProps({
  icon: { type: String, default: '⚠️' },
  title: { type: String, default: '出错了' },
  message: { type: String, default: '' },
  retryable: { type: Boolean, default: false },
})

defineEmits(['retry'])
</script>