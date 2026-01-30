/**
 * CONTEXT INTELLIGENCE - C-Score & Budget Management
 *
 * "Context is everything, but everything cannot be context" - κυνικός
 *
 * C-Score = (Pertinence × Fraîcheur × Densité × Entropy) / √Taille
 *
 * φ-aligned context limits:
 * - Target: 23.6% (φ⁻³) - Optimal working context
 * - Soft: 38.2% (φ⁻²) - Start compacting
 * - Hard: 61.8% (φ⁻¹) - Force compaction
 *
 * "Lost in the middle" problem: LLMs perform worse with info
 * in middle of long contexts. We sort by C-Score to keep
 * best content at START and END.
 *
 * Entropy-guided scoring (2025 research):
 * - Reduces attention collapse by 80%
 * - Tokens with healthy entropy (φ⁻¹ range) retained
 * - High entropy (noise) prioritized for compaction
 *
 * @module @cynic/core/context
 * @philosophy Context quality over quantity
 */

'use strict';

import {
  PHI_INV,
  PHI_INV_2,
  PHI_INV_3,
  DECIMAL_PRECISION,
} from '../axioms/constants.js';

import {
  calculateEntropyFactor,
  ENTROPY_THRESHOLDS,
  ENTROPY_WEIGHTS,
  calculateShannonEntropy,
  calculateWordEntropy,
  calculateLexicalEntropy,
  calculateStructuralEntropy,
  entropyToRetentionFactor,
} from './entropy.js';

// =============================================================================
// CONSTANTS
// =============================================================================

/**
 * Default context limits in tokens (Claude Code ~200k)
 */
export const DEFAULT_CONTEXT_SIZE = 200000;

/**
 * φ-aligned budget thresholds
 * Based on research showing 60% utilization is optimal
 */
export const BUDGET_THRESHOLDS = {
  TARGET: PHI_INV_3,  // 23.6% - Optimal working context
  SOFT: PHI_INV_2,    // 38.2% - Start compacting
  HARD: PHI_INV,      // 61.8% - Force compaction
};

/**
 * Freshness decay parameters
 * Content loses relevance exponentially over time
 */
export const FRESHNESS_DECAY = {
  HALF_LIFE_TURNS: 5,     // Context loses 50% freshness every 5 turns
  MIN_FRESHNESS: 0.1,      // Never go below 10%
  IMMEDIATE_BOOST: 1.5,    // Recent (last 2 turns) gets 50% boost
};

/**
 * Density thresholds
 * How much useful information per token
 */
export const DENSITY_THRESHOLDS = {
  HIGH: 0.8,     // Code, structured data
  MEDIUM: 0.5,   // Technical prose
  LOW: 0.2,      // Filler, verbose text
};

// =============================================================================
// TOKEN COUNTER
// =============================================================================

/**
 * Estimate token count for text
 *
 * Uses simple heuristic: ~4 chars per token for English/code
 * This is intentionally conservative (overestimates slightly)
 *
 * @param {string} text - Text to count
 * @returns {number} Estimated token count
 */
export function countTokens(text) {
  if (!text || typeof text !== 'string') return 0;

  // Rough heuristic: 4 chars per token (conservative)
  // This accounts for:
  // - English words: ~4-5 chars/token
  // - Code: ~3-4 chars/token (more symbols)
  // - JSON/structured: ~3 chars/token
  const CHARS_PER_TOKEN = 4;

  return Math.ceil(text.length / CHARS_PER_TOKEN);
}

/**
 * Estimate tokens for various content types
 *
 * @param {*} content - Content to estimate
 * @returns {number} Estimated token count
 */
export function estimateTokens(content) {
  if (content === null || content === undefined) return 0;

  if (typeof content === 'string') {
    return countTokens(content);
  }

  if (typeof content === 'object') {
    // JSON serialization gives a reasonable estimate
    try {
      return countTokens(JSON.stringify(content, null, 2));
    } catch {
      return 0;
    }
  }

  // Primitives
  return countTokens(String(content));
}

