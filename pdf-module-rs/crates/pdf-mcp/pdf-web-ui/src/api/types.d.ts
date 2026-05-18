/** @typedef {object} RenderedEntry */
/**
 * @property {string} title
 * @property {string} domain
 * @property {string[]} tags
 * @property {string} level
 * @property {number} quality_score
 * @property {string} status
 * @property {number} version
 * @property {string} body_markdown
 * @property {string} [body_html] @deprecated Server no longer generates HTML; use body_markdown + MarkdownRenderer
 * @property {string[]} related
 * @property {string[]} contradictions
 * @property {string[]} backlinks
 */

/** @typedef {object} SearchHit */
/**
 * @property {string} path
 * @property {string} title
 * @property {string} domain
 * @property {number} score
 * @property {string} snippet
 * @property {number} [highlight_count] Display-only snippet highlight count (not Tantivy score)
 * @property {number} [match_count] @deprecated Use highlight_count
 */

/** @typedef {object} SearchMeta */
/**
 * @property {boolean} index_empty
 * @property {boolean} used_fallback
 * @property {string} mode
 */

/** @typedef {object} WikiTreeNode */
/**
 * @property {string} name
 * @property {string} path
 * @property {WikiTreeNode[]} children
 * @property {boolean} is_entry
 * @property {string} [title]
 * @property {string} [domain]
 */

/** @typedef {'keyword' | 'semantic' | 'hybrid'} SearchMode */

/** @typedef {object} CompileStatusRecord */
/**
 * @property {boolean} running
 * @property {string} [last_started]
 * @property {string} [last_finished]
 * @property {string} [last_outcome]
 * @property {string} message
 * @property {CompileHistoryEntry[]} history
 * @property {QualitySnapshot} [quality_snapshot]
 */

/** @typedef {object} CompileHistoryEntry */
/**
 * @property {string} started_at
 * @property {string} finished_at
 * @property {number} duration_ms
 * @property {string} outcome
 * @property {number} entries_compiled
 * @property {number} entries_skipped
 */

/** @typedef {object} QualitySnapshot */
/**
 * @property {string} [scanned_at]
 * @property {number} issues_count
 * @property {number} orphan_count
 * @property {number} contradiction_pairs
 * @property {number} drift_pairs
 * @property {QualityIssueBrief[]} top_issues
 */

/** @typedef {object} QualityIssueBrief */
/**
 * @property {string} severity
 * @property {string} entry_path
 * @property {string} message
 */

export {}
