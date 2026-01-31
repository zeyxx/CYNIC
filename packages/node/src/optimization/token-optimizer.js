/**
 * Token Optimizer
 *
 * Intelligent token optimization for LLM requests, achieving significant
 * token reduction through compression, caching, and deduplication.
 *
 * Strategies:
 * - Prompt compression (whitespace, common patterns)
 * - Response caching (semantic content hash)
 * - Context deduplication (remove repeated content)
 * - Template pre-compression (static content)
 *
 * "Every token saved is a dog treat earned" - κυνικός
 *
 * @module @cynic/node/optimization/token-optimizer
 */

'use strict';

import { EventEmitter } from 'events';
import { createHash } from 'crypto';
import { PHI_INV } from '@cynic/core';

/**
 * φ-aligned configuration
 */
const OPTIMIZER_CONFIG = Object.freeze({
  // Cache settings
  CACHE_MAX_SIZE: 233,           // F(13) - max cached responses
  CACHE_TTL_MS: 300000,          // 5 minutes (Claude cache TTL)
  CACHE_EXTENDED_TTL_MS: 3600000, // 1 hour (extended TTL for 4.5 models)

  // Compression thresholds (φ-aligned)
  MIN_COMPRESSIBLE_TOKENS: 21,   // F(8) - minimum tokens to compress
  COMPRESSION_TARGET: PHI_INV,   // 61.8% target compression ratio

  // Deduplication
  MIN_DUPLICATE_LENGTH: 55,      // F(10) - minimum chars for dedup
  SIMILARITY_THRESHOLD: 0.85,    // 85% similarity for dedup

  // Stats
  STATS_WINDOW_SIZE: 144,        // F(12) - rolling window for stats
});

/**
 * Compression strategies
 */
const CompressionStrategy = Object.freeze({
  WHITESPACE: 'whitespace',
  ABBREVIATION: 'abbreviation',
  TEMPLATE: 'template',
  SEMANTIC: 'semantic',
});

/**
 * Common abbreviations for compression
 */
const ABBREVIATIONS = Object.freeze({
  'function': 'fn',
  'return': 'ret',
  'const': 'c',
  'export': 'exp',
  'import': 'imp',
  'default': 'def',
  'async': 'as',
  'await': 'aw',
  'undefined': 'undef',
  'parameters': 'params',
  'arguments': 'args',
  'configuration': 'cfg',
  'implementation': 'impl',
  'documentation': 'docs',
  'description': 'desc',
  'information': 'info',
  'application': 'app',
  'environment': 'env',
  'development': 'dev',
  'production': 'prod',
});

/**
 * Reverse abbreviations for decompression
 */
const ABBREVIATIONS_REVERSE = Object.freeze(
  Object.fromEntries(Object.entries(ABBREVIATIONS).map(([k, v]) => [v, k]))
);

/**
 * Common filler phrases to remove
 */
const FILLER_PHRASES = [
  /\bplease\b/gi,
  /\bkindly\b/gi,
  /\bI would like to\b/gi,
  /\bI want to\b/gi,
  /\bCan you\b/gi,
  /\bCould you\b/gi,
  /\bWould you\b/gi,
  /\bI need you to\b/gi,
  /\bI'm looking for\b/gi,
  /\bin order to\b/gi,
  /\bbasically\b/gi,
  /\bactually\b/gi,
  /\bso to speak\b/gi,
];

/**
 * Token Optimizer
 *
 * Optimizes tokens through multiple strategies.
 */
export class TokenOptimizer extends EventEmitter {
  constructor(options = {}) {
    super();

    this.config = { ...OPTIMIZER_CONFIG, ...options.config };

    // Response cache (content hash -> response)
    this._cache = new Map();
    this._cacheOrder = []; // LRU tracking

    // Template registry (name -> compressed template)
    this._templates = new Map();

    // Statistics
    this.stats = {
      requests: 0,
      tokensIn: 0,
      tokensOut: 0,
      cacheHits: 0,
      cacheMisses: 0,
      compressionRatio: 0,
      deduplications: 0,
    };

    // Rolling window for ratio calculation
    this._ratioWindow = [];
  }

