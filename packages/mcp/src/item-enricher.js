/**
 * Item Enricher
 *
 * Analyzes raw content and generates metadata for richer judgments.
 * Transforms bare items into structured objects with properties
 * that the 25-dimension scorers can evaluate.
 *
 * "Enrich before judging" - κυνικός
 *
 * @module @cynic/mcp/item-enricher
 */

'use strict';

import { createHash } from 'crypto';

// ═══════════════════════════════════════════════════════════════════════════
// CONTENT ANALYSIS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Extract text content from item
 * @param {Object|string} item
 * @returns {string}
 */
function extractText(item) {
  if (typeof item === 'string') return item;
  if (typeof item.content === 'string') return item.content;
  if (typeof item.body === 'string') return item.body;
  if (typeof item.text === 'string') return item.text;
  if (typeof item.data === 'string') return item.data;
  if (typeof item.description === 'string') return item.description;
  return '';
}

/**
 * Detect content type from text
 * @param {string} text
 * @returns {string} 'code' | 'decision' | 'pattern' | 'knowledge' | 'text'
 */
function detectType(text) {
  if (!text) return 'unknown';

  // Code patterns
  const codeIndicators = [
    /^(import|export|const|let|var|function|class|def|async|await)\s/m,
    /\{\s*\n.*\n\s*\}/s,
    /=>\s*[{(]/,
    /\(\s*\)\s*=>/,
    /;\s*$/m,
  ];
  const codeMatches = codeIndicators.filter(p => p.test(text)).length;
  if (codeMatches >= 2) return 'code';

  // Decision patterns
  const decisionIndicators = [
    /\b(decide|decision|chose|choose|option|alternative)\b/i,
    /\b(pros?|cons?|trade-?off|versus|vs\.?)\b/i,
    /\b(should|must|will|recommend)\b/i,
  ];
  const decisionMatches = decisionIndicators.filter(p => p.test(text)).length;
  if (decisionMatches >= 2) return 'decision';

  // Pattern/observation
  const patternIndicators = [
    /\b(pattern|trend|observation|notice|detect)\b/i,
    /\b(always|never|usually|often|sometimes)\b/i,
    /\b(correlation|relationship|connection)\b/i,
  ];
  const patternMatches = patternIndicators.filter(p => p.test(text)).length;
  if (patternMatches >= 2) return 'pattern';

  // Knowledge/fact
  const knowledgeIndicators = [
    /\b(is|are|was|were|has|have|does|do)\b/i,
    /\b(definition|means|refers|describes)\b/i,
  ];
  const knowledgeMatches = knowledgeIndicators.filter(p => p.test(text)).length;
  if (knowledgeMatches >= 2) return 'knowledge';

  return 'text';
}

/**
 * Analyze code quality indicators
 * @param {string} text
 * @returns {Object}
 */
function analyzeCode(text) {
  if (!text) return null;

  const analysis = {
    hasCode: false,
    language: null,
    hasComments: false,
    hasErrorHandling: false,
    hasTypes: false,
    hasTests: false,
    hasDocs: false,
    functionCount: 0,
    classCount: 0,
    importCount: 0,
    lineCount: 0,
    complexity: 'low',
  };

  // Check if it's code
  const codePatterns = [
    /function\s+\w+/,
    /class\s+\w+/,
    /const\s+\w+\s*=/,
    /import\s+.*from/,
    /def\s+\w+\(/,
    /=>\s*[{(]/,
  ];
  analysis.hasCode = codePatterns.some(p => p.test(text));
  if (!analysis.hasCode) return null;

  // Detect language
  if (/import\s+.*from\s+['"]|export\s+(default\s+)?/.test(text)) {
    analysis.language = 'javascript';
  } else if (/def\s+\w+\(|class\s+\w+:|import\s+\w+$/.test(text)) {
    analysis.language = 'python';
  } else if (/fn\s+\w+|impl\s+\w+|use\s+\w+::/.test(text)) {
    analysis.language = 'rust';
  } else if (/func\s+\w+|package\s+\w+/.test(text)) {
    analysis.language = 'go';
  }

  // Analyze quality
  analysis.hasComments = /\/\*[\s\S]*?\*\/|\/\/.*$|#.*$/m.test(text);
  analysis.hasErrorHandling = /try\s*\{|catch\s*\(|\.catch\(|except:|raise\s|throw\s/.test(text);
  analysis.hasTypes = /:\s*(string|number|boolean|object|int|float|str|bool|Array|Promise)/.test(text);
  analysis.hasTests = /test\(|describe\(|it\(|expect\(|assert|@Test/.test(text);
  analysis.hasDocs = /\/\*\*[\s\S]*?\*\/|"""[\s\S]*?"""|'''[\s\S]*?'''/.test(text);

  // Count elements
  analysis.functionCount = (text.match(/function\s+\w+|=>\s*[{(]|def\s+\w+\(/g) || []).length;
  analysis.classCount = (text.match(/class\s+\w+/g) || []).length;
  analysis.importCount = (text.match(/import\s+/gm) || []).length;
  analysis.lineCount = text.split('\n').length;

  // Estimate complexity
  const nestedDepth = (text.match(/\{[^{}]*\{[^{}]*\{/g) || []).length;
  const cyclomaticIndicators = (text.match(/if\s*\(|else\s*\{|for\s*\(|while\s*\(|switch\s*\(|\?\s*:/g) || []).length;

  if (nestedDepth > 3 || cyclomaticIndicators > 10) {
    analysis.complexity = 'high';
  } else if (nestedDepth > 1 || cyclomaticIndicators > 5) {
    analysis.complexity = 'medium';
  }

  return analysis;
}

/**
 * Analyze text quality indicators
 * @param {string} text
 * @returns {Object}
 */
function analyzeText(text) {
  if (!text) return {};

  const words = text.split(/\s+/).filter(w => w.length > 0);
  const sentences = text.split(/[.!?]+/).filter(s => s.trim().length > 0);
  const paragraphs = text.split(/\n\n+/).filter(p => p.trim().length > 0);

  const wordCount = words.length;
  const sentenceCount = sentences.length;
  const avgWordLength = wordCount > 0
    ? words.reduce((sum, w) => sum + w.length, 0) / wordCount
    : 0;
  const avgSentenceLength = sentenceCount > 0 ? wordCount / sentenceCount : 0;

  // Vocabulary richness (unique words / total words)
  const uniqueWords = new Set(words.map(w => w.toLowerCase()));
  const vocabularyRichness = wordCount > 0 ? uniqueWords.size / wordCount : 0;

  // Detect structure
  const hasHeaders = /^#+\s|^\d+\.\s|^-\s/m.test(text);
  const hasBullets = /^[-*]\s/m.test(text);
  const hasNumberedList = /^\d+\.\s/m.test(text);

  // Detect tone
  const hasQuestions = /\?/.test(text);
  const hasExclamations = /!/.test(text);
  const hasCitations = /\[\d+\]|"[^"]+"|'[^']+'/.test(text);

  return {
    wordCount,
    sentenceCount,
    paragraphCount: paragraphs.length,
    avgWordLength: Math.round(avgWordLength * 10) / 10,
    avgSentenceLength: Math.round(avgSentenceLength * 10) / 10,
    vocabularyRichness: Math.round(vocabularyRichness * 100) / 100,
    hasHeaders,
    hasBullets,
    hasNumberedList,
    hasQuestions,
    hasExclamations,
    hasCitations,
    charCount: text.length,
  };
}

/**
 * Detect sources and references in text
 * @param {string} text
 * @returns {string[]}
 */
function detectSources(text) {
  if (!text) return [];

  const sources = [];

  // URLs
  const urls = text.match(/https?:\/\/[^\s)>\]]+/g) || [];
  sources.push(...urls);

  // GitHub references
  const githubRefs = text.match(/github\.com\/[\w-]+\/[\w-]+/g) || [];
  sources.push(...githubRefs.map(r => `https://${r}`));

  // Package references
  const npmRefs = text.match(/@[\w-]+\/[\w-]+|[\w-]+@\d+\.\d+/g) || [];
  sources.push(...npmRefs.map(r => `npm:${r}`));

  // Academic citations [1], [Author, 2024]
  const citations = text.match(/\[\d+\]|\[[\w\s,]+\d{4}\]/g) || [];
  sources.push(...citations.map(c => `citation:${c}`));

  return [...new Set(sources)]; // Dedupe
}

/**
 * Generate content hash
 * @param {string} text
 * @returns {string}
 */
function generateHash(text) {
  if (!text) return null;
  return createHash('sha256').update(text).digest('hex').slice(0, 16);
}

// ═══════════════════════════════════════════════════════════════════════════
// MAIN ENRICHER
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Enrich an item with metadata for richer judgment
 *
 * @param {Object|string} item - Raw item to enrich
 * @param {Object} [context] - Additional context
 * @returns {Object} Enriched item with metadata
 */
export function enrichItem(item, context = {}) {
  // Normalize item to object
  const normalizedItem = typeof item === 'string'
    ? { content: item }
    : { ...item };

  const text = extractText(normalizedItem);
  const detectedType = detectType(text);
  const codeAnalysis = analyzeCode(text);
  const textAnalysis = analyzeText(text);
  const sources = detectSources(text);

  // Build enriched item
  const enriched = {
    // Original data
    ...normalizedItem,

    // Identity
    id: normalizedItem.id || `item_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`,
    type: normalizedItem.type || context.type || detectedType,

    // Timestamps
    createdAt: normalizedItem.createdAt || context.createdAt || Date.now(),
    enrichedAt: Date.now(),

    // Content analysis
    hash: normalizedItem.hash || generateHash(text),
    textAnalysis,

    // Sources (merge detected + provided)
    sources: [
      ...sources,
      ...(normalizedItem.sources || []),
      ...(context.sources || []),
    ].filter(Boolean),

    // Verification status
    verified: normalizedItem.verified ?? context.verified ?? false,

    // Author/provenance
    author: normalizedItem.author || context.author || context.operator || null,
    origin: normalizedItem.origin || context.origin || context.source || null,

    // Quality indicators (for scorers)
    quality: normalizedItem.quality || null,
  };

  // Add code-specific metadata
  if (codeAnalysis) {
    enriched.codeAnalysis = codeAnalysis;
    enriched.hasCode = true;
    enriched.language = codeAnalysis.language;

    // Map to scorer-friendly properties
    enriched.tested = codeAnalysis.hasTests;
    enriched.documentation = codeAnalysis.hasDocs;
    enriched.hasErrorHandling = codeAnalysis.hasErrorHandling;
  }

  // Calculate derived scores based on analysis
  enriched.derivedScores = calculateDerivedScores(enriched, textAnalysis, codeAnalysis);

  return enriched;
}

/**
 * Calculate derived scores from analysis
 * These help scorers by providing pre-computed metrics
 *
 * @param {Object} item - Enriched item
 * @param {Object} textAnalysis - Text analysis results
 * @param {Object|null} codeAnalysis - Code analysis results
 * @returns {Object} Derived scores
 */
function calculateDerivedScores(item, textAnalysis, codeAnalysis) {
  const scores = {};

  // COHERENCE indicators
  if (textAnalysis.vocabularyRichness) {
    // Good vocabulary richness is 0.3-0.7
    const vr = textAnalysis.vocabularyRichness;
    scores.coherenceHint = vr >= 0.3 && vr <= 0.7 ? 70 : vr < 0.3 ? 40 : 50;
  }

  // STRUCTURE indicators
  scores.structureHint = 50;
  if (textAnalysis.hasHeaders) scores.structureHint += 15;
  if (textAnalysis.hasBullets || textAnalysis.hasNumberedList) scores.structureHint += 10;
  if (textAnalysis.paragraphCount >= 2) scores.structureHint += 10;
  scores.structureHint = Math.min(scores.structureHint, 100);

  // COMPLETENESS indicators
  scores.completenessHint = 50;
  if (item.id) scores.completenessHint += 5;
  if (item.type && item.type !== 'unknown') scores.completenessHint += 10;
  if (textAnalysis.wordCount > 20) scores.completenessHint += 10;
  if (item.sources?.length > 0) scores.completenessHint += 10;
  if (item.hash) scores.completenessHint += 5;
  scores.completenessHint = Math.min(scores.completenessHint, 100);

  // VERIFIABILITY indicators
  scores.verifiabilityHint = 40;
  if (item.hash) scores.verifiabilityHint += 15;
  if (item.sources?.length > 0) scores.verifiabilityHint += item.sources.length * 5;
  if (item.verified) scores.verifiabilityHint += 20;
  scores.verifiabilityHint = Math.min(scores.verifiabilityHint, 100);

  // Code-specific
  if (codeAnalysis) {
    // TRANSPARENCY (comments, docs)
    scores.transparencyHint = 50;
    if (codeAnalysis.hasComments) scores.transparencyHint += 15;
    if (codeAnalysis.hasDocs) scores.transparencyHint += 20;
    scores.transparencyHint = Math.min(scores.transparencyHint, 100);

    // CONTRIBUTION (tests, error handling)
    scores.contributionHint = 50;
    if (codeAnalysis.hasTests) scores.contributionHint += 20;
    if (codeAnalysis.hasErrorHandling) scores.contributionHint += 15;
    if (codeAnalysis.hasTypes) scores.contributionHint += 10;
    scores.contributionHint = Math.min(scores.contributionHint, 100);

    // EFFICIENCY (complexity)
    scores.efficiencyHint = codeAnalysis.complexity === 'low' ? 75
      : codeAnalysis.complexity === 'medium' ? 55
      : 35;
  }

  return scores;
}

/**
 * Batch enrich multiple items
 * @param {Array} items - Items to enrich
 * @param {Object} [context] - Shared context
 * @returns {Array} Enriched items
 */
export function enrichItems(items, context = {}) {
  return items.map(item => enrichItem(item, context));
}

export default {
  enrichItem,
  enrichItems,
  // Expose utilities for testing
  detectType,
  analyzeCode,
  analyzeText,
  detectSources,
  generateHash,
};
