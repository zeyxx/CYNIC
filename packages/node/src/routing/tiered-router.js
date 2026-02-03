/**
 * Tiered Router
 *
 * Routes requests to optimal handler based on complexity.
 * - Tier 1 (Local): Simple operations, no LLM needed
 * - Tier 2 (Light): Haiku-level, fast responses
 * - Tier 3 (Full): Sonnet/Opus for complex reasoning
 *
 * Inspired by Claude Flow's ADR-026 3-tier routing with 75% cost savings.
 *
 * "The smallest dog that can do the job" - κυνικός
 *
 * @module @cynic/node/routing/tiered-router
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV } from '@cynic/core';
import {
  ComplexityClassifier,
  ComplexityTier,
  COMPLEXITY_THRESHOLDS,
} from './complexity-classifier.js';

/**
 * Handler costs (relative units)
 */
const HANDLER_COSTS = Object.freeze({
  [ComplexityTier.LOCAL]: 0,      // $0 - no API call
  [ComplexityTier.LIGHT]: 1,      // Baseline (Haiku)
  [ComplexityTier.FULL]: 15,      // ~15x Haiku cost (Sonnet/Opus)
});

/**
 * Handler latencies (approximate ms)
 */
const HANDLER_LATENCIES = Object.freeze({
  [ComplexityTier.LOCAL]: 1,      // <1ms local
  [ComplexityTier.LIGHT]: 500,    // ~500ms Haiku
  [ComplexityTier.FULL]: 3000,    // ~3s Sonnet/Opus
});

/**
 * Tiered Router
 *
 * Intelligently routes requests to minimize cost while maintaining quality.
 */
export class TieredRouter extends EventEmitter {
  /**
   * @param {Object} [options] - Router options
   * @param {Object} [options.handlers] - Handler functions by tier
   * @param {ComplexityClassifier} [options.classifier] - Custom classifier
   * @param {boolean} [options.trackSavings=true] - Track cost savings
   */
  constructor(options = {}) {
    super();

    this.classifier = options.classifier || new ComplexityClassifier();
    this.handlers = {
      [ComplexityTier.LOCAL]: options.handlers?.local || this._defaultLocalHandler.bind(this),
      [ComplexityTier.LIGHT]: options.handlers?.light || null,
      [ComplexityTier.FULL]: options.handlers?.full || null,
    };

    this.trackSavings = options.trackSavings !== false;

    // Statistics
    this.stats = {
      routed: 0,
      byTier: { local: 0, light: 0, full: 0 },
      costSaved: 0,
      avgLatency: 0,
      fallbacks: 0,
    };

    // Local handler registry
    this._localHandlers = new Map();
    this._initLocalHandlers();
  }

  /**
   * Route a request to the optimal handler
   *
   * @param {Object} request - Request to route
   * @param {string} request.content - Request content
   * @param {string} [request.type] - Request type
   * @param {Object} [request.context] - Additional context
   * @returns {Promise<Object>} Handler result
   */
  async route(request) {
    const startTime = performance.now();

    // Classify complexity
    const classification = this.classifier.classify(request);
    const { tier, complexity, reason } = classification;

    this.emit('route:classified', { request, classification });

    // Get handler for tier
    let handler = this.handlers[tier];
    let actualTier = tier;

    // Fallback if handler not available
    if (!handler) {
      actualTier = this._fallbackTier(tier);
      handler = this.handlers[actualTier];
      this.stats.fallbacks++;

      this.emit('route:fallback', { from: tier, to: actualTier });
    }

    // Execute handler
    let result;
    try {
      result = await handler(request, classification);
    } catch (err) {
      this.emit('route:error', { tier: actualTier, error: err });

      // Try to escalate on error
      if (actualTier !== ComplexityTier.FULL && this.handlers[ComplexityTier.FULL]) {
        actualTier = ComplexityTier.FULL;
        result = await this.handlers[ComplexityTier.FULL](request, classification);
        this.emit('route:escalated', { reason: 'error', tier: actualTier });
      } else {
        throw err;
      }
    }

    // Track statistics
    const elapsed = performance.now() - startTime;
    this._updateStats(actualTier, elapsed, tier);

    this.emit('route:completed', {
      tier: actualTier,
      complexity,
      reason,
      latency: elapsed,
    });

    return {
      result,
      routing: {
        tier: actualTier,
        originalTier: tier,
        complexity,
        reason,
        latency: elapsed,
        cost: HANDLER_COSTS[actualTier],
      },
    };
  }

  /**
   * Register a local handler for specific patterns
   *
   * @param {string} name - Handler name
   * @param {RegExp} pattern - Pattern to match
   * @param {Function} handler - Handler function
   */
  registerLocalHandler(name, pattern, handler) {
    this._localHandlers.set(name, { pattern, handler });
  }

