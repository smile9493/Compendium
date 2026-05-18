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
 * @property {string} [body_html]
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
 * @property {number} [match_count]
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

export {}
