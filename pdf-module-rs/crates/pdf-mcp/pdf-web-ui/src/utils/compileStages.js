/** @type {Record<string, string>} */
export const COMPILE_STAGE_LABELS = {
  extract: '提取',
  prompt_gen: 'Prompt',
  agent_wiki: 'Agent 写 wiki',
  index_rebuild: '索引重建',
  quality_gate: '质量门禁',
}

/** @param {string} stage */
export function stageLabel(stage) {
  return COMPILE_STAGE_LABELS[stage] || stage
}
