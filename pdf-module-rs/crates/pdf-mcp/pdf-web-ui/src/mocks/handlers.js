import { http, HttpResponse } from 'msw'

const sampleStages = [
  { stage: 'extract', status: 'succeeded', duration_ms: 80 },
  { stage: 'agent_wiki', status: 'running' },
]

export const handlers = [
  http.get('/api/v1/workspaces', () =>
    HttpResponse.json({
      workspaces: [{ id: 'default', name: 'Default', path: '/tmp/kb', active: true }],
      active_kb_id: 'default',
    })
  ),
  http.get('/api/health', () =>
    HttpResponse.json({
      total_entries: 3,
      orphan_count: 0,
      contradiction_count: 0,
      graph_nodes: 5,
      graph_edges: 4,
      avg_quality_score: '88.0%',
      extraction: {
        backends: ['pdfium'],
        vlm_configured: false,
        default_method: 'pdfium',
      },
    })
  ),
  http.get('/api/compile/status', () =>
    HttpResponse.json({
      running: true,
      pipeline_status: 'running',
      job: { stages: sampleStages },
      history: [],
      message: '',
    })
  ),
  http.get('/api/share/:token/wiki/entries/*', () =>
    HttpResponse.json({
      entry: {
        title: 'Shared',
        body_markdown: '# Shared\n\nRead-only body.',
        domain: 'IT',
        tags: [],
      },
    })
  ),
  http.get('/api/wiki/search', ({ request }) => {
    const q = new URL(request.url).searchParams.get('q')
    return HttpResponse.json({
      results: [],
      meta: { index_empty: true, used_fallback: false, mode: 'hybrid' },
      total: 0,
      query: q,
    })
  }),
]
