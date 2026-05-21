/**
 * Compendium Code Mode API — generated from pdf-mcp-contracts.
 *
 * Invoke via MCP tool `execute_compendium` with:
 *   calls: [{ method: "search_knowledge", args: { ... } }]
 */

declare namespace Compendium {
  type Args = Record<string, unknown>;


  // --- extract ---
  /** Extract plain text from a PDF file using pdfium engine */
  function extractText(args: Args): Promise<unknown>;
  /** Extract structured data (per-page text + bbox) from PDF */
  function extractStructured(args: Args): Promise<unknown>;
  /** Get the number of pages in a PDF file */
  function getPageCount(args: Args): Promise<unknown>;
  /** Search for keywords in a PDF file and return matches with page numbers and context */
  function searchKeywords(args: Args): Promise<unknown>;
  /** Extract PDF to server-side wiki raw/ (Karpathy paradigm) */
  function extrudeToServerWiki(args: Args): Promise<unknown>;
  /** Extract PDF and return markdown payload with compilation instructions for AI Agent */
  function extrudeToAgentPayload(args: Args): Promise<unknown>;

  // --- knowledge ---
  /** Initialize a new knowledge base from Karpathy-style templates (schema, index, log) */
  function initKnowledgeBase(args: Args): Promise<unknown>;
  /** Karpathy lint: orphans, broken links, contradictions, drift, missing concepts */
  function lintWiki(args: Args): Promise<unknown>;
  /** Write a query answer back to the wiki as an overview page */
  function archiveAnswer(args: Args): Promise<unknown>;
  /** Compile a PDF into the knowledge base (Karpathy compiler pattern) */
  function compileToWiki(args: Args): Promise<unknown>;
  /** Scan raw/ for changed PDFs and compile only those that need it */
  function incrementalCompile(args: Args): Promise<unknown>;
  /** On-demand PDF extraction for conversation context (not saved to wiki) */
  function microCompile(args: Args): Promise<unknown>;
  /** Identify clusters of related L1 entries for L2 synthesis */
  function aggregateEntries(args: Args): Promise<unknown>;
  /** Find contradicting entry pairs and generate debate framework */
  function hypothesisTest(args: Args): Promise<unknown>;
  /** Recompile a single wiki entry with version bump */
  function recompileEntry(args: Args): Promise<unknown>;
  /** Create or update a wiki entry (YAML front matter required) */
  function saveWikiEntry(args: Args): Promise<unknown>;
  /** Finish a compile job: rebuild indexes and run quality gate */
  function completeCompileJob(args: Args): Promise<unknown>;
  /** Generate compile_plan.json with L1/L2/L3 tasks */
  function generateCompilePlan(args: Args): Promise<unknown>;
  /** Read the current compile plan and task statuses */
  function getCompilePlan(args: Args): Promise<unknown>;
  /** Mark a compile plan task as done */
  function markPlanTaskDone(args: Args): Promise<unknown>;
  /** Compile an uploaded PDF by file_id from POST /api/upload */
  function compileUploadedPdf(args: Args): Promise<unknown>;

  // --- index ---
  /** Search wiki entries (hybrid Tantivy + TF-IDF RRF) */
  function searchKnowledge(args: Args): Promise<unknown>;
  /** Rebuild Tantivy, petgraph, and TF-IDF indexes from wiki Markdown */
  function rebuildIndex(args: Args): Promise<unknown>;
  /** Get N-hop neighbors of a knowledge entry */
  function getEntryContext(args: Args): Promise<unknown>;
  /** Token-efficient context bundle: center body, neighbors, related snippets */
  function getAgentContext(args: Args): Promise<unknown>;
  /** Preview a structured patch (unified diff, no write) */
  function previewWikiPatch(args: Args): Promise<unknown>;
  /** Apply structured patch and reindex entry */
  function patchWikiEntry(args: Args): Promise<unknown>;
  /** Alias for patch_wiki_entry — apply structured patch and reindex */
  function applyWikiPatch(args: Args): Promise<unknown>;
  /** Find entries with no related/contradiction links */
  function findOrphans(args: Args): Promise<unknown>;
  /** Suggest links based on tag similarity (Jaccard) */
  function suggestLinks(args: Args): Promise<unknown>;
  /** Export local concept map as Mermaid.js text */
  function exportConceptMap(args: Args): Promise<unknown>;
  /** Analyze wiki quality and return report with next actions */
  function checkQuality(args: Args): Promise<unknown>;
  /** Compile-job context for awaiting_agent: stages, artifacts, prompts */
  function getCompilationContext(args: Args): Promise<unknown>;

  // --- management ---
  /** Get runtime configuration for a knowledge base */
  function getConfig(args: Args): Promise<unknown>;
  /** Set a runtime configuration value (atomic write) */
  function setConfig(args: Args): Promise<unknown>;
  /** Comprehensive KB health: entries, graph, index, quality, extraction stack */
  function getHealthReport(args: Args): Promise<unknown>;
  /** Manually trigger incremental compilation */
  function triggerIncrementalCompile(args: Args): Promise<unknown>;
  /** Compile status with stages and quality snapshot */
  function getCompileStatus(args: Args): Promise<unknown>;
  /** List quality issues with stable issue_id */
  function listQualityIssues(args: Args): Promise<unknown>;
  /** Suggest MCP actions to fix a quality issue */
  function fixSuggest(args: Args): Promise<unknown>;
  /** Run publish quality gate on all wiki entries */
  function applyQualityGate(args: Args): Promise<unknown>;
  /** Open interactive wiki browser MCP App resource */
  function showWikiBrowser(args: Args): Promise<unknown>;

  // --- platform ---
  /** List registered knowledge base workspaces */
  function listWorkspaces(args: Args): Promise<unknown>;
  /** Set the active workspace by kb_id */
  function setActiveWorkspace(args: Args): Promise<unknown>;
  /** Register or update a knowledge base workspace */
  function registerWorkspace(args: Args): Promise<unknown>;
  /** List extraction backends in the current router */
  function listExtractionPlugins(args: Args): Promise<unknown>;
  /** Probe which extraction backend would be selected for a PDF */
  function probeExtraction(args: Args): Promise<unknown>;
  /** Compare local KB manifest with remote sync store */
  function syncStatus(args: Args): Promise<unknown>;
  /** Push local KB objects to remote sync store */
  function syncPush(args: Args): Promise<unknown>;
  /** Pull KB objects from remote sync store */
  function syncPull(args: Args): Promise<unknown>;
  /** Submit wiki patch proposal without applying */
  function submitPatchProposal(args: Args): Promise<unknown>;
  /** Apply a pending patch proposal by id */
  function applyPatchProposal(args: Args): Promise<unknown>;
  /** List pending or filtered patch proposals for a knowledge base */
  function listPatchProposals(args: Args): Promise<unknown>;

}

export = Compendium;
