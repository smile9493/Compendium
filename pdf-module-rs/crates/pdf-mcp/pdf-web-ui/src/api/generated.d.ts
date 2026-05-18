/**
 * Generated from pdf-mcp/tests/fixtures/openapi.json
 * Run: cargo test -p pdf-mcp api_doc::tests::write_openapi_fixture -- --ignored && npm run generate:api
 */
export interface paths {
  '/api/health': {
    get: {
      parameters?: { query?: { kb_id?: string } }
      responses: {
        200: { content: { 'application/json': components['schemas']['HealthReportHttp'] } }
        500: { content: { 'application/json': components['schemas']['ErrorBody'] } }
      }
    }
  }
  '/api/compile/status': {
    get: {
      parameters?: { query?: { kb_id?: string } }
      responses: { 200: { description: string } }
    }
  }
  '/api/index/rebuild': {
    post: {
      parameters?: { query?: { kb_id?: string } }
      responses: {
        200: { content: { 'application/json': components['schemas']['IndexRebuildHttp'] } }
        500: { content: { 'application/json': components['schemas']['ErrorBody'] } }
      }
    }
  }
  '/api/index/status': {
    get: {
      parameters?: { query?: { kb_id?: string } }
      responses: { 200: { description: string } }
    }
  }
}

export interface components {
  schemas: {
    ErrorBody: { error: string }
    ExtractionHealthHttp: {
      backends: string[]
      vlm_configured: boolean
      default_method: string
    }
    HealthReportHttp: {
      total_entries: number
      orphan_count: number
      contradiction_count: number
      graph_nodes: number
      graph_edges: number
      avg_quality_score: string
      extraction?: components['schemas']['ExtractionHealthHttp'] | null
    }
    IndexRebuildHttp: {
      status: string
      fulltext_entries_indexed: number
      graph_nodes: number
      graph_edges: number
      vector_entries_indexed: number
    }
  }
}

export type HealthReportHttp = components['schemas']['HealthReportHttp']
export type IndexRebuildHttp = components['schemas']['IndexRebuildHttp']