  /**
   * Optimize a prompt/request
   *
   * @param {Object} params - Optimization parameters
   * @param {string} params.content - Content to optimize
   * @param {Object} [params.context] - Additional context
   * @param {string[]} [params.strategies] - Strategies to apply
   * @returns {Object} Optimization result
   */
  optimize(params) {
    const { content, context = {}, strategies = ['whitespace', 'abbreviation'] } = params;

    if (!content || typeof content !== 'string') {
      return this._result(content, content, 0, []);
    }

    const originalTokens = this._estimateTokens(content);
    let optimized = content;
    const appliedStrategies = [];

    // Check cache first
    const cacheKey = this._computeHash(content);
    const cached = this._getCached(cacheKey);
    if (cached) {
      this.stats.cacheHits++;
      this.emit('cache:hit', { key: cacheKey });
      return this._result(content, cached.optimized, cached.savedTokens, ['cache']);
    }
    this.stats.cacheMisses++;

    // Apply compression strategies
    if (strategies.includes('whitespace') || strategies.includes(CompressionStrategy.WHITESPACE)) {
      optimized = this._compressWhitespace(optimized);
      appliedStrategies.push('whitespace');
    }

    if (strategies.includes('abbreviation') || strategies.includes(CompressionStrategy.ABBREVIATION)) {
      optimized = this._applyAbbreviations(optimized);
      appliedStrategies.push('abbreviation');
    }

    if (strategies.includes('filler') || strategies.includes('semantic')) {
      optimized = this._removeFiller(optimized);
      appliedStrategies.push('filler');
    }

    if (strategies.includes('dedup')) {
      const dedupResult = this._deduplicateContext(optimized);
      optimized = dedupResult.content;
      if (dedupResult.removed > 0) {
        appliedStrategies.push('dedup');
        this.stats.deduplications += dedupResult.removed;
      }
    }

    // Calculate savings
    const optimizedTokens = this._estimateTokens(optimized);
    const savedTokens = originalTokens - optimizedTokens;

    // Cache the result
    if (savedTokens > 0) {
      this._setCache(cacheKey, { optimized, savedTokens });
    }

    return this._result(content, optimized, savedTokens, appliedStrategies);
  }

  /**
   * Register a template for pre-compression
   *
   * @param {string} name - Template name
   * @param {string} template - Template content
   * @returns {Object} Registration result
   */
  registerTemplate(name, template) {
    const compressed = this._compressWhitespace(template);
    const originalTokens = this._estimateTokens(template);
    const compressedTokens = this._estimateTokens(compressed);

    this._templates.set(name, {
      original: template,
      compressed,
      savedTokens: originalTokens - compressedTokens,
    });

    this.emit('template:registered', { name, savedTokens: originalTokens - compressedTokens });

    return {
      name,
      originalTokens,
      compressedTokens,
      savedTokens: originalTokens - compressedTokens,
    };
  }

  /**
   * Get a registered template (compressed version)
   *
   * @param {string} name - Template name
   * @returns {string|null} Compressed template or null
   */
  getTemplate(name) {
    const entry = this._templates.get(name);
    return entry ? entry.compressed : null;
  }

  /**
   * Check if a request might be cached
   *
   * @param {string} content - Content to check
   * @returns {boolean} True if likely cached
   */
  isCached(content) {
    const cacheKey = this._computeHash(content);
    return this._cache.has(cacheKey);
  }

