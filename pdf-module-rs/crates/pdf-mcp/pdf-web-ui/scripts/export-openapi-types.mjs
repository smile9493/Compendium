#!/usr/bin/env node
/**
 * Generates TypeScript types from the Rust OpenAPI test fixture.
 *
 * Refresh fixture:
 *   cargo test -p pdf-mcp api_doc::tests::write_openapi_fixture -- --ignored
 */
import { readFileSync, writeFileSync, existsSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const root = dirname(fileURLToPath(import.meta.url))
const fixture = join(root, '../../tests/fixtures/openapi.json')
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

if (!existsSync(fixture)) {
  writeFileSync(out, fallback)
  console.warn('OpenAPI fixture missing; wrote fallback generated.d.ts')
  process.exit(0)
}

const uiRoot = join(root, '..')
const localBin = join(uiRoot, 'node_modules', 'openapi-typescript', 'bin', 'cli.js')
const cmd = existsSync(localBin) ? process.execPath : 'npx'
const args = existsSync(localBin)
  ? [localBin, fixture, '-o', out]
  : ['openapi-typescript', fixture, '-o', out]

const result = spawnSync(cmd, args, { cwd: uiRoot, stdio: 'inherit', shell: false })

if (result.status !== 0) {
  console.warn('openapi-typescript unavailable; keeping committed generated.d.ts in sync manually')
  process.exit(0)
}

console.log(`Wrote ${out} from ${fixture}`)
