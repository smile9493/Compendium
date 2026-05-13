<template>
  <div class="prose" v-html="html"></div>
</template>

<script setup>
import { computed } from 'vue'
import { useWikiStore } from '@/stores/wiki'
import { marked } from 'marked'
import hljs from 'highlight.js/lib/core'

// Register common languages
import javascript from 'highlight.js/lib/languages/javascript'
import typescript from 'highlight.js/lib/languages/typescript'
import python from 'highlight.js/lib/languages/python'
import rust from 'highlight.js/lib/languages/rust'
import json from 'highlight.js/lib/languages/json'
import bash from 'highlight.js/lib/languages/bash'
import xml from 'highlight.js/lib/languages/xml'
import css from 'highlight.js/lib/languages/css'

hljs.registerLanguage('javascript', javascript)
hljs.registerLanguage('typescript', typescript)
hljs.registerLanguage('python', python)
hljs.registerLanguage('rust', rust)
hljs.registerLanguage('json', json)
hljs.registerLanguage('bash', bash)
hljs.registerLanguage('shell', bash)
hljs.registerLanguage('xml', xml)
hljs.registerLanguage('css', css)

// Configure marked
marked.setOptions({
  breaks: true,
  gfm: true,
  highlight: function (code, lang) {
    if (lang && hljs.getLanguage(lang)) {
      try {
        return hljs.highlight(code, { language: lang }).value
      } catch (e) {
        return code
      }
    }
    return code
  },
})

const props = defineProps({
  markdown: { type: String, default: '' },
})

const wikiStore = useWikiStore()

const html = computed(() => {
  if (!props.markdown) return ''
  let rendered = marked.parse(props.markdown)

  // Convert wiki links [[path]] to clickable links
  rendered = rendered.replace(/\[\[([^\]]+)\]\]/g, (match, path) => {
    const title = path.split('/').pop()?.replace('.md', '') || path
    return `<a class="wikilink" data-path="${path}">${title}</a>`
  })

  return rendered
})

function handleWikilinkClick(e) {
  const link = e.target.closest('.wikilink')
  if (link) {
    e.preventDefault()
    const path = link.dataset.path
    if (path) {
      wikiStore.navigateTo(path)
    }
  }
}

// Use event delegation on the component root
import { onMounted, onBeforeUnmount, getCurrentInstance } from 'vue'
const instance = getCurrentInstance()
onMounted(() => {
  instance?.vnode.el?.addEventListener('click', handleWikilinkClick)
})
onBeforeUnmount(() => {
  instance?.vnode.el?.removeEventListener('click', handleWikilinkClick)
})
</script>