  /**
   * Decompress abbreviated content
   *
   * @param {string} content - Abbreviated content
   * @returns {string} Decompressed content
   */
  decompress(content) {
    let result = content;

    for (const [abbrev, full] of Object.entries(ABBREVIATIONS_REVERSE)) {
      // Only replace standalone abbreviations (word boundaries)
      const pattern = new RegExp(`\\b${abbrev}\\b`, 'g');
      result = result.replace(pattern, full);
    }

    return result;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // COMPRESSION STRATEGIES
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Compress whitespace
   * @private
   */
  _compressWhitespace(content) {
    return content
      // Normalize line endings
      .replace(/\r\n/g, '\n')
      // Remove trailing whitespace
      .replace(/[ \t]+$/gm, '')
      // Collapse multiple blank lines to single
      .replace(/\n{3,}/g, '\n\n')
      // Collapse multiple spaces to single
      .replace(/[ \t]{2,}/g, ' ')
      // Remove leading whitespace on lines (preserve indentation in code blocks)
      .replace(/^[ \t]+(?![`])/gm, (match, offset, str) => {
        // Keep indentation within code blocks
        const beforeMatch = str.slice(0, offset);
        const inCodeBlock = (beforeMatch.match(/```/g) || []).length % 2 !== 0;
        return inCodeBlock ? match : '';
      })
      .trim();
  }

  /**
   * Apply common abbreviations
   * @private
   */
  _applyAbbreviations(content) {
    let result = content;

    for (const [full, abbrev] of Object.entries(ABBREVIATIONS)) {
      // Only abbreviate outside of quotes and code blocks
      const pattern = new RegExp(`\\b${full}\\b`, 'gi');
      result = result.replace(pattern, (match) => {
        // Preserve case of first letter
        return match[0] === match[0].toUpperCase()
          ? abbrev[0].toUpperCase() + abbrev.slice(1)
          : abbrev;
      });
    }

    return result;
  }

  /**
   * Remove filler phrases
   * @private
   */
  _removeFiller(content) {
    let result = content;

    for (const pattern of FILLER_PHRASES) {
      result = result.replace(pattern, '');
    }

    // Clean up extra spaces left by removal
    return result.replace(/\s{2,}/g, ' ').trim();
  }

  /**
   * Deduplicate repeated context
   * @private
   */
  _deduplicateContext(content) {
    const lines = content.split('\n');
    const seen = new Set();
    const deduplicated = [];
    let removed = 0;

    for (const line of lines) {
      const trimmed = line.trim();

      // Skip empty lines
      if (!trimmed) {
        deduplicated.push(line);
        continue;
      }

      // Only dedup lines above minimum length
      if (trimmed.length < this.config.MIN_DUPLICATE_LENGTH) {
        deduplicated.push(line);
        continue;
      }

      // Compute normalized form for comparison
      const normalized = this._normalizeForDedup(trimmed);

      if (seen.has(normalized)) {
        removed++;
        // Add a reference marker instead
        deduplicated.push(`[see above: ${trimmed.slice(0, 30)}...]`);
      } else {
        seen.add(normalized);
        deduplicated.push(line);
      }
    }

    return {
      content: deduplicated.join('\n'),
      removed,
    };
  }

