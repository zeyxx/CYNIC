/**
 * CYNIC Dimension Scorers
 *
 * Real scoring logic for all 25 dimensions.
 * Replaces mock scoring with intelligent content analysis.
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/judge/scorers
 */

'use strict';

// VERSION MARKER: v2.0 - Negative scoring enabled (2026-01-20)
// This log MUST appear at startup if this code is loaded
console.log('[SCORERS] *** VERSION 2.0 LOADED - Negative scoring active ***');

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════
// UTILITY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Extract text content from item
 * @param {Object} item - Item to extract from
 * @returns {string} Extracted text
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
 * Count words in text
 * @param {string} text
 * @returns {number}
 */
function wordCount(text) {
  if (!text) return 0;
  return text.split(/\s+/).filter(w => w.length > 0).length;
}

/**
 * Count sentences in text
 * @param {string} text
 * @returns {number}
 */
function sentenceCount(text) {
  if (!text) return 0;
  return text.split(/[.!?]+/).filter(s => s.trim().length > 0).length;
}

/**
 * Calculate average word length
 * @param {string} text
 * @returns {number}
 */
function avgWordLength(text) {
  if (!text) return 0;
  const words = text.split(/\s+/).filter(w => w.length > 0);
  if (words.length === 0) return 0;
  return words.reduce((sum, w) => sum + w.length, 0) / words.length;
}

/**
 * Check if text has code patterns
 * @param {string} text
 * @returns {boolean}
 */