// =============================================================================
// FRESHNESS CALCULATOR
// =============================================================================

/**
 * Calculate freshness score based on age
 *
 * Uses exponential decay with turn-based half-life
 *
 * @param {number} turnsSinceAdded - Number of conversation turns since content was added
 * @param {Object} [options] - Options
 * @param {number} [options.halfLife=5] - Turns until 50% decay
 * @param {number} [options.minFreshness=0.1] - Minimum freshness floor
 * @returns {number} Freshness score 0-1
 */
export function calculateFreshness(turnsSinceAdded, options = {}) {
  const {
    halfLife = FRESHNESS_DECAY.HALF_LIFE_TURNS,
    minFreshness = FRESHNESS_DECAY.MIN_FRESHNESS,
  } = options;

  if (turnsSinceAdded < 0) return 1;

  // Exponential decay: f(t) = e^(-λt) where λ = ln(2)/halfLife
  const lambda = Math.log(2) / halfLife;
  const rawFreshness = Math.exp(-lambda * turnsSinceAdded);

  // Apply floor and boost for very recent content
  let freshness = Math.max(rawFreshness, minFreshness);

  // Boost for content from last 2 turns
  if (turnsSinceAdded <= 2) {
    freshness = Math.min(1, freshness * FRESHNESS_DECAY.IMMEDIATE_BOOST);
  }

  return Math.round(freshness * 1000) / 1000;
}

// =============================================================================
// PERTINENCE CALCULATOR
// =============================================================================

/**
 * Calculate pertinence score based on relevance to current task
 *
 * Factors:
 * - Keyword overlap with current query
 * - Type match (code for code tasks, docs for doc tasks)
 * - Explicit references (file paths, function names mentioned)
 *
 * @param {Object} content - Content item with metadata
 * @param {Object} context - Current context (query, task type, etc.)
 * @returns {number} Pertinence score 0-1
 */
export function calculatePertinence(content, context = {}) {
  const { query = '', taskType = 'general', references = [] } = context;

  let pertinence = 0.5; // Default neutral

  // Extract content text for comparison
  const contentText = content.text || content.content || JSON.stringify(content);
  const contentLower = contentText.toLowerCase();

  // 1. Keyword overlap (up to 0.3)
  if (query) {
    const queryWords = query.toLowerCase().split(/\W+/).filter(w => w.length > 2);
    const matches = queryWords.filter(word => contentLower.includes(word));
    const overlap = queryWords.length > 0 ? matches.length / queryWords.length : 0;
    pertinence += overlap * 0.3;
  }

  // 2. Type match (up to 0.2)
  const contentType = content.type || 'unknown';
  const typeMatch = {
    code: ['code', 'implementation', 'function', 'class'],
    docs: ['documentation', 'readme', 'guide', 'explanation'],
    error: ['error', 'stack', 'exception', 'bug'],
    config: ['config', 'settings', 'environment', 'setup'],
  };

  const matchingTypes = typeMatch[taskType] || [];
  if (matchingTypes.some(t => contentType.includes(t) || contentLower.includes(t))) {
    pertinence += 0.2;
  }

  // 3. Explicit references (up to 0.3)
  if (references.length > 0) {
    const refMatches = references.filter(ref =>
      contentLower.includes(ref.toLowerCase())
    );
    if (refMatches.length > 0) {
      pertinence += 0.3 * Math.min(1, refMatches.length / references.length);
    }
  }

  // 4. Source priority (up to 0.1)
  if (content.source === 'user' || content.source === 'explicit') {
    pertinence += 0.1;
  }

  return Math.min(1, Math.max(0, Math.round(pertinence * 1000) / 1000));
}

// =============================================================================
// DENSITY CALCULATOR
// =============================================================================

