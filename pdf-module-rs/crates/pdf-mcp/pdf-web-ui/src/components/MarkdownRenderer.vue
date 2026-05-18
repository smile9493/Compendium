<template>
  <div class="prose" v-html="html" @click="handleWikilinkClick"></div>
</template>

<script setup>
import { computed } from 'vue'
import { marked } from 'marked'
import { openEntry } from '@/composables/useWikiNavigation'
import { normalizeWikiPath } from '@/utils/path'
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

// Configure marked — disable raw HTML to prevent XSS
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

// Block raw HTML to prevent XSS — wiki content from PDF compilation is pure markdown
marked.use({
  renderer: {
    html(token) {
      return ''
    }
  }
})

const props = defineProps({
  markdown: { type: String, default: '' },
})

const WIKILINK_PATH = /^[\w\u4e00-\u9fff./_-]+$/

const html = computed(() => {
  if (!props.markdown) return ''
  let rendered = marked.parse(props.markdown)

  rendered = rendered.replace(/\[\[([^\]]+)\]\]/g, (_match, rawPath) => {
    const normalized = normalizeWikiPath(rawPath)
    if (!normalized || !WIKILINK_PATH.test(rawPath.trim())) {
      return rawPath
    }
    const title = normalized.split('/').pop()?.replace('.md', '') || normalized
    const esc = (s) => s.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;')
    return `<a class="wikilink" href="#" data-path="${esc(normalized)}">${esc(title)}</a>`
  })

  return rendered
})

function handleWikilinkClick(e) {
  const link = e.target.closest('.wikilink')
  if (link) {
    e.preventDefault()
    const path = link.dataset.path
    if (path) {
      openEntry(path)
    }
  }
}
</script>
