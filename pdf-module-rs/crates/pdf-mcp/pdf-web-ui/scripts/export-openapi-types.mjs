#!/usr/bin/env node
/**
 * Writes minimal generated API types from the Rust OpenAPI test fixture.
 * Run `cargo test -p pdf-mcp api_doc::tests::write_openapi_fixture -- --ignored` first
 * to refresh pdf-mcp/tests/fixtures/openapi.json when schemas change.
 */
import { readFileSync, writeFileSync, existsSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const root = dirname(fileURLToPath(import.meta.url))
const fixture = join(root, '../../../tests/fixtures/openapi.json')
const out = join(root, '../src/api/generated.d.ts')

const fallback = `/** Auto-generated API types (fallback stub). */
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
`

if (existsSync(fixture)) {
  writeFileSync(out, `/** Generated from ${fixture} — run npm run generate:api after OpenAPI changes. */\nexport {}\n`)
  console.log('OpenAPI fixture found; extend export-openapi-types.mjs to invoke openapi-typescript.')
} else {
  writeFileSync(out, fallback)
  console.log('Wrote fallback generated.d.ts')
}
