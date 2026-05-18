import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'
import { mockTree, mockEntries } from './src/mock-data.js'

function createMockServer() {
  return {
    name: 'mock-server',
    configureServer(server) {
      console.log('[Mock] Server configured')

      server.middlewares.use('/api/wiki/tree', (req, res) => {
        console.log('[Mock] GET /api/wiki/tree')
        res.setHeader('Content-Type', 'application/json')
        res.end(JSON.stringify({ tree: mockTree }))
      })

      server.middlewares.use('/api/wiki/entries/', (req, res) => {
        const rawPath = req.url.replace('/api/wiki/entries/', '').split('?')[0]
        const decodedPath = decodeURIComponent(rawPath).replace(/^\//, '')
        console.log('[Mock] GET /api/wiki/entries/')
        console.log('  raw:', rawPath)
        console.log('  decoded:', decodedPath)
        const entry = mockEntries[decodedPath]
        if (entry) {
          if (!entry.body_markdown) {
            res.statusCode = 500
            res.end(JSON.stringify({ error: 'Mock entry missing body_markdown' }))
            return
          }
          res.setHeader('Content-Type', 'application/json')
          res.end(JSON.stringify({ entry }))
        } else {
          res.statusCode = 404
          res.end(JSON.stringify({ error: 'Entry not found: ' + decodedPath }))
        }
      })

      server.middlewares.use('/api/wiki/stats', (req, res) => {
        res.setHeader('Content-Type', 'application/json')
        res.end(JSON.stringify({
          total_entries: 10,
          orphan_count: 2,
          contradiction_count: 1,
          broken_link_count: 0,
          graph_node_count: 15,
          avg_quality_score: 0.88,
        }))
      })

      server.middlewares.use('/api/wiki/search', (req, res) => {
        const query = new URL(req.url, 'http://localhost').searchParams.get('q') || ''
        const results = []
        for (const [path, entry] of Object.entries(mockEntries)) {
          const titleMatch = entry.title.includes(query)
          const bodyMatch = entry.body_markdown.includes(query)
          if (titleMatch || bodyMatch) {
            results.push({
              path,
              title: entry.title,
              domain: entry.domain,
              score: query.length > 0 ? 8.5 : 0,
              snippet: entry.body_markdown.slice(0, 200) + '...',
              match_count: entry.body_markdown.split(query).length - 1,
            })
          }
        }
        res.setHeader('Content-Type', 'application/json')
        res.end(JSON.stringify({ results }))
      })

      server.middlewares.use('/api/wiki/graph/', (req, res) => {
        const path = req.url.replace('/api/wiki/graph/', '')
        const entry = mockEntries[decodeURIComponent(path)]
        if (entry && entry.related) {
          res.setHeader('Content-Type', 'application/json')
          const mermaid = 'graph TD\n  A["' + entry.title + '"]\n' +
            entry.related.map(r => {
              const e = mockEntries[r]
              return '  A --> B["' + (e?.title || r) + '"]'
            }).join('\n')
          res.end(JSON.stringify({ mermaid }))
        } else {
          res.end(JSON.stringify({ mermaid: '' }))
        }
      })
    },
  }
}

export default defineConfig({
  plugins: [vue(), createMockServer()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  base: '/app/',
  build: {
    target: 'es2020',
    reportCompressedSize: false,
    rollupOptions: {
      output: {
        manualChunks: {
          vendor: ['vue', 'vue-router', 'pinia'],
          markdown: ['marked', 'highlight.js'],
          mermaid: ['mermaid'],
        },
      },
    },
  },
  server: {
    port: 5173,
  },
})