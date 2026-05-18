import { describe, expect, it } from 'vitest'
import {
  MCP_UI_MESSAGE_TYPES,
  MCP_UI_PROTOCOL_VERSION,
  MCP_UI_SOURCE_WIKI_BROWSER,
  isValidMcpUiMessage,
  mcpTargetOrigin,
} from './protocol'

describe('mcp protocol', () => {
  it('validates ask-ai envelope', () => {
    const msg = {
      v: MCP_UI_PROTOCOL_VERSION,
      type: MCP_UI_MESSAGE_TYPES.ASK_AI,
      source: MCP_UI_SOURCE_WIKI_BROWSER,
      title: 'T',
      path: 'a.md',
      excerpt: 'x',
    }
    expect(isValidMcpUiMessage(msg, 'https://host.example')).toBe(true)
  })

  it('rejects wrong version', () => {
    expect(isValidMcpUiMessage({ v: 0, type: 'mcp-ask-ai', source: 'x' }, '')).toBe(false)
  })

  it('mcpTargetOrigin falls back to location', () => {
    expect(mcpTargetOrigin('https://parent.test')).toBe('https://parent.test')
  })

  it('defines compile-status message type', () => {
    expect(MCP_UI_MESSAGE_TYPES.COMPILE_STATUS).toBe('mcp-compile-status')
  })
})