/**
 * Calculate information density of content
 *
 * High density = lots of useful information per token
 * Low density = verbose, repetitive, or filler content
 *
 * @param {Object} content - Content with text/metadata
 * @returns {number} Density score 0-1
 */
export function calculateDensity(content) {
  const text = content.text || content.content || '';
  if (!text || text.length === 0) return 0;

  let density = 0.5; // Default neutral

  // 1. Code detection (high density)
  const codeIndicators = [
    /function\s+\w+/g,
    /const\s+\w+\s*=/g,
    /class\s+\w+/g,
    /import\s+.*from/g,
    /export\s+(default|const|function)/g,
    /\{\s*\w+:\s*\w+/g,  // Object properties
  ];

  const codeMatches = codeIndicators.reduce((sum, re) => {
    const matches = text.match(re);
    return sum + (matches ? matches.length : 0);
  }, 0);

  if (codeMatches > 5) density += 0.3;
  else if (codeMatches > 2) density += 0.15;

  // 2. Structured data detection (high density)
  const structuredPatterns = [
    /\{[\s\S]*?\}/g,   // JSON-like
    /\[[\s\S]*?\]/g,   // Arrays
    /^\s*[-*]\s+/gm,   // Lists
    /^\s*\d+\.\s+/gm,  // Numbered lists
  ];

  const structuredMatches = structuredPatterns.reduce((sum, re) => {
    const matches = text.match(re);
    return sum + (matches ? matches.length : 0);
  }, 0);

  if (structuredMatches > 10) density += 0.2;
  else if (structuredMatches > 3) density += 0.1;

  // 3. Verbosity penalty (low density)
  const words = text.split(/\s+/).length;
  const avgWordLength = text.replace(/\s+/g, '').length / words;

  // Very short avg word length = might be filler
  if (avgWordLength < 3.5) density -= 0.1;

  // 4. Repetition penalty
  const lines = text.split('\n');
  const uniqueLines = new Set(lines.map(l => l.trim().toLowerCase()));
  const repetitionRatio = uniqueLines.size / lines.length;

  if (repetitionRatio < 0.5) density -= 0.2;
  else if (repetitionRatio < 0.8) density -= 0.1;

  return Math.min(1, Math.max(0, Math.round(density * 1000) / 1000));
}

// =============================================================================
// C-SCORE CALCULATION
// =============================================================================

/**
 * Calculate C-Score (Context Score) for content item
 *
 * C = (Pertinence × Fraîcheur × Densité × Entropy) / √Taille
 *
 * The √Taille (square root of size) creates a sublinear penalty:
 * - Small content with high PFDE is strongly favored
 * - Large content needs proportionally higher PFDE to compete
 *
 * Entropy factor (E) guides retention:
 * - Low entropy (focused, like code) → E ≈ 1.0 (strong retention)
 * - Healthy entropy (φ⁻¹ range) → E ≈ 0.9 (normal retention)
 * - High entropy (diffuse, noise) → E ≈ 0.5 (compaction candidate)
 *
 * @param {Object} content - Content item to score
 * @param {Object} context - Current context for pertinence
 * @param {Object} [options] - Scoring options
 * @returns {Object} C-Score result
 */
export function calculateCScore(content, context = {}, options = {}) {
  const { turnsSinceAdded = 0 } = options;

  // Calculate component scores
  const pertinence = calculatePertinence(content, context);
  const freshness = calculateFreshness(turnsSinceAdded);
  const density = calculateDensity(content);

  // Calculate entropy factor
  const entropyResult = calculateEntropyFactor(content);
  const entropy = entropyResult.factor;

  // Calculate size in tokens
  const text = content.text || content.content || JSON.stringify(content);
  const tokens = countTokens(text);

  // Avoid division by zero and ensure minimum size penalty
  const sizeNormalized = Math.max(1, tokens);

  // C = (P × F × D × E) / √T
  // Square root creates sublinear penalty (larger content isn't infinitely penalized)
  const rawScore = (pertinence * freshness * density * entropy) / Math.sqrt(sizeNormalized / 100);

  // Normalize to 0-100 scale
  const C = Math.min(100, Math.round(rawScore * 100 * DECIMAL_PRECISION) / DECIMAL_PRECISION);

  return {
    C,
    breakdown: {
      pertinence: Math.round(pertinence * 100),
      freshness: Math.round(freshness * 100),
      density: Math.round(density * 100),
      entropy: Math.round(entropy * 100),
      entropyRaw: entropyResult.entropy,
      entropyBreakdown: entropyResult.breakdown,
      tokens,
    },
    formula: 'C = (P × F × D × E) / √(T/100)',
    meta: {
      turnsSinceAdded,
      timestamp: new Date().toISOString(),
      isCodeLike: entropyResult.isCodeLike,
    },
  };
}

// =============================================================================
// BUDGET MANAGER
// =============================================================================

/**
 * Context Budget Manager
 *
 * Manages context window utilization with φ-aligned thresholds
 */
export class BudgetManager {
  /**
   * Create budget manager
   *
   * @param {Object} [options] - Options
   * @param {number} [options.maxTokens] - Maximum context window size
   * @param {number} [options.targetRatio] - Target utilization ratio (φ⁻³)
   * @param {number} [options.softRatio] - Soft limit ratio (φ⁻²)
   * @param {number} [options.hardRatio] - Hard limit ratio (φ⁻¹)
   */
  constructor(options = {}) {
    this.maxTokens = options.maxTokens || DEFAULT_CONTEXT_SIZE;
    this.targetRatio = options.targetRatio || BUDGET_THRESHOLDS.TARGET;
    this.softRatio = options.softRatio || BUDGET_THRESHOLDS.SOFT;
    this.hardRatio = options.hardRatio || BUDGET_THRESHOLDS.HARD;

    // Derived limits
    this.targetLimit = Math.floor(this.maxTokens * this.targetRatio);
    this.softLimit = Math.floor(this.maxTokens * this.softRatio);
    this.hardLimit = Math.floor(this.maxTokens * this.hardRatio);

    // Current state
    this.currentTokens = 0;
    this.items = [];
  }

  /**
   * Get current utilization as percentage
   * @returns {number} Utilization 0-100
   */
  getUtilization() {
    return Math.round((this.currentTokens / this.maxTokens) * 100 * 10) / 10;
  }

  /**
   * Get budget status
   * @returns {Object} Status with level, remaining, etc.
   */
  getStatus() {
    const utilization = this.getUtilization();
    const remaining = this.hardLimit - this.currentTokens;

    let level, action;

    if (this.currentTokens <= this.targetLimit) {
      level = 'OPTIMAL';
      action = 'Continue normally';
    } else if (this.currentTokens <= this.softLimit) {
      level = 'SOFT';
      action = 'Consider compaction';
    } else if (this.currentTokens <= this.hardLimit) {
      level = 'WARNING';
      action = 'Start compaction';
    } else {
      level = 'CRITICAL';
      action = 'Force compaction immediately';
    }

    return {
      level,
      action,
      utilization,
      currentTokens: this.currentTokens,
      remaining: Math.max(0, remaining),
      limits: {
        target: this.targetLimit,
        soft: this.softLimit,
        hard: this.hardLimit,
        max: this.maxTokens,
      },
      thresholds: {
        target: `${Math.round(this.targetRatio * 100)}%`,
        soft: `${Math.round(this.softRatio * 100)}%`,
        hard: `${Math.round(this.hardRatio * 100)}%`,
      },
    };
  }

  /**
   * Check if content can be added within budget
   *
   * @param {number} tokens - Tokens to add
   * @param {string} [priority='normal'] - Priority level
   * @returns {Object} { allowed, reason, wouldExceed }
   */
  canAdd(tokens, priority = 'normal') {
    const newTotal = this.currentTokens + tokens;

    // High priority can exceed soft limit but not hard
    if (priority === 'high') {
      return {
        allowed: newTotal <= this.hardLimit,
        reason: newTotal <= this.hardLimit
          ? 'High priority: within hard limit'
          : `Would exceed hard limit (${this.hardLimit})`,
        wouldExceed: newTotal > this.softLimit ? 'soft' : null,
      };
    }

    // Normal priority should stay within soft limit
    if (priority === 'normal') {
      return {
        allowed: newTotal <= this.softLimit,
        reason: newTotal <= this.softLimit
          ? 'Within soft limit'
          : `Would exceed soft limit (${this.softLimit})`,
        wouldExceed: newTotal > this.targetLimit ? 'target' : null,
      };
    }

    // Low priority should stay within target
    return {
      allowed: newTotal <= this.targetLimit,
      reason: newTotal <= this.targetLimit
        ? 'Within target limit'
        : `Would exceed target limit (${this.targetLimit})`,
      wouldExceed: null,
    };
  }

  /**
   * Add content to budget tracking
   *
   * @param {string} id - Content identifier
   * @param {number} tokens - Token count
   * @param {Object} [metadata] - Additional metadata
   * @returns {boolean} Whether addition was tracked
   */
  track(id, tokens, metadata = {}) {
    this.items.push({ id, tokens, ...metadata, addedAt: Date.now() });
    this.currentTokens += tokens;
    return true;
  }

  /**
   * Remove content from budget tracking
   *
   * @param {string} id - Content identifier
   * @returns {boolean} Whether removal was successful
   */
  untrack(id) {
    const idx = this.items.findIndex(item => item.id === id);
    if (idx === -1) return false;

    this.currentTokens -= this.items[idx].tokens;
    this.items.splice(idx, 1);
    return true;
  }

  /**
   * Get items that should be compacted
   *
   * Returns items to remove to get under target
   * Sorted by priority:
   * 1. Lowest C-Score first (primary)
   * 2. Highest entropy first (secondary, when C-Scores are similar)
   *
   * Entropy-guided compaction prioritizes high-entropy (diffuse)
   * content for removal, preserving focused, information-dense content.
   *
   * @returns {Array} Items to compact
   */
  getCompactionCandidates() {
    if (this.currentTokens <= this.softLimit) {
      return []; // No compaction needed
    }

    const tokensToFree = this.currentTokens - this.targetLimit;

    // Sort by C-Score (lowest first), with entropy as tiebreaker
    // When C-Scores are within 5 points, use entropy to decide
    const sorted = [...this.items].sort((a, b) => {
      const cScoreA = a.cScore || 0;
      const cScoreB = b.cScore || 0;
      const cScoreDiff = cScoreA - cScoreB;

      // If C-Scores are similar (within 5 points), use entropy
      if (Math.abs(cScoreDiff) < 5) {
        // Higher entropy (entropyRaw) = more noise = remove first
        const entropyA = a.entropyRaw || 0.5;
        const entropyB = b.entropyRaw || 0.5;
        // Sort descending by entropy (higher entropy first)
        return entropyB - entropyA;
      }

      // Otherwise sort by C-Score ascending (lower = remove first)
      return cScoreDiff;
    });

    const candidates = [];
    let freedTokens = 0;

    for (const item of sorted) {
      if (freedTokens >= tokensToFree) break;
      candidates.push(item);
      freedTokens += item.tokens;
    }

    return candidates;
  }

  /**
   * Reset budget state
   */
  reset() {
    this.currentTokens = 0;
    this.items = [];
  }
}

// =============================================================================
// CONTEXT ASSEMBLER
// =============================================================================

/**
 * Assemble context from items, prioritized by C-Score
 *
 * Uses "ends matter" strategy:
 * - Highest C-Score items at START
 * - Second-highest at END
 * - Lower scores in MIDDLE (where LLM attention is weakest)
 *
 * @param {Array} items - Content items with text and metadata
 * @param {Object} context - Current context for scoring
 * @param {Object} [options] - Assembly options
 * @returns {Object} Assembled context with metadata
 */
export function assembleContext(items, context = {}, options = {}) {
  const {
    maxTokens = DEFAULT_CONTEXT_SIZE * BUDGET_THRESHOLDS.HARD,
    strategy = 'ends-matter',
    turnsSinceStart = 0,
  } = options;

  if (!items || items.length === 0) {
    return { content: '', items: [], tokens: 0, dropped: [] };
  }

  // Score all items
  const scored = items.map((item, idx) => {
    const cScore = calculateCScore(
      item,
      context,
      { turnsSinceAdded: turnsSinceStart - (item.turn || 0) }
    );
    return { ...item, cScore: cScore.C, cScoreBreakdown: cScore.breakdown, originalIndex: idx };
  });

  // Sort by C-Score descending
  scored.sort((a, b) => b.cScore - a.cScore);

  // Select items within budget
  const selected = [];
  const dropped = [];
  let totalTokens = 0;

  for (const item of scored) {
    const itemTokens = item.cScoreBreakdown?.tokens || countTokens(item.text || item.content || '');

    if (totalTokens + itemTokens <= maxTokens) {
      selected.push(item);
      totalTokens += itemTokens;
    } else {
      dropped.push(item);
    }
  }

  // Apply "ends matter" arrangement if using that strategy
  let arranged;
  if (strategy === 'ends-matter' && selected.length > 2) {
    // High scores at ends, lower in middle
    // [highest, 3rd, 5th, ..., 6th, 4th, 2nd]
    const highEnd = [];
    const lowEnd = [];

    for (let i = 0; i < selected.length; i++) {
      if (i % 2 === 0) {
        highEnd.push(selected[i]);
      } else {
        lowEnd.unshift(selected[i]); // Prepend to reverse order
      }
    }

    arranged = [...highEnd, ...lowEnd];
  } else {
    // Simple descending order
    arranged = selected;
  }

  // Assemble content
  const content = arranged
    .map(item => item.text || item.content || '')
    .join('\n\n---\n\n');

  return {
    content,
    items: arranged,
    tokens: totalTokens,
    dropped,
    stats: {
      selected: selected.length,
      dropped: dropped.length,
      avgCScore: selected.length > 0
        ? Math.round(selected.reduce((sum, i) => sum + i.cScore, 0) / selected.length * 10) / 10
        : 0,
      strategy,
    },
  };
}

// =============================================================================
// RE-EXPORTS FROM ENTROPY MODULE
// =============================================================================

export {
  calculateEntropyFactor,
  ENTROPY_THRESHOLDS,
  ENTROPY_WEIGHTS,
  calculateShannonEntropy,
  calculateWordEntropy,
  calculateLexicalEntropy,
  calculateStructuralEntropy,
  entropyToRetentionFactor,
};

// =============================================================================
// DEFAULT EXPORT
// =============================================================================

export default {
  // Constants
  DEFAULT_CONTEXT_SIZE,
  BUDGET_THRESHOLDS,
  FRESHNESS_DECAY,
  DENSITY_THRESHOLDS,
  ENTROPY_THRESHOLDS,
  ENTROPY_WEIGHTS,

  // Token counting
  countTokens,
  estimateTokens,

  // Component scores
  calculateFreshness,
  calculatePertinence,
  calculateDensity,

  // Entropy
  calculateEntropyFactor,
  calculateShannonEntropy,
  calculateWordEntropy,
  calculateLexicalEntropy,
  calculateStructuralEntropy,
  entropyToRetentionFactor,

  // C-Score
  calculateCScore,

  // Budget management
  BudgetManager,

  // Assembly
  assembleContext,
};
