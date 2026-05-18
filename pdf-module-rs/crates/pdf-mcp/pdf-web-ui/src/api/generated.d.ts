/** API types aligned with pdf-mcp OpenAPI (see /api-docs/openapi.json). */
export interface ExtractionHealth {
  backends: string[]
  vlm_configured: boolean
  default_method: string
}

export interface HealthReport {
  total_entries: number
  orphan_count: number
  contradiction_count: number
  graph_nodes: number
  graph_edges: number
  avg_quality_score: string
  extraction?: ExtractionHealth
}

export interface IndexRebuildResponse {
  status: string
  fulltext_entries_indexed: number
  graph_nodes: number
  graph_edges: number
  vector_entries_indexed: number
}