function hasCodePatterns(text) {
  if (!text) return false;
  const codeIndicators = [
    /function\s+\w+/,
    /class\s+\w+/,
    /const\s+\w+\s*=/,
    /let\s+\w+\s*=/,
    /import\s+.*from/,
    /export\s+(default\s+)?/,
    /def\s+\w+\(/,
    /=>\s*[{(]/,
    /\{\s*\n/,
  ];
  return codeIndicators.some(p => p.test(text));
}

/**
 * Check for contradictions (simple heuristic)
 * @param {string} text
 * @returns {number} Number of potential contradictions
 */
function detectContradictions(text) {
  if (!text) return 0;
  const contradictionPatterns = [
    /but\s+also\s+not/i,
    /however.*but.*however/i,
    /is\s+not.*is\s+/i,
    /always.*never/i,
    /never.*always/i,
  ];
  return contradictionPatterns.filter(p => p.test(text)).length;
}

/**
 * Calculate text complexity score
 * @param {string} text
 * @returns {number} 0-100
 */
function calculateComplexity(text) {
  if (!text) return 50;

  const words = wordCount(text);
  const sentences = sentenceCount(text);
  const avgLen = avgWordLength(text);

  // Flesch-Kincaid approximation
  const wordsPerSentence = sentences > 0 ? words / sentences : words;

  // Technical words (longer words suggest more complexity)
  const techScore = Math.min(avgLen / 8, 1) * 30;

  // Sentence complexity
  const sentenceScore = Math.min(wordsPerSentence / 25, 1) * 30;

  // Code detection adds complexity
  const codeScore = hasCodePatterns(text) ? 20 : 0;

  return Math.min(techScore + sentenceScore + codeScore + 20, 100);
}

/**
 * Normalize score to 0-100
 */
function normalize(score) {
  return Math.min(Math.max(Math.round(score * 10) / 10, 0), 100);
}

// ═══════════════════════════════════════════════════════════════════════════
// PHI AXIOM SCORERS - Structure & Coherence
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Score COHERENCE - Internal logical consistency
 */
export function scoreCoherence(item, context = {}) {
  let score = 50;
  const text = extractText(item);

  // Structured data is more coherent
  if (typeof item === 'object' && item !== null) {
    score += 10;
  }

  // Has required fields
  if (item.id) score += 5;
  if (item.type) score += 5;

  // Has content
  if (text.length > 0) score += 10;

  // Check for contradictions
  const contradictions = detectContradictions(text);
  score -= contradictions * 10;

  // Consistent terminology (no wild variation)
  const words = text.toLowerCase().split(/\s+/);
  const uniqueRatio = new Set(words).size / Math.max(words.length, 1);
  // Good ratio is 0.3-0.7 (some repetition = consistency)
  if (uniqueRatio >= 0.3 && uniqueRatio <= 0.7) {
    score += 10;
  }

  return normalize(score);
}

/**
 * Score HARMONY - Balance and proportion (φ-alignment)
 */
export function scoreHarmony(item, context = {}) {
  let score = 50;
  const text = extractText(item);

  if (text.length === 0) return score;

  const words = wordCount(text);
  const sentences = sentenceCount(text);

  // φ-aligned proportions
  // Ideal words per sentence: ~13-21 (Fibonacci range)
  if (sentences > 0) {
    const wps = words / sentences;
    if (wps >= 13 && wps <= 21) {
      score += 20; // Perfect φ range
    } else if (wps >= 8 && wps <= 34) {
      score += 10; // Acceptable Fibonacci range
    }
  }

  // Balanced structure (intro/body/conclusion-like)
  if (sentences >= 3) {
    score += 10;
  }

  // Not too short, not too long
  if (words >= 21 && words <= 987) { // Fibonacci bounds
    score += 10;
  }

  return normalize(score);
}

/**
 * Score STRUCTURE - Organizational clarity
 */
export function scoreStructure(item, context = {}) {
  let score = 50;
  const text = extractText(item);

  // Has clear organization
  if (typeof item === 'object') {
    const keys = Object.keys(item);
    if (keys.length >= 3 && keys.length <= 13) { // Fibonacci bounds
      score += 15;
    }
  }

  // Has sections/paragraphs
  const paragraphs = text.split(/\n\n+/).filter(p => p.trim().length > 0);
  if (paragraphs.length >= 2) {
    score += 10;
  }

  // Has headers or markers
  if (/^#+\s|^\d+\.\s|^-\s/m.test(text)) {
    score += 10;
  }

  // Code has structure markers
  if (hasCodePatterns(text)) {
    if (/\{\s*\n.*\n\s*\}/s.test(text)) score += 10;
    if (/\/\*[\s\S]*?\*\/|\/\/.*$/m.test(text)) score += 5; // Comments
  }

  return normalize(score);
}

/**
 * Score ELEGANCE - Simplicity and beauty
 */
export function scoreElegance(item, context = {}) {
  let score = 50;
  const text = extractText(item);

  if (text.length === 0) return score;

  const words = wordCount(text);
  const avgLen = avgWordLength(text);

  // Concise is elegant - penalize verbosity
  if (words > 0 && words < 144) { // Fib(12)
    score += 15;
  } else if (words > 987) { // Too verbose
    score -= 10;
  }

  // Clear language (not overly complex)
  if (avgLen < 6) {
    score += 10; // Simple words
  } else if (avgLen > 8) {
    score -= 5; // Overly complex
  }

  // Minimal filler words
  const fillers = (text.match(/\b(very|really|just|actually|basically|literally)\b/gi) || []).length;
  score -= fillers * 3;

  // Code elegance: short functions, clear names
  if (hasCodePatterns(text)) {
    const lines = text.split('\n').length;
    if (lines < 50) score += 10;
  }

  return normalize(score);
}

/**
 * Score COMPLETENESS - Wholeness of solution
 */
export function scoreCompleteness(item, context = {}) {
  let score = 50;
  const text = extractText(item);

  // Has multiple required elements
  if (item.id) score += 5;
  if (item.type) score += 5;
  if (text.length > 0) score += 10;
  if (item.metadata || item.meta) score += 5;
  if (item.timestamp || item.createdAt) score += 5;

  // Has introduction and conclusion signals
  if (/^(first|to begin|introduction|overview)/im.test(text)) score += 5;
  if (/(in conclusion|finally|summary|to summarize)/im.test(text)) score += 5;

  // Code completeness: has imports, exports, error handling
  if (hasCodePatterns(text)) {
    if (/import\s+/m.test(text)) score += 5;
    if (/export\s+/m.test(text)) score += 5;
    if (/try\s*\{|catch\s*\(/m.test(text)) score += 5;
    if (/return\s+/m.test(text)) score += 5;
  }

  return normalize(score);
}

/**
 * Score PRECISION - Accuracy and exactness
 */
export function scorePrecision(item, context = {}) {
  let score = 50;
  const text = extractText(item);

  // Has specific identifiers
  if (item.id) score += 10;
  if (item.version) score += 5;

  // Contains numbers/specifics
  const hasNumbers = /\d+/.test(text);
  if (hasNumbers) score += 10;

  // Has precise timestamps
  if (item.timestamp || item.createdAt) {
    if (typeof item.timestamp === 'number' || typeof item.createdAt === 'number') {
      score += 10;
    }
  }

  // Avoids vague language
  const vagueWords = (text.match(/\b(some|many|few|several|various|etc|stuff|things)\b/gi) || []).length;
  score -= vagueWords * 3;

  // Code precision: typed, specific variable names
  if (hasCodePatterns(text)) {
    if (/:\s*(string|number|boolean|object|array)/i.test(text)) score += 10;
  }

  return normalize(score);
}

// ═══════════════════════════════════════════════════════════════════════════
// VERIFY AXIOM SCORERS - Verification & Trust
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Score ACCURACY - Factual correctness
 */
export function scoreAccuracy(item, context = {}) {
  let score = 50;

  // Has sources
  if (item.sources && Array.isArray(item.sources)) {
    score += Math.min(item.sources.length * 5, 20);
  }

  // Has verification
  if (item.verified === true) {
    score += 20;
  }

  // Has references
  if (item.references && Array.isArray(item.references)) {
    score += Math.min(item.references.length * 3, 15);
  }

  // Hash/signature for data integrity
  if (item.hash) score += 5;
  if (item.signature) score += 10;

  return normalize(score);
}

/**
 * Score VERIFIABILITY - Can be independently verified
 */
export function scoreVerifiability(item, context = {}) {
  let score = 40;

  // Has proof
  if (item.proof) score += 30;

  // Has signature
  if (item.signature) score += 15;

  // Has hash
  if (item.hash) score += 15;

  // Has public source
  if (item.url || item.sourceUrl) score += 10;

  // Has checksum
  if (item.checksum) score += 10;

  // Has testable claims
  const text = extractText(item);
  if (/can be verified|reproducible|testable/i.test(text)) score += 5;

  return normalize(score);
}

/**
 * Score TRANSPARENCY - Clear reasoning visible
 */
export function scoreTransparency(item, context = {}) {
  let score = 50;
  const text = extractText(item);

  // Has explicit reasoning
  if (item.reasoning || item.rationale) score += 15;

  // Shows methodology
  if (item.methodology || item.method) score += 15;

  // Text explains why
  if (/because|therefore|reason|explains|due to/i.test(text)) score += 10;

  // Code has comments
  if (hasCodePatterns(text)) {
    const commentMatches = text.match(/\/\*[\s\S]*?\*\/|\/\/.*$/gm) || [];
    score += Math.min(commentMatches.length * 3, 15);
  }

  // Has visible decision process
  if (item.decisions || item.steps) score += 10;

  return normalize(score);
}

/**
 * Score REPRODUCIBILITY - Results can be reproduced
 */
export function scoreReproducibility(item, context = {}) {
  let score = 45;

  // Has version info
  if (item.version) score += 10;

  // Has dependencies listed
  if (item.dependencies) score += 10;

  // Has environment info
  if (item.environment || item.env) score += 10;

  // Has seed/config for randomness
  if (item.seed || item.config) score += 10;

  // Has steps to reproduce
  if (item.steps || item.instructions) score += 15;

  return normalize(score);
}

/**
 * Score PROVENANCE - Source is traceable
 */
export function scoreProvenance(item, context = {}) {
  let score = 40;

  // Has author
  if (item.author || item.creator || item.operator) score += 15;

  // Has timestamp
  if (item.timestamp || item.createdAt) score += 10;

  // Has origin
  if (item.origin || item.source) score += 15;

  // Has chain of custody
  if (item.history || item.audit || item.chain) score += 15;

  // Has signature
  if (item.signature) score += 10;

  return normalize(score);
}

/**
 * Score INTEGRITY - Has not been tampered with
 */
export function scoreIntegrity(item, context = {}) {
  let score = 50;

  // Has hash
  if (item.hash) score += 20;

  // Has signature
  if (item.signature) score += 20;

  // Has checksum
  if (item.checksum) score += 10;

  // Immutable fields present
  if (item.id && item.createdAt) score += 5;

  // Has merkle proof
  if (item.merkleProof || item.proof) score += 10;

  return normalize(score);
}

// ═══════════════════════════════════════════════════════════════════════════
// CULTURE AXIOM SCORERS - Cultural Fit
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Score AUTHENTICITY - Genuine and original
 */
export function scoreAuthenticity(item, context = {}) {
  let score = 50;
  const text = extractText(item);
  const words = item.wordCount || wordCount(text);

  // ═══ POSITIVE INDICATORS ═══
  // Enriched authenticity signals (from item-enricher)
  if (item.original === true) score += 15;
  if (item.authentic === true) score += 15;

  // Has author/creator (enricher adds 'CYNIC' as default)
  if (item.author || item.creator) score += 10;

  // Unique identifier
  if (item.id && typeof item.id === 'string' && item.id.length > 8) score += 5;

  // Not a copy/fork
  if (!item.forkedFrom && !item.copiedFrom) score += 10;

  // Has personal voice (I, we statements)
  if (/\b(I|we|my|our)\s+(?!==)/i.test(text)) score += 5;

  // Has enriched tags (sign of analysis)
  if (item.tags && Array.isArray(item.tags) && item.tags.length > 0) score += 5;

  // Quality tags boost authenticity
  if (item.tags && Array.isArray(item.tags)) {
    const qualityTags = item.tags.filter(t => t.startsWith('quality:'));
    score += Math.min(qualityTags.length * 8, 20);
  }

  // ═══ NEGATIVE INDICATORS ═══
  // Risk tags from enricher - NOT authentic
  if (item.tags && Array.isArray(item.tags)) {
    const riskTags = item.tags.filter(t => t.startsWith('risk:'));
    score -= riskTags.length * 15;
  }

  // Direct scam indicators destroy authenticity
  const scamPatterns = [
    /scam|fraud|rug\s*pull|honeypot|ponzi/i,
    /anonymous\s*(team|dev)/i,
    /fake/i,
  ];
  const scamCount = scamPatterns.filter(p => p.test(text)).length;
  score -= scamCount * 20;

  // Very short content - reduced penalty (items can be identifiers)
  if (words < 3) score -= 10;
  else if (words < 5) score -= 5;

  // Generic template phrases
  const genericPatterns = [
    /lorem ipsum/i,
    /click here/i,
    /buy now/i,
    /limited time/i,
    /act now/i,
    /\[insert\s+\w+\]/i,
  ];
  const genericCount = genericPatterns.filter(p => p.test(text)).length;
  score -= genericCount * 8;

  // Is a copy/fork
  if (item.forkedFrom || item.copiedFrom) score -= 15;

  return normalize(score);
}

/**
 * Score RELEVANCE - Pertinent to context
 */
export function scoreRelevance(item, context = {}) {
  let score = 50;
  const text = extractText(item);
  const words = item.wordCount || wordCount(text);

  // ═══ POSITIVE INDICATORS ═══
  // Has explicit relevance (from enricher)
  if (item.relevance) {
    score += typeof item.relevance === 'number' ? Math.min(item.relevance / 2, 25) : 20;
  }

  // Has tags/categories (from enricher)
  if (item.tags && Array.isArray(item.tags)) {
    score += Math.min(item.tags.length * 3, 20);
  }

  // Context match
  if (context.topic && text.toLowerCase().includes(context.topic.toLowerCase())) {
    score += 15;
  }

  // Domain relevance (crypto/blockchain)
  const domainTerms = ['token', 'crypto', 'blockchain', 'solana', 'ethereum', 'wallet', 'defi'];
  const domainMatches = domainTerms.filter(t => text.toLowerCase().includes(t)).length;
  if (domainMatches > 0) score += Math.min(domainMatches * 5, 15);

  // Recent is more relevant
  if (item.createdAt) {
    const age = Date.now() - item.createdAt;
    const dayAge = age / (1000 * 60 * 60 * 24);
    if (dayAge < 1) score += 10;
    else if (dayAge < 7) score += 5;
  }

  // ═══ NEGATIVE INDICATORS ═══
  // Empty content - reduced penalty
  if (words < 2) score -= 15;
  else if (words < 5) score -= 5;

  // Old content loses relevance (> 90 days)
  if (item.createdAt) {
    const age = Date.now() - item.createdAt;
    const dayAge = age / (1000 * 60 * 60 * 24);
    if (dayAge > 365) score -= 15;
    else if (dayAge > 90) score -= 8;
  }

  // Generic filler content
  if (/^(test|example|sample|placeholder|untitled)/i.test(text.trim())) {
    score -= 15;
  }

  return normalize(score);
}

/**
 * Score NOVELTY - New or unique contribution
 */
export function scoreNovelty(item, context = {}) {
  let score = 50; // Start neutral, not pessimistic
  const text = extractText(item);
  const words = item.wordCount || wordCount(text);

  // ═══ POSITIVE INDICATORS ═══
  // New item (from enricher - items are now tagged with createdAt)
  if (item.createdAt && Date.now() - item.createdAt < 86400000) {
    score += 15;
  }

  // Marked as original/new (from enricher authenticity detection)
  if (item.original === true || item.isNew === true) {
    score += 15;
  }

  // Has unique content
  if (item.unique === true) score += 15;

  // First of its kind
  if (item.first === true || item.pioneer === true) score += 15;

  // Has hash (unique identifier from enricher)
  if (item.hash) score += 5;

  // ═══ NEGATIVE INDICATORS ═══
  // Old content is not novel (> 30 days)
  if (item.createdAt) {
    const age = Date.now() - item.createdAt;
    const dayAge = age / (1000 * 60 * 60 * 24);
    if (dayAge > 180) score -= 15;
    else if (dayAge > 30) score -= 8;
  }

  // Copy/fork = not novel
  if (item.forkedFrom || item.copiedFrom || item.duplicate) {
    score -= 20;
  }

  // Template/boilerplate patterns
  const boilerplatePatterns = [
    /^(hello world|foo bar|test)/i,
    /lorem ipsum/i,
    /\{\{.*\}\}/,  // Mustache templates
    /<%-?.*%>/,    // EJS templates
  ];
  if (boilerplatePatterns.some(p => p.test(text))) {
    score -= 15;
  }

  // Very short - mild penalty only
  if (words < 2) score -= 10;

  return normalize(score);
}

/**
 * Score ALIGNMENT - Fits cultural values
 */
export function scoreAlignment(item, context = {}) {
  let score = 50;
  const text = extractText(item);
  const words = item.wordCount || wordCount(text);

  // ═══ POSITIVE INDICATORS ═══
  // φ-aligned values
  if (/φ|phi|golden\s*ratio/i.test(text)) score += 10;
  if (/verify|trust.*verify/i.test(text)) score += 10;
  if (/burn.*extract|non.*extractive/i.test(text)) score += 10;

  // Follows standards
  if (item.standards || item.compliance) score += 10;

  // Has community endorsement
  if (item.endorsed === true || item.approved === true) score += 10;

  // Ethical considerations
  if (item.ethical || /ethic|fair|equit/i.test(text)) score += 5;

  // Quality tags from enricher boost score
  if (item.tags && Array.isArray(item.tags)) {
    const qualityTags = item.tags.filter(t => t.startsWith('quality:'));
    score += Math.min(qualityTags.length * 8, 20);
  }

  // ═══ NEGATIVE INDICATORS ═══
  // Risk tags from enricher - HEAVY penalties
  if (item.tags && Array.isArray(item.tags)) {
    const riskTags = item.tags.filter(t => t.startsWith('risk:'));
    score -= riskTags.length * 15; // Each risk tag = -15
  }

  // Direct scam/fraud indicators in text
  const scamPatterns = [
    /scam|fraud|rug\s*pull|honeypot|ponzi/i,
    /anonymous\s*(team|dev)/i,
    /fake\s*(liquidity|volume|audit)/i,
    /100%\s*(tax|fee)/i,
    /copy[\s-]*paste\s*code/i,
  ];
  const scamCount = scamPatterns.filter(p => p.test(text)).length;
  score -= scamCount * 20;

  // Anti-pattern indicators (extractive, exploitative)
  const antiPatterns = [
    /get rich quick/i,
    /guaranteed.*return/i,
    /\d+x\s*(return|profit|gain)/i,
    /pump|moon|lambo/i,
    /shill|fomo|fud/i,
    /free money/i,
    /act fast|limited offer/i,
  ];
  const antiCount = antiPatterns.filter(p => p.test(text)).length;
  score -= antiCount * 12;

  // Spam/scam patterns
  if (/\$\$\$|!!!|👉|🚀{3,}/u.test(text)) {
    score -= 15;
  }

  // No substance - mild penalty
  if (words < 3) score -= 10;

  // Rejected/flagged
  if (item.rejected === true || item.flagged === true) {
    score -= 25;
  }

  return normalize(score);
}

/**
 * Score IMPACT - Meaningful effect
 */
export function scoreImpact(item, context = {}) {
  let score = 45;
  const text = extractText(item);
  const words = wordCount(text);

  // ═══ POSITIVE INDICATORS ═══
  // Has explicit impact
  if (item.impact) {
    score += typeof item.impact === 'number' ? item.impact / 2 : 15;
  }

  // Has usage metrics
  if (item.usageCount && item.usageCount > 0) {
    score += Math.min(Math.log10(item.usageCount + 1) * 10, 25);
  }

  // Has citations/references
  if (item.citations && item.citations > 0) {
    score += Math.min(item.citations * 2, 20);
  }

  // Has downstream effects
  if (item.derivatives && item.derivatives > 0) score += 10;

  // ═══ NEGATIVE INDICATORS ═══
  // No metrics at all = no measurable impact
  if (!item.usageCount && !item.citations && !item.derivatives && !item.impact) {
    score -= 15;
  }

  // Zero engagement explicitly
  if (item.views === 0 || item.downloads === 0 || item.usageCount === 0) {
    score -= 10;
  }

  // No clear purpose
  if (!item.purpose && !item.goal && words < 10) {
    score -= 10;
  }

  // Low-effort content markers
  if (words < 5) score -= 20;
  else if (words < 10) score -= 10;

  // Deprecated/archived = diminished impact
  if (item.deprecated === true || item.archived === true) {
    score -= 15;
  }

  return normalize(score);
}

/**
 * Score RESONANCE - Connects emotionally
 */
export function scoreResonance(item, context = {}) {
  let score = 45;
  const text = extractText(item);
  const words = wordCount(text);

  // ═══ POSITIVE INDICATORS ═══
  // Has emotional language
  const emotionalWords = (text.match(/\b(love|hate|fear|joy|hope|trust|believe|feel|passion|inspire)\b/gi) || []).length;
  score += Math.min(emotionalWords * 5, 20);

  // Has engagement metrics
  if (item.likes || item.reactions) {
    const engagement = item.likes || item.reactions;
    score += Math.min(Math.log10(engagement + 1) * 8, 20);
  }

  // Has comments/discussion
  if (item.comments && item.comments > 0) {
    score += Math.min(item.comments * 2, 15);
  }

  // Personal/relatable
  if (/\b(you|your|we|our|us)\b/i.test(text)) score += 5;

  // ═══ NEGATIVE INDICATORS ═══
  // No emotional content at all
  if (emotionalWords === 0 && words > 20) {
    score -= 10;
  }

  // Corporate/robotic language
  const corporatePatterns = [
    /\b(leverage|synergy|stakeholder|deliverable|bandwidth)\b/i,
    /\b(circle back|move the needle|low-hanging fruit)\b/i,
    /\b(pursuant to|in accordance with|hereby)\b/i,
  ];
  const corporateCount = corporatePatterns.filter(p => p.test(text)).length;
  score -= corporateCount * 8;

  // Zero engagement
  if (item.likes === 0 && item.comments === 0 && item.reactions === 0) {
    score -= 10;
  }

  // No substance to resonate with
  if (words < 5) score -= 20;
  else if (words < 10) score -= 10;

  // Generic filler
  if (/^(ok|okay|yes|no|thanks|thank you|good|nice|cool)$/i.test(text.trim())) {
    score -= 25;
  }

  return normalize(score);
}

// ═══════════════════════════════════════════════════════════════════════════
// BURN AXIOM SCORERS - Value & Sustainability
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Score UTILITY - Practical usefulness
 */
export function scoreUtility(item, context = {}) {
  let score = 50;

  // Has clear purpose
  if (item.purpose || item.goal) score += 15;

  // Has usage count
  if (item.usageCount && item.usageCount > 0) {
    score += Math.min(Math.log10(item.usageCount + 1) * 10, 25);
  }

  // Actionable
  if (item.actionable === true) score += 10;

  // Has instructions/how-to
  if (item.instructions || item.howTo) score += 10;

  // Solves a problem
  if (item.problem || item.solution) score += 10;

  return normalize(score);
}

/**
 * Score SUSTAINABILITY - Long-term viability
 */
export function scoreSustainability(item, context = {}) {
  let score = 50;

  // Has long-term support
  if (item.maintained === true || item.supported === true) score += 15;

  // Has versioning
  if (item.version) score += 10;

  // Low maintenance burden
  if (item.maintenanceBurden === 'low') score += 10;

  // Has roadmap/future plans
  if (item.roadmap || item.future) score += 10;

  // Not deprecated
  if (item.deprecated === true) score -= 30;

  // Has community
  if (item.community || item.contributors) score += 10;

  return normalize(score);
}

/**
 * Score EFFICIENCY - Resource optimization
 */
export function scoreEfficiency(item, context = {}) {
  let score = 50;
  const text = extractText(item);

  // Size efficiency
  const textSize = text.length;
  if (textSize > 0 && textSize < 5000) score += 10;
  else if (textSize > 50000) score -= 10;

  // Code efficiency indicators
  if (hasCodePatterns(text)) {
    // No unnecessary complexity
    const nestedDepth = (text.match(/\{[^{}]*\{[^{}]*\{/g) || []).length;
    if (nestedDepth === 0) score += 10;
    else if (nestedDepth > 5) score -= 10;

    // Reuses code (imports)
    if (/import\s+/m.test(text)) score += 5;
  }

  // Fast/performant markers
  if (item.performance || /fast|efficient|optimized/i.test(text)) score += 10;

  // Low resource usage
  if (item.resourceUsage === 'low') score += 10;

  return normalize(score);
}

/**
 * Score VALUE_CREATION - Creates more than consumes
 */
export function scoreValueCreation(item, context = {}) {
  let score = 50;

  // Creates output
  if (item.output || item.produces) score += 15;

  // Has derivatives/children
  if (item.derivatives && item.derivatives > 0) {
    score += Math.min(item.derivatives * 5, 20);
  }

  // Enables others
  if (item.enables || item.empowers) score += 10;

  // Net positive
  if (item.netValue && item.netValue > 0) score += 15;

  // Creates vs consumes ratio
  if (item.createdValue && item.consumedValue) {
    const ratio = item.createdValue / Math.max(item.consumedValue, 1);
    score += Math.min(ratio * 10, 20);
  }

  return normalize(score);
}

/**
 * Score NON_EXTRACTIVE - Does not extract value unfairly
 */
export function scoreNonExtractive(item, context = {}) {
  let score = 60; // Assume good faith

  // Explicitly non-extractive
  if (item.nonExtractive === true || item.fair === true) score += 15;

  // Open source/free
  if (item.openSource === true || item.free === true) score += 15;

  // Has fair compensation
  if (item.compensation || item.attribution) score += 10;

  // No hidden costs
  if (item.hiddenCosts === true) score -= 30;

  // Community benefit
  if (item.communityBenefit === true) score += 10;

  // Penalize extraction markers
  if (item.extractive === true) score -= 40;

  return normalize(score);
}

/**
 * Score CONTRIBUTION - Gives back to ecosystem
 */
export function scoreContribution(item, context = {}) {
  let score = 45;

  // Has contributions
  if (item.contributions && item.contributions > 0) {
    score += Math.min(item.contributions * 3, 20);
  }

  // Open source
  if (item.openSource === true) score += 15;

  // Has documentation
  if (item.documentation || item.docs) score += 10;

  // Has examples
  if (item.examples) score += 10;

  // Has tests
  if (item.tests || item.tested === true) score += 10;

  // Community involvement
  if (item.communityInvolved === true) score += 10;

  return normalize(score);
}

// ═══════════════════════════════════════════════════════════════════════════
// SCORER REGISTRY
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Map of dimension names to scorer functions
 */
export const Scorers = {
  // PHI Axiom
  COHERENCE: scoreCoherence,
  HARMONY: scoreHarmony,
  STRUCTURE: scoreStructure,
  ELEGANCE: scoreElegance,
  COMPLETENESS: scoreCompleteness,
  PRECISION: scorePrecision,

  // VERIFY Axiom
  ACCURACY: scoreAccuracy,
  VERIFIABILITY: scoreVerifiability,
  TRANSPARENCY: scoreTransparency,
  REPRODUCIBILITY: scoreReproducibility,
  PROVENANCE: scoreProvenance,
  INTEGRITY: scoreIntegrity,

  // CULTURE Axiom
  AUTHENTICITY: scoreAuthenticity,
  RELEVANCE: scoreRelevance,
  NOVELTY: scoreNovelty,
  ALIGNMENT: scoreAlignment,
  IMPACT: scoreImpact,
  RESONANCE: scoreResonance,

  // BURN Axiom
  UTILITY: scoreUtility,
  SUSTAINABILITY: scoreSustainability,
  EFFICIENCY: scoreEfficiency,
  VALUE_CREATION: scoreValueCreation,
  NON_EXTRACTIVE: scoreNonExtractive,
  CONTRIBUTION: scoreContribution,
};

/**
 * Score a dimension by name
 * @param {string} name - Dimension name
 * @param {Object} item - Item to score
 * @param {Object} [context] - Context
 * @returns {number} Score (0-100)
 */
export function scoreDimension(name, item, context = {}) {
  // Check if item has explicit scores first (for testing/overrides)
  if (item && item.scores && typeof item.scores[name] === 'number') {
    return item.scores[name];
  }

  // Check for derived scores from enrichment (boost or starting point)
  const hintKey = `${name.toLowerCase()}Hint`;
  const derivedHint = item?.derivedScores?.[hintKey];

  const scorer = Scorers[name];
  if (scorer) {
    const baseScore = scorer(item, context);

    // If we have a derived hint, blend it with the scored value
    // This helps when enrichment detected something the scorer might miss
    if (typeof derivedHint === 'number') {
      // Weighted average: 70% scorer, 30% hint
      return normalize(baseScore * 0.7 + derivedHint * 0.3);
    }

    return baseScore;
  }

  // Fallback: use derived hint if available
  if (typeof derivedHint === 'number') {
    return derivedHint;
  }

  // Fallback for unknown dimensions
  return 50;
}

/**
 * Create a composite scorer function for the judge
 * @returns {Function} Scorer function compatible with CYNICJudge
 */
export function createRealScorer() {
  return (name, item, context) => {
    return scoreDimension(name, item, context);
  };
}

export default {
  Scorers,
  scoreDimension,
  createRealScorer,
  // Individual scorers (for testing/customization)
  scoreCoherence,
  scoreHarmony,
  scoreStructure,
  scoreElegance,
  scoreCompleteness,
  scorePrecision,
  scoreAccuracy,
  scoreVerifiability,
  scoreTransparency,
  scoreReproducibility,
  scoreProvenance,
  scoreIntegrity,
  scoreAuthenticity,
  scoreRelevance,
  scoreNovelty,
  scoreAlignment,
  scoreImpact,
  scoreResonance,
  scoreUtility,
  scoreSustainability,
  scoreEfficiency,
  scoreValueCreation,
  scoreNonExtractive,
  scoreContribution,
};
