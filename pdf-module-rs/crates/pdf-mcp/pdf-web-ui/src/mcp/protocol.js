/** MCP UI extension protocol constants (see doc/MCP_UI_EXTENSION.md). */

export const MCP_UI_PROTOCOL_VERSION = 1

export const MCP_UI_MESSAGE_TYPES = {
  ASK_AI: 'mcp-ask-ai',
  COMPILE_STATUS: 'mcp-compile-status',
}

export const MCP_UI_SOURCE_WIKI_BROWSER = 'wiki-browser'

/**
 * @param {unknown} data
 * @param {string} expectedOrigin
 * @returns {boolean}
 */
export function isValidMcpUiMessage(data, expectedOrigin) {
  if (!data || typeof data !== 'object') return false
  if (data.v !== MCP_UI_PROTOCOL_VERSION) return false
  if (typeof data.type !== 'string' || typeof data.source !== 'string') return false
  if (expectedOrigin && typeof expectedOrigin === 'string' && expectedOrigin.length > 0) {
    // validated by caller via event.origin
  }
  return true
}

/**
 * @param {string} origin
 * @returns {string}
 */
export function mcpTargetOrigin(origin) {
  return origin || window.location.origin
}