  /**
   * Find fallback tier when handler unavailable
   * @private
   */
  _fallbackTier(tier) {
    if (tier === ComplexityTier.LOCAL) {
      return this.handlers[ComplexityTier.LIGHT]
        ? ComplexityTier.LIGHT
        : ComplexityTier.FULL;
    }
    if (tier === ComplexityTier.LIGHT) {
      return ComplexityTier.FULL;
    }
    return ComplexityTier.FULL;
  }

  /**
   * Default local handler - handles simple patterns
   * @private
   */
  async _defaultLocalHandler(request, classification) {
    const { content } = request;

    // Try registered local handlers
    for (const [name, { pattern, handler }] of this._localHandlers) {
      if (pattern.test(content)) {
        return await handler(request);
      }
    }

    // No local handler found, signal for fallback
    throw new Error('NO_LOCAL_HANDLER');
  }

  /**
   * Initialize built-in local handlers
   * @private
   */
  _initLocalHandlers() {
    // File existence check
    this.registerLocalHandler(
      'file_exists',
      /^(does|is) .+ exist/i,
      async (req) => {
        const match = req.content.match(/(?:does|is)\s+(.+?)\s+exist/i);
        if (match) {
          return { type: 'check', target: match[1], action: 'exists' };
        }
        throw new Error('PARSE_FAILED');
      }
    );

    // List files
    this.registerLocalHandler(
      'list_files',
      /^(list|show|get) (all )?(files?|dirs?)/i,
      async (req) => {
        return { type: 'list', action: 'files' };
      }
    );

    // Git status
    this.registerLocalHandler(
      'git_status',
      /^git status/i,
      async (req) => {
        return { type: 'git', action: 'status' };
      }
    );

    // Simple format/lint
    this.registerLocalHandler(
      'format',
      /^(format|lint|prettify)/i,
      async (req) => {
        return { type: 'format', action: 'run' };
      }
    );
  }

  /**
   * Update routing statistics
   * @private
   */
  _updateStats(tier, latency, originalTier) {
    this.stats.routed++;
    this.stats.byTier[tier]++;

    // Rolling average latency
    this.stats.avgLatency =
      (this.stats.avgLatency * (this.stats.routed - 1) + latency) /
      this.stats.routed;

    // Calculate cost savings
    if (this.trackSavings && tier !== ComplexityTier.FULL) {
      const savedCost = HANDLER_COSTS[ComplexityTier.FULL] - HANDLER_COSTS[tier];
      this.stats.costSaved += savedCost;
    }
  }

  /**
   * Set handler for a tier
   *
   * @param {string} tier - Tier name
   * @param {Function} handler - Handler function
   */
  setHandler(tier, handler) {
    if (!Object.values(ComplexityTier).includes(tier)) {
      throw new Error(`Invalid tier: ${tier}`);
    }
    this.handlers[tier] = handler;
  }

  /**
   * Get routing statistics
   * @returns {Object} Stats
   */
  getStats() {
    const total = this.stats.routed || 1;
    const localPercent = (this.stats.byTier.local / total) * 100;
    const lightPercent = (this.stats.byTier.light / total) * 100;

    // Estimate cost savings percentage
    const maxPossibleCost = total * HANDLER_COSTS[ComplexityTier.FULL];
    const actualCost = maxPossibleCost - this.stats.costSaved;
    const savingsPercent = maxPossibleCost > 0
      ? ((this.stats.costSaved / maxPossibleCost) * 100)
      : 0;

    return {
      ...this.stats,
      localPercent: Math.round(localPercent * 10) / 10,
      lightPercent: Math.round(lightPercent * 10) / 10,
      fullPercent: Math.round((100 - localPercent - lightPercent) * 10) / 10,
      savingsPercent: Math.round(savingsPercent * 10) / 10,
      classifierStats: this.classifier.getStats(),
    };
  }

  /**
   * Reset statistics
   */
  resetStats() {
    this.stats = {
      routed: 0,
      byTier: { local: 0, light: 0, full: 0 },
      costSaved: 0,
      avgLatency: 0,
      fallbacks: 0,
    };
    this.classifier.resetStats();
  }

  /**
   * Get cost info for a tier
   *
   * @param {string} tier - Tier name
   * @returns {Object} Cost and latency info
   */
  static getTierInfo(tier) {
    return {
      tier,
      cost: HANDLER_COSTS[tier],
      latency: HANDLER_LATENCIES[tier],
    };
  }
}

/**
 * Create tiered router instance
 *
 * @param {Object} [options] - Options
 * @returns {TieredRouter}
 */
export function createTieredRouter(options = {}) {
  return new TieredRouter(options);
}

export {
  ComplexityTier,
  COMPLEXITY_THRESHOLDS,
  HANDLER_COSTS,
  HANDLER_LATENCIES,
};

export default {
  TieredRouter,
  createTieredRouter,
  ComplexityTier,
  COMPLEXITY_THRESHOLDS,
  HANDLER_COSTS,
  HANDLER_LATENCIES,
};