  /**
   * Normalize content for deduplication comparison
   * @private
   */
  _normalizeForDedup(content) {
    return content
      .toLowerCase()
      .replace(/\s+/g, ' ')
      .replace(/[^\w\s]/g, '')
      .trim();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // CACHING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Compute content hash
   * @private
   */
  _computeHash(content) {
    return createHash('sha256').update(content).digest('hex').slice(0, 16);
  }

  /**
   * Get cached result
   * @private
   */
  _getCached(key) {
    const entry = this._cache.get(key);
    if (!entry) return null;

    // Check TTL
    if (Date.now() > entry.expiresAt) {
      this._cache.delete(key);
      this._cacheOrder = this._cacheOrder.filter(k => k !== key);
      return null;
    }

    // Update LRU order
    this._cacheOrder = this._cacheOrder.filter(k => k !== key);
    this._cacheOrder.push(key);

    return entry;
  }

  /**
   * Set cache entry
   * @private
   */
  _setCache(key, value) {
    // Evict if at capacity
    while (this._cache.size >= this.config.CACHE_MAX_SIZE) {
      const oldest = this._cacheOrder.shift();
      if (oldest) {
        this._cache.delete(oldest);
      }
    }

    this._cache.set(key, {
      ...value,
      createdAt: Date.now(),
      expiresAt: Date.now() + this.config.CACHE_TTL_MS,
    });
    this._cacheOrder.push(key);
  }

  /**
   * Clear cache
   */
  clearCache() {
    this._cache.clear();
    this._cacheOrder = [];
    this.emit('cache:cleared');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // TOKEN ESTIMATION
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Estimate token count (rough approximation)
   * Claude uses ~4 characters per token on average
   * @private
   */
  _estimateTokens(content) {
    if (!content) return 0;

    // Base estimate: ~4 chars per token
    const charEstimate = Math.ceil(content.length / 4);

    // Adjust for whitespace (less tokens)
    const whitespaceRatio = (content.match(/\s/g) || []).length / content.length;
    const whitespaceAdjust = 1 - (whitespaceRatio * 0.3);

    // Adjust for code (more tokens due to special chars)
    const codeChars = (content.match(/[{}[\]();:,.<>\/\\|@#$%^&*+=~`]/g) || []).length;
    const codeRatio = codeChars / content.length;
    const codeAdjust = 1 + (codeRatio * 0.2);

    return Math.ceil(charEstimate * whitespaceAdjust * codeAdjust);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // RESULT & STATS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create result object and update stats
   * @private
   */
  _result(original, optimized, savedTokens, strategies) {
    const originalTokens = this._estimateTokens(original);
    const optimizedTokens = this._estimateTokens(optimized);

    // Update stats
    this.stats.requests++;
    this.stats.tokensIn += originalTokens;
    this.stats.tokensOut += optimizedTokens;

    // Update rolling ratio window
    if (originalTokens > 0) {
      this._ratioWindow.push(optimizedTokens / originalTokens);
      if (this._ratioWindow.length > this.config.STATS_WINDOW_SIZE) {
        this._ratioWindow.shift();
      }
      this.stats.compressionRatio = 1 - (
        this._ratioWindow.reduce((a, b) => a + b, 0) / this._ratioWindow.length
      );
    }

    this.emit('optimize', {
      originalTokens,
      optimizedTokens,
      savedTokens,
      strategies,
    });

    return {
      original,
      optimized,
      originalTokens,
      optimizedTokens,
      savedTokens,
      strategies,
      compressionRatio: originalTokens > 0
        ? (originalTokens - optimizedTokens) / originalTokens
        : 0,
    };
  }

  /**
   * Get optimizer statistics
   * @returns {Object} Stats
   */
  getStats() {
    const totalSaved = this.stats.tokensIn - this.stats.tokensOut;
    const cacheHitRate = this.stats.requests > 0
      ? this.stats.cacheHits / this.stats.requests
      : 0;

    return {
      ...this.stats,
      totalSaved,
      cacheHitRate: Math.round(cacheHitRate * 1000) / 10, // Percentage
      cacheSize: this._cache.size,
      templateCount: this._templates.size,
    };
  }

  /**
   * Reset statistics
   */
  resetStats() {
    this.stats = {
      requests: 0,
      tokensIn: 0,
      tokensOut: 0,
      cacheHits: 0,
      cacheMisses: 0,
      compressionRatio: 0,
      deduplications: 0,
    };
    this._ratioWindow = [];
  }
}

/**
 * Create TokenOptimizer instance
 *
 * @param {Object} [options] - Options
 * @returns {TokenOptimizer}
 */
export function createTokenOptimizer(options = {}) {
  return new TokenOptimizer(options);
}

export {
  OPTIMIZER_CONFIG,
  CompressionStrategy,
  ABBREVIATIONS,
};

export default {
  TokenOptimizer,
  createTokenOptimizer,
  OPTIMIZER_CONFIG,
  CompressionStrategy,
  ABBREVIATIONS,
};
